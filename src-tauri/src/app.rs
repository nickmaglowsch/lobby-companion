//! Estado do app e o laço principal: espera o cliente, ouve o WebSocket e reage
//! à seleção de campeões (buscar build → importar → avisar a janela).

use crate::assets::{Assets, lolalytics_slug};
use crate::bans::{self, BanPick};
use crate::champselect::{self, Ally, Draft, Me};
use crate::import::{ban, itemset, runes, spells};
use crate::lcu::ws::{self, LcuEvent};
use crate::lcu::{self, Lcu};
use crate::lola::extra::{self, Counters, Team, TierList};
use crate::lola::model::{self, Build};
use crate::lola::{Ep, Lolalytics, Query};
use crate::own_runes::{self, Chosen, OwnEntry, OwnPage, OwnPages};
use crate::picks::{self, PickSuggestion, SynergyLine};
use crate::settings::Settings;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
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

impl ImportReport {
    /// Atualiza só o que esta importação tocou (mudar a runa não apaga o aviso dos itens).
    fn merge(&mut self, new: ImportReport) {
        self.runes = new.runes.or(self.runes.take());
        self.items = new.items.or(self.items.take());
        self.spells = new.spells.or(self.spells.take());
    }
}

/// O que uma importação deve escrever no cliente.
#[derive(Debug, Clone, Copy)]
pub struct Scope {
    pub runes: bool,
    pub items: bool,
    pub spells: bool,
}

impl Scope {
    pub const ALL: Self = Self {
        runes: true,
        items: true,
        spells: true,
    };
    pub const RUNES: Self = Self {
        runes: true,
        items: false,
        spells: false,
    };
    pub const ITEMS: Self = Self {
        runes: false,
        items: true,
        spells: false,
    };
}

/// Tudo o que a janela mostra. Vai inteiro no evento "snapshot".
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connected: bool,
    pub phase: String,
    pub in_champ_select: bool,
    /// Na seleção, ainda há ban por fazer (layout "Por fase": mostra os bans).
    pub banning: bool,
    pub locked: bool,
    /// É a sua vez de banir: clicar numa sugestão de ban bane no cliente.
    pub can_ban: bool,
    pub champion_id: i64,
    /// Lane do lolalytics (`None` = a mais jogada do campeão).
    pub lane: Option<String>,
    pub loading: bool,
    pub build: Option<Build>,
    pub page_url: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
    pub import: Option<ImportReport>,
    /// Campeão do outro lado escolhido na janela (0 = build geral).
    pub vs_champion_id: i64,
    pub vs_lane: Option<String>,
    /// Campeões já visíveis do outro time (atalhos para escolher o confronto).
    pub enemy_picks: Vec<i64>,
    /// Campeões escolhidos ou declarados do seu time (você incluso), para a composição.
    pub ally_picks: Vec<i64>,
    pub bans: Option<BanAdvice>,
    /// Sugestão de pick para a sua posição (só na seleção).
    pub picks: Option<PickAdvice>,
    /// Sinergia do seu campeão (escolhido ou declarado) com os aliados declarados.
    pub synergy: Option<SynergyAdvice>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BanAdvice {
    pub loading: bool,
    /// Campeões do seu time usados na conta.
    pub based_on: Vec<Ally>,
    pub picks: Vec<BanPick>,
    pub note: Option<String>,
    /// Ninguém do time declarou: bans pela força geral na sua lane (tier list).
    pub by_tier: bool,
    pub lane: Option<String>,
    pub page_url: Option<String>,
    /// Contra o seu pick: oponentes da sua lane com confronto ruim e pick provável
    /// (sem repetir os de `picks`).
    pub lane_picks: Vec<BanPick>,
    /// Seu campeão e lane usados na lista individual.
    pub lane_for: Option<Ally>,
    pub lane_note: Option<String>,
}

const DECLARE_NOTE: &str = "Declare ou escolha seu campeão para ver os bans contra ele.";

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PickAdvice {
    pub loading: bool,
    /// Sua lane (lolalytics).
    pub lane: Option<String>,
    /// De onde vieram os candidatos: "mastery" (seu pool) ou "tier" (tier list da lane).
    pub pool: String,
    pub suggestions: Vec<PickSuggestion>,
    /// Inimigos e aliados usados na conta.
    pub enemies: Vec<i64>,
    pub allies: Vec<Ally>,
    pub note: Option<String>,
    /// Tier list da lane no lolalytics (atribuição).
    pub page_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SynergyAdvice {
    pub champion_id: i64,
    pub lane: String,
    /// WR geral do seu campeão na lane (tier list).
    pub base_wr: Option<f64>,
    pub lines: Vec<SynergyLine>,
    pub page_url: Option<String>,
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
    /// Uma importação por vez: cliques rápidos não podem intercalar apagar/criar página.
    import_lock: Mutex<()>,
    /// Campeão do outro lado e a lane dele (escolhidos na janela).
    vs: Mutex<Option<(i64, Option<String>)>>,
    draft: Mutex<Option<Draft>>,
    /// Só a conta de bans mais nova escreve no snapshot.
    bans_gen: AtomicU64,
    /// Idem para a sugestão de pick.
    picks_gen: AtomicU64,
    /// Sua lane do lolalytics (posição atribuída), atualizada antes de recalcular bans e picks.
    my_lane: Mutex<Option<String>>,
    /// Maestria do jogador local `[(campeão, pontos)]`, lida uma vez por seleção (GET).
    mastery: Mutex<Option<Vec<(i64, u64)>>>,
    /// Páginas de runas próprias (`rune-pages.json`, ao lado de `settings.json`).
    own_runes: Mutex<OwnPages>,
    own_runes_path: PathBuf,
}

impl Shared {
    pub fn new(settings_path: PathBuf, cache_dir: PathBuf) -> Self {
        let own_runes_path = settings_path.with_file_name("rune-pages.json");
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
            import_lock: Mutex::new(()),
            vs: Mutex::new(None),
            draft: Mutex::new(None),
            bans_gen: AtomicU64::new(0),
            picks_gen: AtomicU64::new(0),
            my_lane: Mutex::new(None),
            mastery: Mutex::new(None),
            own_runes: Mutex::new(OwnPages::load(&own_runes_path)),
            own_runes_path,
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
    *shared.draft.lock().await = None;
    if was || shared.snapshot.lock().await.connected {
        update(app, shared, |s| {
            s.connected = false;
            s.phase = "None".into();
            s.in_champ_select = false;
            s.banning = false;
            s.can_ban = false;
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
            // Seleção nova: o confronto e os bans da anterior não valem mais.
            *shared.vs.lock().await = None;
            *shared.draft.lock().await = None;
            *shared.my_lane.lock().await = None;
            *shared.mastery.lock().await = None;
            update(app, shared, |s| {
                s.phase = phase.into();
                s.in_champ_select = true;
                s.locked = false;
                s.import = None;
                s.error = None;
                s.warning = None;
                s.vs_champion_id = 0;
                s.vs_lane = None;
                s.enemy_picks.clear();
                s.ally_picks.clear();
                s.bans = None;
                s.picks = None;
                s.synergy = None;
            })
            .await;
            crate::show_main(app);
        }
        "GameStart" | "InProgress" | "Reconnect" => {
            update(app, shared, |s| {
                s.phase = phase.into();
                s.in_champ_select = false;
                s.banning = false;
                s.can_ban = false;
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
        *shared.draft.lock().await = None;
        update(app, shared, |s| {
            s.in_champ_select = false;
            s.banning = false;
            s.can_ban = false;
            s.locked = false;
            s.enemy_picks.clear();
            s.ally_picks.clear();
        })
        .await;
        return;
    };
    let draft = session.as_ref().and_then(champselect::parse_draft);
    // Vez de banir: só avisa a janela quando muda (a sessão chega a cada segundo).
    let can_ban = session
        .as_ref()
        .and_then(champselect::my_ban_action)
        .is_some();
    if shared.snapshot.lock().await.can_ban != can_ban {
        update(app, shared, |s| s.can_ban = can_ban).await;
    }
    let my_lane = me
        .position
        .as_deref()
        .and_then(champselect::lolalytics_lane)
        .map(String::from);
    let lane_changed = {
        let mut prev = shared.my_lane.lock().await;
        let changed = *prev != my_lane;
        *prev = my_lane;
        changed
    };
    let draft_changed = {
        let mut prev = shared.draft.lock().await;
        let changed = *prev != draft;
        if changed {
            prev.clone_from(&draft);
        }
        changed
    };
    if draft_changed || lane_changed {
        let allies = draft
            .as_ref()
            .map(|d| d.allies.iter().map(|a| a.champion_id).collect())
            .unwrap_or_default();
        let banning = draft.as_ref().is_some_and(|d| d.banning);
        let enemies = draft.map(|d| d.enemies).unwrap_or_default();
        update(app, shared, |s| {
            s.in_champ_select = true;
            s.banning = banning;
            s.enemy_picks = enemies;
            s.ally_picks = allies;
        })
        .await;
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move {
            refresh_bans(&app, &shared).await;
            refresh_picks(&app, &shared).await;
        });
    }
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
    let vs = shared.vs.lock().await.clone();
    let vs_champ = vs.as_ref().and_then(|(id, l)| {
        let c = assets.as_ref()?.champion(*id)?;
        Some((lolalytics_slug(&c.alias), c.name.clone(), l.clone()))
    });
    let mut q = Query {
        slug: lolalytics_slug(&champ.alias),
        lane,
        tier: settings.tier.clone(),
        region: settings.region.clone(),
        patch: settings.patch_opt(),
        ..Default::default()
    };
    let result = match &vs_champ {
        Some((slug, vs_name, vs_lane)) if *slug != q.slug => {
            let vq = Query {
                vs: Some(slug.clone()),
                // Sem lane escolhida: o oponente na mesma lane que você.
                vs_lane: vs_lane.clone().or_else(|| q.lane.clone()),
                ..q.clone()
            };
            match fetch_build(shared, &vq).await {
                Ok((b, w)) if b.n > 0 => {
                    q = vq;
                    Ok((b, w))
                }
                _ => {
                    let lane = vq.vs_lane.as_deref().map(champselect::lane_label);
                    let msg = format!(
                        "o lolalytics não tem partidas de {} contra {vs_name}{}; mostrando o build geral",
                        champ.name,
                        lane.map(|l| format!(" ({l})")).unwrap_or_default()
                    );
                    fetch_build(shared, &q).await.map(|(b, w)| {
                        let warn = match w {
                            Some(w) => format!("{msg}. {w}"),
                            None => msg,
                        };
                        (b, Some(warn))
                    })
                }
            }
        }
        _ => fetch_build(shared, &q).await,
    };
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
    // Fora da seleção, os bans são os do campeão em tela; na seleção, vêm do time (`handle_session`).
    if ok && !shared.snapshot.lock().await.in_champ_select {
        refresh_bans(app, shared).await;
    }
}

async fn fetch_build(shared: &Shared, q: &Query) -> Result<(Build, Option<String>), String> {
    shared
        .lola
        .build_full(q)
        .await
        .and_then(|f| model::parse_build_full(&f.data).map(|b| (b, f.stale_warning)))
}

/// Recalcula a sugestão de bans: na seleção, pelos campeões do seu time (escolhidos ou
/// declarados); fora dela, pelo campeão em tela. Um build geral por campeão (cache de 6 h).
pub async fn refresh_bans(app: &AppHandle, shared: &Arc<Shared>) {
    let generation = shared.bans_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let settings = shared.settings.lock().await.clone();
    if !settings.suggest_bans {
        update(app, shared, |s| s.bans = None).await;
        return;
    }
    let (in_select, champion_id, lane) = {
        let s = shared.snapshot.lock().await;
        (s.in_champ_select, s.champion_id, s.lane.clone())
    };
    let draft = shared.draft.lock().await.clone();
    let (allies, unavailable) = match draft {
        Some(d) if in_select => (d.allies, d.unavailable),
        _ if !in_select && champion_id > 0 => (
            vec![Ally {
                champion_id,
                lane,
                me: true,
            }],
            vec![champion_id],
        ),
        _ => (Vec::new(), Vec::new()),
    };
    let my_lane = shared.my_lane.lock().await.clone();
    if let Some(lane) = my_lane.filter(|_| in_select && allies.is_empty()) {
        bans_by_tier(app, shared, generation, &settings, lane, &unavailable).await;
        return;
    }
    if allies.is_empty() {
        update(app, shared, |s| {
            s.bans = in_select.then(|| BanAdvice {
                note: Some("Declare ou escolha um campeão para ver sugestões de ban.".into()),
                ..Default::default()
            })
        })
        .await;
        return;
    }
    update(app, shared, |s| {
        // Os da tier list não valem para a conta pelo time.
        let (picks, lane_picks, lane_for) = s
            .bans
            .take()
            .filter(|b| !b.by_tier)
            .map(|b| (b.picks, b.lane_picks, b.lane_for))
            .unwrap_or_default();
        s.bans = Some(BanAdvice {
            loading: true,
            based_on: allies.clone(),
            picks,
            lane_picks,
            lane_for,
            ..Default::default()
        });
    })
    .await;
    let assets = shared.assets.read().await.clone();
    let mut builds: Vec<(Ally, Build)> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for ally in allies.iter().take(5) {
        let Some(c) = assets.as_ref().and_then(|a| a.champion(ally.champion_id)) else {
            continue;
        };
        let q = Query {
            slug: lolalytics_slug(&c.alias),
            lane: ally.lane.clone(),
            tier: settings.tier.clone(),
            region: settings.region.clone(),
            patch: settings.patch_opt(),
            ..Default::default()
        };
        match fetch_build(shared, &q).await {
            Ok((b, _)) if !b.matchups.is_empty() => builds.push((ally.clone(), b)),
            _ => missing.push(c.name.clone()),
        }
        if shared.bans_gen.load(Ordering::SeqCst) != generation {
            return; // o time mudou enquanto buscava; a conta nova assume
        }
    }
    let inputs: Vec<(Ally, &Build)> = builds.iter().map(|(a, b)| (a.clone(), b)).collect();
    let picks = bans::suggest(&inputs, &unavailable, 5);
    // Lista individual: só o seu campeão, oponentes da sua lane, sem repetir a do time.
    let in_team: Vec<i64> = picks.iter().map(|p| p.champion_id).collect();
    let mine = builds.iter().find(|(a, _)| a.me);
    let lane_picks = mine
        .map(|(me, b)| bans::lane_suggest(me, b, &unavailable, &in_team, 5))
        .unwrap_or_default();
    let lane_for = mine.map(|(me, b)| Ally {
        lane: Some(b.lane.clone())
            .filter(|l| !l.is_empty())
            .or(me.lane.clone()),
        ..me.clone()
    });
    let lane_note = if mine.is_none() {
        Some(DECLARE_NOTE.to_string())
    } else if lane_picks.is_empty() {
        Some("Nenhum oponente provável da sua lane vai melhor contra o seu campeão (fora os da lista do time).".into())
    } else {
        None
    };
    let note = (!missing.is_empty()).then(|| format!("sem dados de {}", missing.join(", ")));
    if shared.bans_gen.load(Ordering::SeqCst) != generation {
        return;
    }
    update(app, shared, |s| {
        s.bans = Some(BanAdvice {
            loading: false,
            based_on: allies,
            picks,
            note,
            lane_picks,
            lane_for,
            lane_note,
            ..Default::default()
        })
    })
    .await;
}

/// Consulta de `list`/`counter`/`build-team` com os filtros das configurações.
fn extra_query(
    settings: &Settings,
    ep: Ep,
    slug: String,
    lane: Option<String>,
    vs_lane: Option<String>,
) -> Query {
    Query {
        ep,
        slug,
        lane,
        vs_lane,
        tier: settings.tier.clone(),
        region: settings.region.clone(),
        patch: settings.patch_opt(),
        ..Default::default()
    }
}

async fn fetch_list(shared: &Shared, q: &Query) -> Result<TierList, String> {
    shared
        .lola
        .get(q)
        .await
        .and_then(|f| extra::parse_list(&f.data))
}

async fn fetch_counter(shared: &Shared, q: &Query) -> Result<Counters, String> {
    shared
        .lola
        .get(q)
        .await
        .and_then(|f| extra::parse_counter(&f.data))
}

async fn fetch_team(shared: &Shared, q: &Query) -> Result<Team, String> {
    shared
        .lola
        .get(q)
        .await
        .and_then(|f| extra::parse_build_team(&f.data))
}

/// Ninguém do seu time declarou ainda: bans pela força geral na sua lane (uma tier list, com cache).
/// A sua lane e não todas: o oponente direto é quem mais pesa no seu jogo, e a mesma tier list
/// já serve para a sugestão de pick (sem requisição a mais).
async fn bans_by_tier(
    app: &AppHandle,
    shared: &Arc<Shared>,
    generation: u64,
    settings: &Settings,
    lane: String,
    unavailable: &[i64],
) {
    update(app, shared, |s| {
        let picks = s
            .bans
            .take()
            .filter(|b| b.by_tier)
            .map(|b| b.picks)
            .unwrap_or_default();
        s.bans = Some(BanAdvice {
            loading: true,
            picks,
            by_tier: true,
            lane: Some(lane.clone()),
            ..Default::default()
        });
    })
    .await;
    let q = extra_query(settings, Ep::List, String::new(), Some(lane.clone()), None);
    let result = fetch_list(shared, &q).await;
    if shared.bans_gen.load(Ordering::SeqCst) != generation {
        return;
    }
    let advice = match result {
        // Ninguém declarou, nem você: a lista individual espera o seu campeão.
        Ok(list) => BanAdvice {
            picks: picks::tier_bans(&list, unavailable, 5),
            by_tier: true,
            lane: Some(lane),
            page_url: Some(q.page_url()),
            lane_note: Some(DECLARE_NOTE.into()),
            ..Default::default()
        },
        Err(e) => BanAdvice {
            note: Some(format!("tier list indisponível: {e}")),
            by_tier: true,
            lane: Some(lane),
            lane_note: Some(DECLARE_NOTE.into()),
            ..Default::default()
        },
    };
    update(app, shared, |s| s.bans = Some(advice)).await;
}

/// Maestria do jogador local (só GET; uma vez por seleção).
async fn mastery(shared: &Shared) -> Result<Vec<(i64, u64)>, String> {
    if let Some(m) = shared.mastery.lock().await.clone() {
        return Ok(m);
    }
    let lcu = shared
        .lcu
        .read()
        .await
        .clone()
        .ok_or("cliente do League fechado")?;
    let v = lcu
        .get("/lol-champion-mastery/v1/local-player/champion-mastery")
        .await
        .map_err(|e| e.to_string())?;
    let m = extra::parse_mastery(&v);
    *shared.mastery.lock().await = Some(m.clone());
    Ok(m)
}

/// Recalcula a sugestão de pick e a sinergia quando a seleção muda. Requisições ao lolalytics
/// (cache de 6 h): a tier list da sua lane, o `build-team` do seu campeão (sinergia), um
/// `counter` por inimigo visível (até 5) e um `build-team` por aliado com lane (até 4).
pub async fn refresh_picks(app: &AppHandle, shared: &Arc<Shared>) {
    let generation = shared.picks_gen.fetch_add(1, Ordering::SeqCst) + 1;
    let stale = || shared.picks_gen.load(Ordering::SeqCst) != generation;
    let settings = shared.settings.lock().await.clone();
    let in_select = shared.snapshot.lock().await.in_champ_select;
    if !settings.suggest_picks || !in_select {
        update(app, shared, |s| {
            s.picks = None;
            s.synergy = None;
        })
        .await;
        return;
    }
    let Some(lane) = shared.my_lane.lock().await.clone() else {
        update(app, shared, |s| {
            s.picks = Some(PickAdvice {
                note: Some("Sem posição atribuída (às cegas?): sem sugestão de pick.".into()),
                ..Default::default()
            });
            s.synergy = None;
        })
        .await;
        return;
    };
    let draft = shared.draft.lock().await.clone().unwrap_or_default();
    let mates: Vec<Ally> = draft
        .allies
        .iter()
        .filter(|a| !a.me && a.lane.is_some())
        .take(4)
        .cloned()
        .collect();
    let enemies: Vec<i64> = draft.enemies.iter().take(5).copied().collect();
    update(app, shared, |s| {
        let prev = s.picks.take().map(|p| p.suggestions).unwrap_or_default();
        s.picks = Some(PickAdvice {
            loading: true,
            lane: Some(lane.clone()),
            pool: settings.pick_pool.clone(),
            suggestions: prev,
            enemies: enemies.clone(),
            allies: mates.clone(),
            ..Default::default()
        });
    })
    .await;
    let assets = shared.assets.read().await.clone();
    let slug = |id: i64| {
        assets
            .as_ref()
            .and_then(|a| a.champion(id))
            .map(|c| (lolalytics_slug(&c.alias), c.name.clone()))
    };
    let list_q = extra_query(&settings, Ep::List, String::new(), Some(lane.clone()), None);
    let list = match fetch_list(shared, &list_q).await {
        Ok(l) => l,
        Err(e) => {
            if !stale() {
                update(app, shared, |s| {
                    if let Some(p) = s.picks.as_mut() {
                        p.loading = false;
                        p.note = Some(format!("tier list indisponível: {e}"));
                    }
                })
                .await;
            }
            return;
        }
    };
    if stale() {
        return;
    }

    // Sinergia do seu campeão (escolhido ou declarado) com os aliados declarados.
    let me = draft.allies.iter().find(|a| a.me).cloned();
    let synergy = match me
        .as_ref()
        .and_then(|m| Some((m.champion_id, slug(m.champion_id)?)))
    {
        Some((id, (my_slug, _))) if !mates.is_empty() => {
            let q = extra_query(&settings, Ep::BuildTeam, my_slug, Some(lane.clone()), None);
            fetch_team(shared, &q).await.ok().map(|team| SynergyAdvice {
                champion_id: id,
                lane: lane.clone(),
                base_wr: list.get(id).map(|e| e.wr),
                lines: picks::synergy_lines(&team, &mates),
                page_url: Some(q.page_url()),
            })
        }
        _ => None,
    };
    if stale() {
        return;
    }
    update(app, shared, |s| s.synergy = synergy).await;

    // Candidatos: o seu pool (maestria) ou a tier list da lane. O seu próprio campeão continua
    // valendo (dá para ver como ele se compara); o resto do que está indisponível sai.
    let my_champ = me.as_ref().map(|m| m.champion_id).unwrap_or(0);
    let exclude: Vec<i64> = draft
        .unavailable
        .iter()
        .copied()
        .filter(|id| *id != my_champ)
        .collect();
    let mut notes: Vec<String> = Vec::new();
    let mut pool_kind = settings.pick_pool.clone();
    let mut owned: Vec<(i64, u64)> = Vec::new();
    let mut candidates = Vec::new();
    if pool_kind == "mastery" {
        match mastery(shared).await {
            Ok(m) => {
                candidates = picks::pool_from_mastery(&m, &list, &exclude, picks::POOL_SIZE);
                owned = m;
            }
            Err(e) => notes.push(format!("maestria indisponível ({e})")),
        }
        if candidates.is_empty() {
            notes.push("nenhum campeão do seu pool joga essa lane; usando a tier list".into());
            pool_kind = "tier".into();
        }
    }
    if pool_kind != "mastery" {
        candidates = picks::pool_from_tier(&list, &exclude, picks::POOL_SIZE);
    }

    let mut counters: Vec<(i64, String, Counters)> = Vec::new();
    for &e in &enemies {
        let Some((e_slug, name)) = slug(e) else {
            continue;
        };
        let e_lane = picks::enemy_lane(&list, e, &lane);
        let q = extra_query(
            &settings,
            Ep::Counter,
            e_slug,
            Some(e_lane.clone()),
            Some(lane.clone()),
        );
        match fetch_counter(shared, &q).await {
            Ok(c) => counters.push((e, e_lane, c)),
            Err(_) => notes.push(format!("sem dados de counter de {name}")),
        }
        if stale() {
            return;
        }
    }
    let mut teams: Vec<(i64, String, Team)> = Vec::new();
    for a in &mates {
        let (Some((a_slug, name)), Some(a_lane)) = (slug(a.champion_id), a.lane.clone()) else {
            continue;
        };
        let q = extra_query(&settings, Ep::BuildTeam, a_slug, Some(a_lane.clone()), None);
        match fetch_team(shared, &q).await {
            Ok(t) => teams.push((a.champion_id, a_lane, t)),
            Err(_) => notes.push(format!("sem dados de sinergia de {name}")),
        }
        if stale() {
            return;
        }
    }
    let enemy_inputs: Vec<picks::Enemy> = counters
        .iter()
        .map(|(id, l, c)| picks::Enemy {
            champion_id: *id,
            lane: l.clone(),
            counters: c,
        })
        .collect();
    let mate_inputs: Vec<picks::Mate> = teams
        .iter()
        .map(|(id, l, t)| picks::Mate {
            champion_id: *id,
            lane: l.clone(),
            team: t,
        })
        .collect();
    let suggestions = picks::suggest(
        &lane,
        &list,
        &candidates,
        &enemy_inputs,
        &mate_inputs,
        &owned,
        picks::TOP,
    );
    if stale() {
        return;
    }
    update(app, shared, |s| {
        s.picks = Some(PickAdvice {
            loading: false,
            lane: Some(lane),
            pool: pool_kind,
            suggestions,
            enemies,
            allies: mates,
            note: (!notes.is_empty()).then(|| notes.join("; ")),
            page_url: Some(list_q.page_url()),
        })
    })
    .await;
}

/// Escolhe (ou limpa) o campeão do outro lado e recarrega o build em tela.
pub async fn set_vs(
    app: &AppHandle,
    shared: &Arc<Shared>,
    vs_champion_id: Option<i64>,
    vs_lane: Option<String>,
) {
    let vs = vs_champion_id.filter(|id| *id > 0);
    *shared.vs.lock().await = vs.map(|id| (id, vs_lane.clone()));
    let (champion_id, lane) = {
        let s = shared.snapshot.lock().await;
        (s.champion_id, s.lane.clone())
    };
    update(app, shared, |s| {
        s.vs_champion_id = vs.unwrap_or(0);
        s.vs_lane = vs.and(vs_lane);
    })
    .await;
    if champion_id > 0 {
        lookup(app, shared, champion_id, lane).await;
    }
}

/// Importa tudo do build em tela para o cliente, conforme as configurações.
pub async fn run_import(shared: &Shared) -> Result<ImportReport, String> {
    run_import_scoped(shared, Scope::ALL).await
}

/// A janela pode mandar sozinha o que está na tela para o cliente agora?
async fn should_auto_apply(shared: &Shared, champion_id: i64) -> bool {
    if !shared.settings.lock().await.auto_import {
        return false;
    }
    let (phase, in_select) = {
        let s = shared.snapshot.lock().await;
        (s.phase.clone(), s.in_champ_select)
    };
    let me = shared.me.lock().await.clone();
    champselect::may_apply(&phase, in_select, me.as_ref(), champion_id)
}

/// Algo mudou na janela (runa, amostra mínima): leva a mudança para o cliente.
pub async fn apply_change(app: &AppHandle, shared: &Arc<Shared>, scope: Scope) {
    let (champion_id, has_build) = {
        let s = shared.snapshot.lock().await;
        (s.champion_id, s.build.is_some())
    };
    if !has_build || !should_auto_apply(shared, champion_id).await {
        return;
    }
    let report = run_import_scoped(shared, scope).await;
    update(app, shared, |s| match report {
        Ok(r) => s.import.get_or_insert_with(Default::default).merge(r),
        Err(e) => s.warning = Some(e),
    })
    .await;
}

async fn run_import_scoped(shared: &Shared, scope: Scope) -> Result<ImportReport, String> {
    let _one_at_a_time = shared.import_lock.lock().await;
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
    // Build de confronto: "vs Viego" no nome da página e no título do conjunto.
    let vs_name = build
        .vs
        .as_ref()
        .and_then(|v| assets.champion(v.champion_id))
        .map(|c| c.name.clone());
    let set_label = match &vs_name {
        Some(v) => format!("{lane_label} vs {v}"),
        None => lane_label.to_string(),
    };
    // O nome da página é curto (o cliente corta): com confronto, sai a lane.
    let page_label = match &vs_name {
        Some(v) => format!("vs {v}"),
        None => lane_label.to_string(),
    };
    let mut rep = ImportReport::default();

    if scope.runes && settings.import_runes {
        let own = shared
            .own_runes
            .lock()
            .await
            .get(champion_id, &build.lane)
            .cloned();
        let body = match own_runes::choose(
            &settings.rune_choice,
            build.runes_pick.as_ref(),
            build.runes_win.as_ref(),
            own.as_ref(),
        ) {
            Chosen::None => Err("lolalytics sem página de runas".to_string()),
            Chosen::Lola(set) => {
                runes::page_body(&runes::page_name(&name, &page_label), set, &assets)
            }
            // A sua página vale para qualquer confronto: o nome diz que é a sua.
            Chosen::Own(page) => {
                runes::own_page_body(&runes::page_name(&name, "Minha"), page, &assets)
            }
        };
        rep.runes = Some(match body {
            Err(e) => Outcome::fail(e),
            Ok(body) => {
                Outcome::from(runes::apply(&lcu, &body, settings.replace_current_page).await)
            }
        });
    }
    if scope.items && settings.import_items {
        let set = itemset::build_item_set(
            &build,
            champion_id,
            &name,
            &set_label,
            &assets,
            build.min_games(settings.min_games),
        );
        rep.items = Some(Outcome::from(itemset::apply(&lcu, set).await));
    }
    if scope.spells && settings.import_spells {
        let rec: Vec<i64> = build
            .spells
            .first()
            .map(|c| c.ids.iter().filter_map(|s| s.parse().ok()).collect())
            .unwrap_or_default();
        let session = lcu
            .get("/lol-champ-select/v1/session")
            .await
            .ok()
            .as_ref()
            .and_then(champselect::parse);
        rep.spells = match session {
            // Feitiços só existem na seleção, e só valem para o campeão do próprio jogador.
            None => None,
            Some(me) if me.current_champion() != champion_id => None,
            Some(_) if rec.len() != 2 => Some(Outcome::fail("lolalytics sem feitiços")),
            Some(me) => {
                let pair = spells::arrange(
                    (rec[0], rec[1]),
                    (me.spell1, me.spell2),
                    settings.flash_slot,
                );
                Some(if pair == (me.spell1, me.spell2) {
                    Outcome::ok("feitiços já estavam certos")
                } else {
                    Outcome::from(spells::apply(&lcu, pair).await)
                })
            }
        };
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

/// Troca de campeão, lane ou filtro na janela: carrega e, quando pode, já leva para o cliente.
pub async fn lookup(app: &AppHandle, shared: &Arc<Shared>, champion_id: i64, lane: Option<String>) {
    let vs = shared.vs.lock().await.clone();
    let key = format!(
        "lookup:{champion_id}:{}:{vs:?}",
        lane.as_deref().unwrap_or("default")
    );
    *shared.current_key.lock().await = Some(key.clone());
    let apply = should_auto_apply(shared, champion_id).await;
    load_build(app, shared, champion_id, lane, apply, key).await;
}

/// Campeão e lane do build em tela (a chave da página própria que a janela mostra).
async fn view_key(shared: &Shared) -> Option<(i64, String)> {
    let s = shared.snapshot.lock().await;
    let lane = s.build.as_ref()?.lane.clone();
    (s.champion_id > 0 && !lane.is_empty()).then_some((s.champion_id, lane))
}

fn valid_lane(lane: &str) -> Result<(), String> {
    if champselect::lane_label(lane).is_empty() {
        return Err(format!("lane desconhecida: {lane}"));
    }
    Ok(())
}

/// Grava o arquivo e devolve a lista para a janela.
fn store_own(shared: &Shared, pages: &OwnPages) -> Result<Vec<OwnEntry>, String> {
    pages
        .save(&shared.own_runes_path)
        .map_err(|e| format!("não deu para gravar as suas páginas: {e}"))?;
    Ok(pages.entries())
}

pub async fn own_pages(shared: &Shared) -> Vec<OwnEntry> {
    shared.own_runes.lock().await.entries()
}

/// Salva a página atual do cliente (só GET) como a sua para o campeão e lane, já escolhida.
pub async fn save_own_runes(
    app: &AppHandle,
    shared: &Arc<Shared>,
    champion_id: i64,
    lane: String,
) -> Result<Vec<OwnEntry>, String> {
    valid_lane(&lane)?;
    let lcu = shared
        .lcu
        .read()
        .await
        .clone()
        .ok_or("abra o cliente do League para ler a página atual")?;
    let current = lcu
        .get("/lol-perks/v1/currentpage")
        .await
        .map_err(|e| e.to_string())?;
    let page = OwnPage::from_client(&current, own_runes::now_secs())?;
    if let Some(a) = shared.assets.read().await.as_ref() {
        page.check(a)?;
    }
    let list = {
        let mut pages = shared.own_runes.lock().await;
        pages.set(champion_id, &lane, page);
        store_own(shared, &pages)?
    };
    // A página já está no cliente (é a atual): nada a importar, só o aviso.
    if view_key(shared).await == Some((champion_id, lane)) {
        update(app, shared, |s| {
            s.import.get_or_insert_with(Default::default).runes =
                Some(Outcome::ok("página atual do cliente salva como a sua"));
        })
        .await;
    }
    Ok(list)
}

/// Apaga a sua página; se era a escolhida no build em tela, a importação volta para o lolalytics.
pub async fn delete_own_runes(
    app: &AppHandle,
    shared: &Arc<Shared>,
    champion_id: i64,
    lane: String,
) -> Result<Vec<OwnEntry>, String> {
    let (list, was_active) = {
        let mut pages = shared.own_runes.lock().await;
        let old = pages.remove(champion_id, &lane);
        (store_own(shared, &pages)?, old.is_some_and(|p| p.active))
    };
    if was_active && view_key(shared).await == Some((champion_id, lane)) {
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move { apply_change(&app, &shared, Scope::RUNES).await });
    }
    Ok(list)
}

/// Mais jogada / Maior WR / Minha no build em tela. "Minha" vale só para esse campeão e lane;
/// as outras duas também viram a escolha geral (`runeChoice`), como antes.
pub async fn choose_runes(
    app: &AppHandle,
    shared: &Arc<Shared>,
    choice: String,
) -> Result<Vec<OwnEntry>, String> {
    if !matches!(choice.as_str(), "pick" | "win" | "mine") {
        return Err(format!("escolha de runas desconhecida: {choice}"));
    }
    let key = view_key(shared).await;
    let mut changed = false;
    let list = {
        let mut pages = shared.own_runes.lock().await;
        let has_own = key
            .as_ref()
            .is_some_and(|(id, lane)| pages.get(*id, lane).is_some());
        if choice == "mine" && !has_own {
            return Err("nenhuma página sua salva para esse campeão nessa lane".into());
        }
        if let Some((id, lane)) = &key {
            changed = pages.set_active(*id, lane, choice == "mine");
        }
        if changed {
            store_own(shared, &pages)?
        } else {
            pages.entries()
        }
    };
    if choice != "mine" {
        let mut settings = shared.settings.lock().await;
        if settings.rune_choice != choice {
            settings.rune_choice = choice;
            settings
                .save(&shared.settings_path)
                .map_err(|e| e.to_string())?;
            changed = true;
        }
    }
    if changed {
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move { apply_change(&app, &shared, Scope::RUNES).await });
    }
    Ok(list)
}

/// Bane o campeão clicado na sugestão de ban, se for a sua vez (sessão lida de novo agora).
pub async fn ban_champion(shared: &Shared, champion_id: i64) -> Result<(), String> {
    let lcu = shared
        .lcu
        .read()
        .await
        .clone()
        .ok_or("cliente do League fechado")?;
    ban::apply(&lcu, champion_id).await
}
