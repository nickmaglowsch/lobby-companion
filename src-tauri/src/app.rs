//! Estado do app e o laço principal: espera o cliente, ouve o WebSocket e reage
//! à seleção de campeões (buscar build → importar → avisar a janela).

use crate::assets::{Assets, lolalytics_slug};
use crate::champselect::{self, Me};
use crate::import::{itemset, runes, spells};
use crate::lcu::ws::{self, LcuEvent};
use crate::lcu::{self, Lcu};
use crate::lola::model::{self, Build};
use crate::lola::{Lolalytics, Query};
use crate::settings::Settings;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, RwLock, mpsc};

#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    pub ok: bool,
    pub msg: String,
}

impl Outcome {
    fn ok(msg: impl Into<String>) -> Self {
        Self {
            ok: true,
            msg: msg.into(),
        }
    }
    fn fail(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            msg: msg.into(),
        }
    }
    fn from(r: Result<String, String>) -> Self {
        match r {
            Ok(m) => Self::ok(m),
            Err(e) => Self::fail(e),
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ImportReport {
    pub runes: Option<Outcome>,
    pub items: Option<Outcome>,
    pub spells: Option<Outcome>,
}

/// Tudo o que a janela mostra. Vai inteiro no evento "snapshot".
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connected: bool,
    pub phase: String,
    pub in_champ_select: bool,
    pub locked: bool,
    pub champion_id: i64,
    /// Lane do lolalytics (`None` = a mais jogada do campeão).
    pub lane: Option<String>,
    pub loading: bool,
    pub build: Option<Build>,
    pub page_url: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
    pub import: Option<ImportReport>,
}

pub struct Shared {
    pub lcu: RwLock<Option<Lcu>>,
    pub assets: RwLock<Option<Arc<Assets>>>,
    pub snapshot: Mutex<Snapshot>,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    pub lola: Lolalytics,
    icons: Mutex<HashMap<String, (Vec<u8>, String)>>,
    me: Mutex<Option<Me>>,
    /// "campeão:lane" do build em tela; respostas atrasadas de outro campeão são descartadas.
    current_key: Mutex<Option<String>>,
    /// "campeão:lane" já importado nesta seleção (importa de novo só se trocar).
    auto_key: Mutex<Option<String>>,
}

impl Shared {
    pub fn new(settings_path: PathBuf, cache_dir: PathBuf) -> Self {
        Self {
            lcu: RwLock::new(None),
            assets: RwLock::new(None),
            snapshot: Mutex::new(Snapshot {
                phase: "None".into(),
                ..Default::default()
            }),
            settings: Mutex::new(Settings::load(&settings_path)),
            settings_path,
            lola: Lolalytics::new(cache_dir),
            icons: Mutex::new(HashMap::new()),
            me: Mutex::new(None),
            current_key: Mutex::new(None),
            auto_key: Mutex::new(None),
        }
    }
}

async fn update(app: &AppHandle, shared: &Shared, f: impl FnOnce(&mut Snapshot)) {
    let snap = {
        let mut s = shared.snapshot.lock().await;
        f(&mut s);
        s.clone()
    };
    let _ = app.emit("snapshot", snap);
}

/// Laço principal (roda o tempo todo; leve quando o cliente está fechado ou em partida).
pub async fn worker(app: AppHandle) {
    let shared = app.state::<Arc<Shared>>().inner().clone();
    loop {
        let Some(creds) = lcu::discover() else {
            disconnect(&app, &shared).await;
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };
        let Ok(client) = Lcu::new(&creds) else {
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        };
        // O lockfile aparece antes da API responder enquanto o cliente abre.
        let phase = match client.get("/lol-gameflow/v1/gameflow-phase").await {
            Ok(v) => v.as_str().unwrap_or("None").to_string(),
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        };
        match Assets::load(&client).await {
            Ok(a) => *shared.assets.write().await = Some(Arc::new(a)),
            Err(e) => {
                update(&app, &shared, |s| {
                    s.error = Some(format!("dados do cliente: {e}"))
                })
                .await;
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        }
        *shared.lcu.write().await = Some(client.clone());
        update(&app, &shared, |s| {
            s.connected = true;
            s.error = None;
        })
        .await;
        handle_phase(&app, &shared, &phase).await;
        if phase == "ChampSelect"
            && let Ok(sess) = client.get("/lol-champ-select/v1/session").await
        {
            handle_session(&app, &shared, Some(sess)).await;
        }

        let (tx, mut rx) = mpsc::channel(64);
        let ws_task = tokio::spawn(ws::run(creds.clone(), tx));
        while let Some(ev) = rx.recv().await {
            match ev {
                LcuEvent::Phase(p) => handle_phase(&app, &shared, &p).await,
                LcuEvent::ChampSelect(s) => handle_session(&app, &shared, s).await,
            }
        }
        let _ = ws_task.await;
        disconnect(&app, &shared).await;
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn disconnect(app: &AppHandle, shared: &Shared) {
    let was = shared.lcu.write().await.take().is_some();
    *shared.me.lock().await = None;
    if was || shared.snapshot.lock().await.connected {
        update(app, shared, |s| {
            s.connected = false;
            s.phase = "None".into();
            s.in_champ_select = false;
            s.locked = false;
        })
        .await;
    }
}

async fn handle_phase(app: &AppHandle, shared: &Arc<Shared>, phase: &str) {
    let close_in_game = shared.settings.lock().await.close_in_game;
    match phase {
        "ChampSelect" => {
            *shared.auto_key.lock().await = None;
            *shared.me.lock().await = None;
            update(app, shared, |s| {
                s.phase = phase.into();
                s.in_champ_select = true;
                s.locked = false;
                s.import = None;
                s.error = None;
                s.warning = None;
            })
            .await;
            crate::show_main(app);
        }
        "GameStart" | "InProgress" | "Reconnect" => {
            update(app, shared, |s| {
                s.phase = phase.into();
                s.in_champ_select = false;
            })
            .await;
            if close_in_game {
                crate::close_main(app);
            }
        }
        _ => update(app, shared, |s| s.phase = phase.into()).await,
    }
}

async fn handle_session(app: &AppHandle, shared: &Arc<Shared>, session: Option<Value>) {
    let Some(me) = session.as_ref().and_then(champselect::parse) else {
        *shared.me.lock().await = None;
        *shared.auto_key.lock().await = None;
        update(app, shared, |s| {
            s.in_champ_select = false;
            s.locked = false;
        })
        .await;
        return;
    };
    {
        // A sessão muda a cada segundo (timer); só reagimos quando o NOSSO estado muda.
        let mut prev = shared.me.lock().await;
        if prev.as_ref() == Some(&me) {
            return;
        }
        *prev = Some(me.clone());
    }
    let shown = if me.champion_id > 0 {
        me.champion_id
    } else {
        me.hover_id
    };
    let lane = me
        .position
        .as_deref()
        .and_then(champselect::lolalytics_lane)
        .map(String::from);
    let key = format!(
        "{}:{}",
        me.champion_id,
        lane.as_deref().unwrap_or("default")
    );
    let start = me.locked && me.champion_id > 0 && {
        let mut auto = shared.auto_key.lock().await;
        let changed = auto.as_deref() != Some(key.as_str());
        if changed {
            *auto = Some(key.clone());
            *shared.current_key.lock().await = Some(key.clone());
        }
        changed
    };
    update(app, shared, |s| {
        s.in_champ_select = true;
        s.locked = me.locked;
        if !start && s.build.is_none() && shown > 0 {
            s.champion_id = shown;
        }
    })
    .await;
    if start {
        let auto = shared.settings.lock().await.auto_import;
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move {
            load_build(&app, &shared, me.champion_id, lane, auto, key).await;
        });
    }
}

/// Busca o build no lolalytics, mostra e (se pedido) importa.
pub async fn load_build(
    app: &AppHandle,
    shared: &Arc<Shared>,
    champion_id: i64,
    lane: Option<String>,
    do_import: bool,
    key: String,
) {
    update(app, shared, |s| {
        s.champion_id = champion_id;
        s.lane = lane.clone();
        s.loading = true;
        s.error = None;
        s.warning = None;
        s.import = None;
    })
    .await;
    let assets = shared.assets.read().await.clone();
    let Some(champ) = assets
        .as_ref()
        .and_then(|a| a.champion(champion_id).cloned())
    else {
        update(app, shared, |s| {
            s.loading = false;
            s.error = Some("campeão desconhecido (cliente fechado?)".into());
        })
        .await;
        return;
    };
    let settings = shared.settings.lock().await.clone();
    let mut q = Query {
        slug: lolalytics_slug(&champ.alias),
        lane,
        tier: settings.tier.clone(),
        region: settings.region.clone(),
        patch: settings.patch_opt(),
    };
    let result = shared
        .lola
        .build_full(&q)
        .await
        .and_then(|f| model::parse_build_full(&f.data).map(|b| (b, f.stale_warning)));
    if shared.current_key.lock().await.as_deref() != Some(key.as_str()) {
        return; // trocou de campeão enquanto buscava
    }
    let ok = result.is_ok();
    match result {
        Err(e) => {
            update(app, shared, |s| {
                s.loading = false;
                s.build = None;
                s.error = Some(e);
            })
            .await
        }
        Ok((mut build, warning)) => {
            build.slug = q.slug.clone();
            if !build.lane.is_empty() {
                q.lane = Some(build.lane.clone());
            }
            let url = q.page_url();
            update(app, shared, |s| {
                s.loading = false;
                s.lane = q.lane.clone();
                s.page_url = Some(url);
                s.warning = warning;
                s.build = Some(build);
            })
            .await;
        }
    }
    if ok && do_import {
        let report = run_import(shared).await;
        update(app, shared, |s| match report {
            Ok(r) => s.import = Some(r),
            Err(e) => s.warning = Some(e),
        })
        .await;
    }
}

/// Importa o build em tela para o cliente, conforme as configurações.
pub async fn run_import(shared: &Shared) -> Result<ImportReport, String> {
    let lcu = shared
        .lcu
        .read()
        .await
        .clone()
        .ok_or("cliente do League fechado")?;
    let assets = shared
        .assets
        .read()
        .await
        .clone()
        .ok_or("dados do cliente ainda não carregados")?;
    let (build, champion_id) = {
        let s = shared.snapshot.lock().await;
        (
            s.build.clone().ok_or("nenhum build carregado")?,
            s.champion_id,
        )
    };
    let settings = shared.settings.lock().await.clone();
    // Os dados são de ranqueada no Summoner's Rift: em outros mapas não importamos nada.
    if let Ok(gs) = lcu.get("/lol-gameflow/v1/session").await {
        let map = gs
            .pointer("/map/id")
            .or_else(|| gs.pointer("/gameData/queue/mapId"))
            .and_then(Value::as_i64);
        if map.is_some_and(|m| m != 11) {
            return Err(
                "modo fora do Summoner's Rift: só mostrando o build, nada importado".into(),
            );
        }
    }
    let name = assets
        .champion(champion_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let lane_label = champselect::lane_label(&build.lane);
    let mut rep = ImportReport::default();

    if settings.import_runes {
        let set = if settings.rune_choice == "win" {
            build.runes_win.as_ref().or(build.runes_pick.as_ref())
        } else {
            build.runes_pick.as_ref()
        };
        rep.runes = Some(match set {
            None => Outcome::fail("lolalytics sem página de runas"),
            Some(set) => match runes::page_body(&runes::page_name(&name, lane_label), set, &assets)
            {
                Err(e) => Outcome::fail(e),
                Ok(body) => {
                    Outcome::from(runes::apply(&lcu, &body, settings.replace_current_page).await)
                }
            },
        });
    }
    if settings.import_items {
        let set = itemset::build_item_set(
            &build,
            champion_id,
            &name,
            lane_label,
            &assets,
            settings.min_games,
        );
        rep.items = Some(Outcome::from(itemset::apply(&lcu, set).await));
    }
    if settings.import_spells {
        let rec: Vec<i64> = build
            .spells
            .first()
            .map(|c| c.ids.iter().filter_map(|s| s.parse().ok()).collect())
            .unwrap_or_default();
        rep.spells = Some(if rec.len() != 2 {
            Outcome::fail("lolalytics sem feitiços")
        } else {
            match lcu
                .get("/lol-champ-select/v1/session")
                .await
                .ok()
                .as_ref()
                .and_then(champselect::parse)
            {
                None => Outcome::fail("fora da seleção de campeões"),
                Some(me) => {
                    let pair = spells::arrange(
                        (rec[0], rec[1]),
                        (me.spell1, me.spell2),
                        settings.flash_slot,
                    );
                    if pair == (me.spell1, me.spell2) {
                        Outcome::ok("feitiços já estavam certos")
                    } else {
                        Outcome::from(spells::apply(&lcu, pair).await)
                    }
                }
            }
        });
    }
    Ok(rep)
}

/// Ícones do cliente para a janela (`http://lcu.localhost/lol-game-data/assets/...`).
pub async fn fetch_icon(shared: &Shared, path: &str) -> Result<(Vec<u8>, String), String> {
    if !path.starts_with("/lol-game-data/assets/") || path.contains("..") {
        return Err("caminho não permitido".into());
    }
    if let Some(hit) = shared.icons.lock().await.get(path) {
        return Ok(hit.clone());
    }
    let lcu = shared.lcu.read().await.clone().ok_or("cliente fechado")?;
    let got = lcu.get_bytes(path).await.map_err(|e| e.to_string())?;
    let mut cache = shared.icons.lock().await;
    if cache.len() > 600 {
        cache.clear();
    }
    cache.insert(path.to_string(), got.clone());
    Ok(got)
}

/// Busca manual (fora da seleção, ou outra lane): só mostra.
pub async fn lookup(app: &AppHandle, shared: &Arc<Shared>, champion_id: i64, lane: Option<String>) {
    let key = format!(
        "lookup:{champion_id}:{}",
        lane.as_deref().unwrap_or("default")
    );
    *shared.current_key.lock().await = Some(key.clone());
    load_build(app, shared, champion_id, lane, false, key).await;
}
