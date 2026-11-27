//! Estado do app e o laço principal: espera o cliente, ouve o WebSocket e reage
//! à seleção de campeões (buscar build → importar → avisar a janela).
//!
//! Concorrência: o laço do WebSocket é o único que escreve o estado da seleção ([`Selection`]);
//! buscas e contas rodam em tarefas soltas. Cada tipo de conta (build em tela, bans, picks) tem
//! um contador de geração: quem começa sobe o contador, e só a geração atual escreve no snapshot
//! (conferido com o snapshot travado, em [`update_if`]). Assim uma resposta atrasada nunca
//! escreve por cima de uma mais nova, nem deixa "carregando" para trás.

use crate::assets::{Assets, lolalytics_slug};
use crate::bans::{self, BanPick};
use crate::champselect::{self, Ally, Draft, Me};
use crate::i18n::{self, Lang};
use crate::import::{ban, itemset, runes, spells};
use crate::lcu::ws::{self, LcuEvent};
use crate::lcu::{self, Lcu};
use crate::lola::extra::{self, Counters, Team};
use crate::lola::model::{self, Build};
use crate::lola::{Ep, Lolalytics, Query};
use crate::metric::Metric;
use crate::own_runes::{self, Chosen, OwnEntry, OwnPage, OwnPages};
use crate::picks::{self, PickSuggestion, SynergyLine};
use crate::settings::Settings;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, PoisonError};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio::task::AbortHandle;

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
    /// `Arc`: importar e a composição leem o build sem copiar.
    pub build: Option<Arc<Build>>,
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
    /// Idioma da janela (locale do cliente, ex. "pt_BR"): configuração → cliente → sistema → inglês.
    pub locale: String,
    /// Nesta seleção o app trocou a tela cheia do jogo por "Sem bordas" (overlay ligado).
    pub borderless_switched: bool,
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
    /// Métrica dos ganhos (`gain`): pontos de WR ou probabilidade de vitória adicionada.
    pub metric: Metric,
    /// Contra o seu pick: oponentes da sua lane com confronto ruim e pick provável
    /// (sem repetir os de `picks`).
    pub lane_picks: Vec<BanPick>,
    /// Seu campeão e lane usados na lista individual.
    pub lane_for: Option<Ally>,
    pub lane_note: Option<String>,
}

fn declare_note() -> String {
    crate::t!("bans.declareMine")
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PickAdvice {
    pub loading: bool,
    /// Sua lane (lolalytics).
    pub lane: Option<String>,
    /// De onde vieram os candidatos: "mastery" (seu pool) ou "tier" (tier list da lane).
    pub pool: String,
    pub suggestions: Vec<PickSuggestion>,
    /// Métrica dos pontos (`total`, `base`, `delta`).
    pub metric: Metric,
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

/// O que o laço do WebSocket leu da seleção atual. Uma trava só: quem lê vê tudo da mesma sessão.
#[derive(Default)]
struct Selection {
    me: Option<Me>,
    draft: Option<Draft>,
    /// Sua lane do lolalytics (posição atribuída), atualizada antes de recalcular bans e picks.
    my_lane: Option<String>,
    /// "campeão:lane" já importado nesta seleção (importa de novo só se trocar).
    auto_key: Option<String>,
    /// Maestria do jogador local `[(campeão, pontos)]`, lida uma vez por seleção (GET).
    mastery: Option<Vec<(i64, u64)>>,
}

pub struct Shared {
    pub lcu: RwLock<Option<Lcu>>,
    pub assets: RwLock<Option<Arc<Assets>>>,
    pub snapshot: Mutex<Snapshot>,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    /// Layout "classic" (o tamanho da janela depende dele): cópia das configurações para abrir a
    /// janela sem esperar a trava delas.
    pub classic_layout: AtomicBool,
    pub lola: Lolalytics,
    icons: Mutex<HashMap<String, (Vec<u8>, String)>>,
    /// `assets.json` e `icons/` (cópia do cliente para usar sem ele aberto).
    data_dir: PathBuf,
    /// Download dos ícones das runas da conexão atual (trava síncrona: nunca atravessa `.await`).
    prefetch: std::sync::Mutex<Option<AbortHandle>>,
    select: Mutex<Selection>,
    /// Geração do build em tela (`load_build`): respostas atrasadas são descartadas.
    build_gen: AtomicU64,
    /// Uma importação por vez: cliques rápidos não podem intercalar apagar/criar página.
    import_lock: Mutex<()>,
    /// Geração da conta de bans.
    bans_gen: AtomicU64,
    /// Geração da sugestão de pick e da sinergia.
    picks_gen: AtomicU64,
    /// Páginas de runas próprias (`rune-pages.json`, ao lado de `settings.json`).
    own_runes: Mutex<OwnPages>,
    own_runes_path: PathBuf,
    /// Idioma do sistema operacional (último recurso do idioma automático).
    system_locale: Option<String>,
    /// Estado do overlay na partida (o mesmo `Arc` fica no `manage` do Tauri).
    pub overlay: Arc<crate::overlay::OverlayState>,
}

impl Shared {
    pub fn new(settings_path: PathBuf, cache_dir: PathBuf) -> Self {
        let own_runes_path = settings_path.with_file_name("rune-pages.json");
        let data_dir = cache_dir
            .parent()
            .map_or_else(|| cache_dir.clone(), PathBuf::from);
        // Sem o cliente aberto, a janela usa a cópia da última conexão (nomes, runas, árvores).
        let cached = Assets::load_cached(&data_dir.join("assets.json")).map(Arc::new);
        let settings = Settings::load(&settings_path);
        let system_locale = i18n::system_locale();
        let locale = settings.locale(system_locale.as_deref());
        i18n::set_current(locale);
        Self {
            lcu: RwLock::new(None),
            assets: RwLock::new(cached),
            snapshot: Mutex::new(Snapshot {
                phase: "None".into(),
                locale: locale.into(),
                ..Default::default()
            }),
            classic_layout: AtomicBool::new(settings.is_classic()),
            settings: Mutex::new(settings),
            settings_path,
            lola: Lolalytics::new(cache_dir),
            icons: Mutex::new(HashMap::new()),
            data_dir,
            prefetch: std::sync::Mutex::default(),
            select: Mutex::default(),
            build_gen: AtomicU64::new(0),
            import_lock: Mutex::new(()),
            bans_gen: AtomicU64::new(0),
            picks_gen: AtomicU64::new(0),
            own_runes: Mutex::new(OwnPages::load(&own_runes_path)),
            own_runes_path,
            system_locale,
            overlay: Arc::default(),
        }
    }
}

/// Recalcula o idioma pelas configurações; se mudou, avisa a janela (que recarrega os textos),
/// troca o menu da bandeja e refaz as notas de bans e picks no idioma novo.
pub async fn apply_locale(app: &AppHandle, shared: &Arc<Shared>) {
    // Troca com as configurações travadas: duas chamadas juntas aplicam na ordem em que leram.
    let changed = {
        let s = shared.settings.lock().await;
        i18n::set_current(s.locale(shared.system_locale.as_deref()))
    };
    if !changed {
        return;
    }
    // O idioma em vigor agora (não o lido acima): a última escrita no snapshot é sempre o atual.
    update(app, shared, |s| s.locale = Lang::current().locale.into()).await;
    crate::refresh_tray(app).await;
    let (app, shared) = (app.clone(), shared.clone());
    tauri::async_runtime::spawn(async move {
        refresh_bans(&app, &shared).await;
        refresh_picks(&app, &shared).await;
    });
}

/// Idioma do cliente (só GET): guarda nas configurações para abrir no mesmo idioma sem ele.
/// `RegionLocale {locale, region, webLanguage, webRegion}` (schema do LCU); usamos `locale`.
async fn detect_locale(app: &AppHandle, shared: &Arc<Shared>, client: &Lcu) {
    let Ok(v) = client.get("/riotclient/region-locale").await else {
        return;
    };
    let Some(locale) = v
        .get("locale")
        .and_then(Value::as_str)
        .and_then(|l| i18n::supported(l).or_else(|| i18n::from_tag(l)))
    else {
        return;
    };
    {
        let mut s = shared.settings.lock().await;
        if s.client_locale == locale {
            return;
        }
        s.client_locale = locale.into();
        let _ = s.save(&shared.settings_path);
    }
    apply_locale(app, shared).await;
}

/// Muda o snapshot e manda para a janela **com a trava segura**: os eventos chegam na ordem em
/// que o estado mudou (soltar antes de mandar deixava um snapshot velho chegar por último) e
/// vão por referência, sem copiar o snapshot.
pub(crate) async fn update<R>(
    app: &AppHandle,
    shared: &Shared,
    f: impl FnOnce(&mut Snapshot) -> R,
) -> R {
    let mut s = shared.snapshot.lock().await;
    let r = f(&mut s);
    let _ = app.emit_to(crate::MAIN, "snapshot", &*s);
    r
}

/// Como [`update`], mas só se `mine` ainda é a geração atual de `generation`. A conferência é
/// feita com o snapshot travado: uma conta nova sobe o contador antes de escrever, então escreve
/// depois desta; e uma conta velha nunca escreve por cima da nova.
async fn update_if<R>(
    app: &AppHandle,
    shared: &Shared,
    generation: &AtomicU64,
    mine: u64,
    f: impl FnOnce(&mut Snapshot) -> R,
) -> Option<R> {
    let mut s = shared.snapshot.lock().await;
    if generation.load(Ordering::SeqCst) != mine {
        return None;
    }
    let r = f(&mut s);
    let _ = app.emit_to(crate::MAIN, "snapshot", &*s);
    Some(r)
}

/// Começa uma geração nova (as contas em andamento da anterior deixam de escrever).
fn next_gen(generation: &AtomicU64) -> u64 {
    generation.fetch_add(1, Ordering::SeqCst) + 1
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
        if client.get("/lol-gameflow/v1/gameflow-phase").await.is_err() {
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        }
        match Assets::load(&client).await {
            Ok(a) => {
                let a = Arc::new(a);
                *shared.assets.write().await = Some(a.clone());
                // Cópia para usar sem o cliente e os ícones das runas, em segundo plano (o laço
                // segue para a fase e a sessão). Reconectou no meio da anterior: a velha para
                // (as duas baixariam os mesmos ícones).
                let task = {
                    let shared = shared.clone();
                    tokio::spawn(async move {
                        let _ = a.save_cached(&shared.data_dir.join("assets.json"));
                        prefetch_rune_icons(&shared, &a).await
                    })
                };
                let old = shared
                    .prefetch
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .replace(task.abort_handle());
                if let Some(old) = old {
                    old.abort();
                }
            }
            Err(e) => {
                update(&app, &shared, |s| {
                    s.error = Some(crate::t!("err.clientData", error = e))
                })
                .await;
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        }
        *shared.lcu.write().await = Some(client.clone());
        detect_locale(&app, &shared, &client).await;
        update(&app, &shared, |s| {
            s.connected = true;
            s.error = None;
        })
        .await;
        // Assina antes de ler a fase e a sessão: o que mudar depois da leitura chega como evento
        // (antes, uma troca de fase durante o carregamento dos dados do cliente se perdia).
        let (tx, mut rx) = mpsc::channel(64);
        let Ok(events) = ws::subscribe(&creds, tx).await else {
            disconnect(&app, &shared).await;
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        };
        let ws_task = tokio::spawn(events);
        let phase = match client.get("/lol-gameflow/v1/gameflow-phase").await {
            Ok(Value::String(p)) => p,
            _ => "None".into(),
        };
        handle_phase(&app, &shared, &phase).await;
        if phase == "ChampSelect"
            && let Ok(sess) = client.get("/lol-champ-select/v1/session").await
        {
            handle_session(&app, &shared, Some(sess)).await;
        }
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
    {
        let mut sel = shared.select.lock().await;
        sel.me = None;
        sel.draft = None;
    }
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
    // Diagnóstico no terminal (`npm run tauri dev`).
    eprintln!("[fase] {phase}");
    match phase {
        "ChampSelect" => {
            *shared.select.lock().await = Selection::default();
            // Overlay: em tela cheia exclusiva ele não aparece. O jogo ainda está fechado aqui.
            let overlay_on = shared.settings.lock().await.overlay.enabled;
            let switched = overlay_on && crate::overlay::ensure_borderless();
            update(app, shared, |s| {
                // Seleção nova: o confronto, os bans e as buscas em andamento (do lobby ou da
                // seleção anterior) não valem mais; nada delas escreve nem importa daqui em diante.
                for g in [&shared.build_gen, &shared.bans_gen, &shared.picks_gen] {
                    next_gen(g);
                }
                s.phase = phase.into();
                s.in_champ_select = true;
                s.borderless_switched = switched;
                s.loading = false;
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
            if shared.settings.lock().await.close_in_game {
                crate::close_main(app);
            }
            // Overlay: o laço espera a API da partida responder e termina com a partida.
            let (app, shared) = (app.clone(), shared.clone());
            tauri::async_runtime::spawn(async move { crate::overlay::run(app, shared).await });
        }
        _ => update(app, shared, |s| s.phase = phase.into()).await,
    }
}

async fn handle_session(app: &AppHandle, shared: &Arc<Shared>, session: Option<Value>) {
    let Some((session, me)) = session
        .as_ref()
        .and_then(|s| Some((s, champselect::parse(s)?)))
    else {
        {
            let mut sel = shared.select.lock().await;
            sel.me = None;
            sel.auto_key = None;
            sel.draft = None;
        }
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
    let draft = champselect::parse_draft(session);
    // Vez de banir: só avisa a janela quando muda (a sessão chega a cada segundo).
    let can_ban = champselect::my_ban_action(session).is_some();
    // Overlay: você na partida é achado pelo campeão (o app não lê nomes).
    if me.current_champion() > 0 {
        let alias = champion_alias(shared, me.current_champion()).await;
        shared.overlay.set_champion(alias);
    }
    let lane = me
        .position
        .as_deref()
        .and_then(champselect::lolalytics_lane);
    // Tudo numa trava: a sessão muda a cada segundo (timer) e só reagimos quando o que importa
    // muda (o time visível, a sua posição, o seu campeão).
    let (draft_view, me_changed, start) = {
        let mut sel = shared.select.lock().await;
        let draft_view = if sel.draft != draft || sel.my_lane.as_deref() != lane {
            let view = draft.as_ref().map(|d| {
                (
                    d.allies.iter().map(|a| a.champion_id).collect::<Vec<_>>(),
                    d.enemies.clone(),
                    d.banning,
                )
            });
            sel.draft = draft;
            sel.my_lane = lane.map(String::from);
            Some(view.unwrap_or_default())
        } else {
            None
        };
        let me_changed = sel.me.as_ref() != Some(&me);
        // Travou (ou trocou de campeão/lane depois de travar): busca e importa uma vez.
        let mut start = None;
        if me_changed {
            if me.locked && me.champion_id > 0 {
                let key = format!("{}:{}", me.champion_id, lane.unwrap_or("default"));
                if sel.auto_key.as_deref() != Some(key.as_str()) {
                    sel.auto_key = Some(key);
                    start = Some(next_gen(&shared.build_gen));
                }
            }
            sel.me = Some(me.clone());
        }
        (draft_view, me_changed, start)
    };
    let ban_changed = shared.snapshot.lock().await.can_ban != can_ban;
    if draft_view.is_none() && !me_changed && !ban_changed {
        return;
    }
    let refresh = draft_view.is_some();
    let shown = me.current_champion();
    update(app, shared, |s| {
        s.can_ban = can_ban;
        if let Some((allies, enemies, banning)) = draft_view {
            s.in_champ_select = true;
            s.banning = banning;
            s.enemy_picks = enemies;
            s.ally_picks = allies;
        }
        if me_changed {
            s.in_champ_select = true;
            s.locked = me.locked;
            if start.is_none() && s.build.is_none() && shown > 0 {
                s.champion_id = shown;
            }
        }
    })
    .await;
    if refresh {
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move {
            refresh_bans(&app, &shared).await;
            refresh_picks(&app, &shared).await;
        });
    }
    if let Some(generation) = start {
        let auto = shared.settings.lock().await.auto_import;
        let (app, shared) = (app.clone(), shared.clone());
        let lane = lane.map(String::from);
        tauri::async_runtime::spawn(async move {
            load_build(&app, &shared, me.champion_id, lane, auto, generation).await;
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
    generation: u64,
) {
    let current = &shared.build_gen;
    // O confronto vem do snapshot, lido junto com o início da busca.
    let Some(vs) = update_if(app, shared, current, generation, |s| {
        s.champion_id = champion_id;
        s.lane.clone_from(&lane);
        s.loading = true;
        s.error = None;
        s.warning = None;
        s.import = None;
        (s.vs_champion_id > 0).then(|| (s.vs_champion_id, s.vs_lane.clone()))
    })
    .await
    else {
        return; // outra busca já começou
    };
    let assets = shared.assets.read().await.clone();
    let Some(champ) = assets.as_ref().and_then(|a| a.champion(champion_id)) else {
        update_if(app, shared, current, generation, |s| {
            s.loading = false;
            // Sem o build antigo (de outro campeão) na tela, "Importar" não o manda com este nome.
            s.build = None;
            s.error = Some(crate::t!("err.unknownChampion"));
        })
        .await;
        return;
    };
    let settings = shared.settings.lock().await.clone();
    let vs_champ = vs.and_then(|(id, l)| {
        let c = assets.as_ref()?.champion(id)?;
        Some((lolalytics_slug(&c.alias), &c.name, l))
    });
    let mut q = query(
        &settings,
        Ep::BuildFull,
        lolalytics_slug(&champ.alias),
        lane,
        None,
    );
    let result = match vs_champ {
        Some((slug, vs_name, vs_lane)) if slug != q.slug => {
            let vq = Query {
                // Sem lane escolhida: o oponente na mesma lane que você.
                vs_lane: vs_lane.or_else(|| q.lane.clone()),
                vs: Some(slug),
                ..q.clone()
            };
            match fetch_build(shared, &vq).await {
                Ok((b, w)) if b.n > 0 => {
                    q = vq;
                    Ok((b, w))
                }
                _ => {
                    let lane = vq
                        .vs_lane
                        .as_deref()
                        .map(|l| champselect::lane_label(Lang::current(), l))
                        .filter(|l| !l.is_empty());
                    let msg = match lane {
                        Some(lane) => crate::t!(
                            "warn.noVsGamesLane",
                            champion = champ.name,
                            vs = vs_name,
                            lane = lane
                        ),
                        None => crate::t!("warn.noVsGames", champion = champ.name, vs = vs_name),
                    };
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
    let ok = result.is_ok();
    let shown = match result {
        Err(e) => {
            update_if(app, shared, current, generation, |s| {
                s.loading = false;
                s.build = None;
                s.error = Some(e);
            })
            .await
        }
        Ok((mut build, warning)) => {
            build.slug.clone_from(&q.slug);
            if !build.lane.is_empty() {
                q.lane = Some(build.lane.clone());
            }
            let url = q.page_url();
            update_if(app, shared, current, generation, |s| {
                s.loading = false;
                s.lane = q.lane;
                s.page_url = Some(url);
                s.warning = warning;
                s.build = Some(Arc::new(build));
            })
            .await
        }
    };
    if shown.is_none() {
        return; // trocou de campeão enquanto buscava
    }
    if ok && do_import {
        // Só o build desta busca: se outra começou enquanto esperava a vez, ela decide o dela.
        let still = |_: &Snapshot| current.load(Ordering::SeqCst) == generation;
        match run_import_scoped(shared, Scope::ALL, still).await {
            Ok(None) => {}
            Ok(Some(r)) => {
                update_if(app, shared, current, generation, |s| s.import = Some(r)).await;
            }
            Err(e) => {
                update_if(app, shared, current, generation, |s| s.warning = Some(e)).await;
            }
        }
    }
    // Fora da seleção, os bans são os do campeão em tela; na seleção, vêm do time (`handle_session`).
    if ok && !shared.snapshot.lock().await.in_champ_select {
        refresh_bans(app, shared).await;
    }
}

async fn fetch_build(shared: &Shared, q: &Query) -> Result<(Build, Option<String>), String> {
    shared
        .lola
        .get(q)
        .await
        .and_then(|f| model::parse_build_full(&f.data).map(|b| (b, f.stale_warning)))
}

/// Recalcula a sugestão de bans: na seleção, pelos campeões do seu time (escolhidos ou
/// declarados); fora dela, pelo campeão em tela. Um build geral por campeão (cache de 6 h).
pub async fn refresh_bans(app: &AppHandle, shared: &Arc<Shared>) {
    let current = &shared.bans_gen;
    let generation = next_gen(current);
    let settings = shared.settings.lock().await.clone();
    if !settings.suggest_bans {
        update_if(app, shared, current, generation, |s| s.bans = None).await;
        return;
    }
    let (in_select, champion_id, lane) = {
        let s = shared.snapshot.lock().await;
        (s.in_champ_select, s.champion_id, s.lane.clone())
    };
    let (allies, unavailable, my_lane) = {
        let sel = shared.select.lock().await;
        let (allies, unavailable) = match &sel.draft {
            Some(d) if in_select => (d.allies.clone(), d.unavailable.clone()),
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
        (allies, unavailable, sel.my_lane.clone())
    };
    if let Some(lane) = my_lane.filter(|_| in_select && allies.is_empty()) {
        bans_by_tier(app, shared, generation, &settings, lane, &unavailable).await;
        return;
    }
    if allies.is_empty() {
        update_if(app, shared, current, generation, |s| {
            s.bans = in_select.then(|| BanAdvice {
                note: Some(crate::t!("bans.declareAny")),
                ..Default::default()
            })
        })
        .await;
        return;
    }
    update_if(app, shared, current, generation, |s| {
        // Os da tier list não valem para a conta pelo time.
        let (picks, lane_picks, lane_for, metric) = s
            .bans
            .take()
            .filter(|b| !b.by_tier)
            .map(|b| (b.picks, b.lane_picks, b.lane_for, b.metric))
            .unwrap_or_default();
        s.bans = Some(BanAdvice {
            loading: true,
            based_on: allies.clone(),
            picks,
            lane_picks,
            lane_for,
            metric,
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
        let q = query(
            &settings,
            Ep::BuildFull,
            lolalytics_slug(&c.alias),
            ally.lane.clone(),
            None,
        );
        match fetch_build(shared, &q).await {
            Ok((b, _)) if !b.matchups.is_empty() => builds.push((ally.clone(), b)),
            _ => missing.push(c.name.clone()),
        }
        if current.load(Ordering::SeqCst) != generation {
            return; // o time mudou enquanto buscava; a conta nova assume
        }
    }
    let inputs: Vec<(Ally, &Build)> = builds.iter().map(|(a, b)| (a.clone(), b)).collect();
    let metric = Metric::from_setting(&settings.metric);
    let picks = bans::suggest(&inputs, &unavailable, 5, metric);
    // Lista individual: só o seu campeão, oponentes da sua lane, sem repetir a do time.
    let in_team: Vec<i64> = picks.iter().map(|p| p.champion_id).collect();
    let mine = builds.iter().find(|(a, _)| a.me);
    let lane_picks = mine
        .map(|(me, b)| bans::lane_suggest(me, b, &unavailable, &in_team, 5, metric))
        .unwrap_or_default();
    let lane_for = mine.map(|(me, b)| Ally {
        lane: Some(b.lane.clone())
            .filter(|l| !l.is_empty())
            .or(me.lane.clone()),
        ..me.clone()
    });
    let lane_note = if mine.is_none() {
        Some(declare_note())
    } else if lane_picks.is_empty() {
        Some(crate::t!("bans.noLaneOpponent"))
    } else {
        None
    };
    let note = (!missing.is_empty()).then(|| crate::t!("bans.noData", names = missing.join(", ")));
    update_if(app, shared, current, generation, |s| {
        s.bans = Some(BanAdvice {
            loading: false,
            based_on: allies,
            picks,
            note,
            lane_picks,
            lane_for,
            lane_note,
            metric,
            ..Default::default()
        })
    })
    .await;
}

/// Consulta ao lolalytics com os filtros das configurações (elo, região, patch).
fn query(
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

/// `list`/`counter`/`build-team` (cache de 6 h) já lidos pelo parser do endpoint.
async fn fetch<T>(
    shared: &Shared,
    q: &Query,
    parse: impl FnOnce(&Value) -> Result<T, String>,
) -> Result<T, String> {
    shared.lola.get(q).await.and_then(|f| parse(&f.data))
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
    let current = &shared.bans_gen;
    update_if(app, shared, current, generation, |s| {
        let (picks, metric) = s
            .bans
            .take()
            .filter(|b| b.by_tier)
            .map(|b| (b.picks, b.metric))
            .unwrap_or_default();
        s.bans = Some(BanAdvice {
            loading: true,
            picks,
            metric,
            by_tier: true,
            lane: Some(lane.clone()),
            ..Default::default()
        });
    })
    .await;
    let q = query(settings, Ep::List, String::new(), Some(lane.clone()), None);
    let metric = Metric::from_setting(&settings.metric);
    let result = fetch(shared, &q, extra::parse_list).await;
    let advice = match result {
        // Ninguém declarou, nem você: a lista individual espera o seu campeão.
        Ok(list) => BanAdvice {
            picks: picks::tier_bans(&list, unavailable, 5, metric),
            metric,
            by_tier: true,
            lane: Some(lane),
            page_url: Some(q.page_url()),
            lane_note: Some(declare_note()),
            ..Default::default()
        },
        Err(e) => BanAdvice {
            note: Some(crate::t!("tierList.unavailable", error = e)),
            by_tier: true,
            lane: Some(lane),
            lane_note: Some(declare_note()),
            ..Default::default()
        },
    };
    update_if(app, shared, current, generation, |s| s.bans = Some(advice)).await;
}

/// Maestria do jogador local (só GET; uma vez por seleção).
async fn mastery(shared: &Shared) -> Result<Vec<(i64, u64)>, String> {
    if let Some(m) = shared.select.lock().await.mastery.clone() {
        return Ok(m);
    }
    let lcu = shared
        .lcu
        .read()
        .await
        .clone()
        .ok_or_else(|| crate::t!("err.clientClosed"))?;
    let v = lcu
        .get("/lol-champion-mastery/v1/local-player/champion-mastery")
        .await
        .map_err(|e| e.to_string())?;
    let m = extra::parse_mastery(&v);
    shared.select.lock().await.mastery = Some(m.clone());
    Ok(m)
}

/// Recalcula a sugestão de pick e a sinergia quando a seleção muda. Requisições ao lolalytics
/// (cache de 6 h): a tier list da sua lane, o `build-team` do seu campeão (sinergia), um
/// `counter` por inimigo visível (até 5) e um `build-team` por aliado com lane (até 4).
pub async fn refresh_picks(app: &AppHandle, shared: &Arc<Shared>) {
    let current = &shared.picks_gen;
    let generation = next_gen(current);
    // Para de buscar assim que uma conta mais nova começa (as escritas também conferem).
    let stale = || current.load(Ordering::SeqCst) != generation;
    let settings = shared.settings.lock().await.clone();
    let in_select = shared.snapshot.lock().await.in_champ_select;
    if !settings.suggest_picks || !in_select {
        update_if(app, shared, current, generation, |s| {
            s.picks = None;
            s.synergy = None;
        })
        .await;
        return;
    }
    let (lane, draft) = {
        let sel = shared.select.lock().await;
        (sel.my_lane.clone(), sel.draft.clone().unwrap_or_default())
    };
    let Some(lane) = lane else {
        update_if(app, shared, current, generation, |s| {
            s.picks = Some(PickAdvice {
                note: Some(crate::t!("picks.noPosition")),
                ..Default::default()
            });
            s.synergy = None;
        })
        .await;
        return;
    };
    let mates: Vec<Ally> = draft
        .allies
        .iter()
        .filter(|a| !a.me && a.lane.is_some())
        .take(4)
        .cloned()
        .collect();
    let enemies: Vec<i64> = draft.enemies.iter().take(5).copied().collect();
    update_if(app, shared, current, generation, |s| {
        let (prev, metric) = s
            .picks
            .take()
            .map(|p| (p.suggestions, p.metric))
            .unwrap_or_default();
        s.picks = Some(PickAdvice {
            loading: true,
            lane: Some(lane.clone()),
            pool: settings.pick_pool.clone(),
            suggestions: prev,
            metric,
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
    let list_q = query(&settings, Ep::List, String::new(), Some(lane.clone()), None);
    let list = match fetch(shared, &list_q, extra::parse_list).await {
        Ok(l) => l,
        Err(e) => {
            update_if(app, shared, current, generation, |s| {
                if let Some(p) = s.picks.as_mut() {
                    p.loading = false;
                    p.note = Some(crate::t!("tierList.unavailable", error = e));
                }
            })
            .await;
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
            let q = query(&settings, Ep::BuildTeam, my_slug, Some(lane.clone()), None);
            let team = fetch(shared, &q, extra::parse_build_team).await;
            team.ok().map(|team| SynergyAdvice {
                champion_id: id,
                lane: lane.clone(),
                base_wr: list.get(id).map(|e| e.wr),
                lines: picks::synergy_lines(&team, &mates),
                page_url: Some(q.page_url()),
            })
        }
        _ => None,
    };
    if update_if(app, shared, current, generation, |s| s.synergy = synergy)
        .await
        .is_none()
    {
        return;
    }

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
            Err(e) => notes.push(crate::t!("picks.masteryUnavailable", error = e)),
        }
        if candidates.is_empty() {
            notes.push(crate::t!("picks.poolEmpty"));
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
        let q = query(
            &settings,
            Ep::Counter,
            e_slug,
            Some(e_lane.clone()),
            Some(lane.clone()),
        );
        match fetch(shared, &q, extra::parse_counter).await {
            Ok(c) => counters.push((e, e_lane, c)),
            Err(_) => notes.push(crate::t!("picks.noCounter", name = name)),
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
        let q = query(&settings, Ep::BuildTeam, a_slug, Some(a_lane.clone()), None);
        match fetch(shared, &q, extra::parse_build_team).await {
            Ok(t) => teams.push((a.champion_id, a_lane, t)),
            Err(_) => notes.push(crate::t!("picks.noSynergy", name = name)),
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
    let metric = Metric::from_setting(&settings.metric);
    let suggestions = picks::suggest(
        &lane,
        &list,
        &candidates,
        &enemy_inputs,
        &mate_inputs,
        &owned,
        picks::TOP,
        metric,
    );
    update_if(app, shared, current, generation, |s| {
        s.picks = Some(PickAdvice {
            loading: false,
            lane: Some(lane),
            pool: pool_kind,
            suggestions,
            metric,
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
    // O confronto mora só no snapshot (a busca lê de lá): escolher e ler o campeão em tela
    // acontecem juntos, sem uma cópia à parte que pudesse ficar fora de ordem.
    let (champion_id, lane) = update(app, shared, |s| {
        s.vs_champion_id = vs.unwrap_or(0);
        s.vs_lane = vs.and(vs_lane);
        (s.champion_id, s.lane.clone())
    })
    .await;
    if champion_id > 0 {
        lookup(app, shared, champion_id, lane).await;
    }
}

/// Botão "Importar": manda o build em tela para o cliente, conforme as configurações.
pub async fn reimport(app: &AppHandle, shared: &Shared) -> Result<ImportReport, String> {
    let report = run_import_scoped(shared, Scope::ALL, |_| true)
        .await?
        .ok_or_else(|| crate::t!("import.noBuild"))?;
    update(app, shared, |s| s.import = Some(report.clone())).await;
    Ok(report)
}

/// A janela pode mandar sozinha o que está na tela para o cliente agora?
async fn should_auto_apply(shared: &Shared, champion_id: i64) -> bool {
    // Sem o cliente (só a janela aberta, com a cópia dos dados): nada a importar.
    if !shared.settings.lock().await.auto_import || shared.lcu.read().await.is_none() {
        return false;
    }
    let (phase, in_select) = {
        let s = shared.snapshot.lock().await;
        (s.phase.clone(), s.in_champ_select)
    };
    let sel = shared.select.lock().await;
    champselect::may_apply(&phase, in_select, sel.me.as_ref(), champion_id)
}

/// Algo mudou na janela (runa, amostra mínima): leva a mudança para o cliente.
pub async fn apply_change(app: &AppHandle, shared: &Arc<Shared>, scope: Scope) {
    // Carregando: a busca em andamento importa (ou não) com as configurações novas.
    let viewed = {
        let s = shared.snapshot.lock().await;
        (s.build.is_some() && !s.loading).then_some(s.champion_id)
    };
    let Some(champion_id) = viewed else {
        return;
    };
    if !should_auto_apply(shared, champion_id).await {
        return;
    }
    // Importa só se a tela ainda mostra o campeão conferido acima (o de outro não vai).
    match run_import_scoped(shared, scope, |s| s.champion_id == champion_id).await {
        Ok(None) => {}
        Ok(Some(r)) => {
            update(app, shared, |s| {
                s.import.get_or_insert_with(Default::default).merge(r)
            })
            .await
        }
        Err(e) => update(app, shared, |s| s.warning = Some(e)).await,
    }
}

/// Alias do campeão no cliente (o mesmo do `rawChampionName` da API da partida).
async fn champion_alias(shared: &Shared, champion_id: i64) -> Option<String> {
    shared
        .assets
        .read()
        .await
        .as_ref()
        .and_then(|a| a.champion(champion_id).map(|c| c.alias.clone()))
}

/// Guarda para o overlay a ordem de compra do build importado (lembrete do próximo item).
fn overlay_path(shared: &Shared, build: &Build, champion_id: i64, assets: &Assets, min: u64) {
    let alias = assets.champion(champion_id).map(|c| c.alias.clone());
    shared.overlay.set_path(
        alias,
        itemset::build_path(build, assets, build.min_games(min)),
    );
}

/// Importa o build em tela. `still` confere, com o snapshot travado e já na vez desta importação,
/// se ela ainda vale; senão (ou com o build ainda carregando) devolve `Ok(None)` sem mexer em nada.
/// O build e o campeão são lidos juntos: nunca vai o build de um com o nome de outro.
async fn run_import_scoped(
    shared: &Shared,
    scope: Scope,
    still: impl FnOnce(&Snapshot) -> bool,
) -> Result<Option<ImportReport>, String> {
    let _one_at_a_time = shared.import_lock.lock().await;
    let lang = Lang::current();
    let (build, champion_id) = {
        let s = shared.snapshot.lock().await;
        if s.loading || !still(&s) {
            return Ok(None);
        }
        (
            s.build.clone().ok_or_else(|| lang.t("import.noBuild"))?,
            s.champion_id,
        )
    };
    let lcu = shared
        .lcu
        .read()
        .await
        .clone()
        .ok_or_else(|| lang.t("err.clientClosed"))?;
    let assets = shared
        .assets
        .read()
        .await
        .clone()
        .ok_or_else(|| lang.t("import.assetsNotLoaded"))?;
    let settings = shared.settings.lock().await.clone();
    // Os dados são de ranqueada no Summoner's Rift: em outros mapas não importamos nada.
    if let Ok(gs) = lcu.get("/lol-gameflow/v1/session").await {
        let map = gs
            .pointer("/map/id")
            .or_else(|| gs.pointer("/gameData/queue/mapId"))
            .and_then(Value::as_i64);
        if map.is_some_and(|m| m != 11) {
            return Err(lang.t("import.notRift"));
        }
    }
    let name = assets
        .champion(champion_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    // Nomes que vão para o cliente (página de runas, conjunto de itens) no idioma do app;
    // os prefixos `LC:`/`lc-` não mudam: é por eles que achamos o que é nosso.
    let lane_label = champselect::lane_label(lang, &build.lane);
    // Build de confronto: "vs Viego" no nome da página e no título do conjunto.
    let vs_name = build
        .vs
        .as_ref()
        .and_then(|v| assets.champion(v.champion_id))
        .map(|c| c.name.clone());
    let set_label = match &vs_name {
        Some(v) => lang.tf("itemset.vs", &[("lane", &lane_label), ("champion", v)]),
        None => lane_label.clone(),
    };
    // O nome da página é curto (o cliente corta): com confronto, sai a lane.
    let page_label = match &vs_name {
        Some(v) => lang.tf("runes.vs", &[("champion", v)]),
        None => lane_label.clone(),
    };
    let mut rep = ImportReport::default();

    if scope.runes && settings.import_runes {
        // A página escolhida do campeão vale em qualquer lane e confronto.
        let own = shared.own_runes.lock().await.active(champion_id).cloned();
        let body = match own_runes::choose(
            &settings.rune_choice,
            build.runes_pick.as_ref(),
            build.runes_win.as_ref(),
            own.as_ref(),
        ) {
            Chosen::None => Err(lang.t("import.noRunes")),
            Chosen::Lola(set) => {
                runes::page_body(&runes::page_name(&name, &page_label), set, &assets)
            }
            // A sua página: o nome dela vai no lugar da lane ("LC: Ahri Full AP").
            Chosen::Own(page) => {
                runes::own_page_body(&runes::page_name(&name, &page.name), page, &assets)
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
            lang,
            Metric::from_setting(&settings.metric),
        );
        rep.items = Some(Outcome::from(itemset::apply(&lcu, set).await));
        overlay_path(shared, &build, champion_id, &assets, settings.min_games);
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
            Some(_) if rec.len() != 2 => Some(Outcome::fail(lang.t("import.noSpells"))),
            Some(me) => {
                let pair = spells::arrange(
                    (rec[0], rec[1]),
                    (me.spell1, me.spell2),
                    settings.flash_slot,
                );
                Some(if pair == (me.spell1, me.spell2) {
                    Outcome::ok(lang.t("import.spellsAlready"))
                } else {
                    Outcome::from(spells::apply(&lcu, pair).await)
                })
            }
        };
    }
    Ok(Some(rep))
}

/// Arquivo do ícone na cópia em disco (nome só com letras, dígitos, `.`, `-` e `_`).
fn icon_file(shared: &Shared, path: &str) -> PathBuf {
    let name: String = path
        .trim_start_matches("/lol-game-data/assets/")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    shared.data_dir.join("icons").join(name)
}

fn content_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or_default();
    let is = |e: &str| ext.eq_ignore_ascii_case(e);
    if is("png") {
        "image/png"
    } else if is("jpg") || is("jpeg") {
        "image/jpeg"
    } else if is("svg") {
        "image/svg+xml"
    } else if is("webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

/// Baixa do cliente os ícones de runas e árvores que ainda não estão no disco (um por vez).
async fn prefetch_rune_icons(shared: &Shared, assets: &Assets) {
    for icon in assets.rune_icons() {
        if !icon_file(shared, &icon).exists() && fetch_icon(shared, &icon).await.is_err() {
            return; // cliente fechou no meio: o resto fica para a próxima conexão
        }
    }
}

/// Ícones do cliente para a janela (`http://lcu.localhost/lol-game-data/assets/...`).
/// Cada ícone baixado fica também no disco, para a janela sem o cliente aberto.
pub async fn fetch_icon(shared: &Shared, path: &str) -> Result<(Vec<u8>, String), String> {
    if !path.starts_with("/lol-game-data/assets/") || path.contains("..") {
        return Err("caminho não permitido".into());
    }
    if let Some(hit) = shared.icons.lock().await.get(path) {
        return Ok(hit.clone());
    }
    let file = icon_file(shared, path);
    let lcu = shared.lcu.read().await.clone();
    let got = match lcu {
        Some(lcu) => {
            let got = lcu.get_bytes(path).await.map_err(|e| e.to_string())?;
            let _ = crate::assets::write_atomic(&file, &got.0);
            got
        }
        None => {
            let bytes = std::fs::read(&file).map_err(|_| "cliente fechado".to_string())?;
            (bytes, content_type(path).to_string())
        }
    };
    let mut cache = shared.icons.lock().await;
    if cache.len() > 600 {
        cache.clear();
    }
    cache.insert(path.to_string(), got.clone());
    Ok(got)
}

/// Troca de campeão, lane ou filtro na janela: carrega e, quando pode, já leva para o cliente.
pub async fn lookup(app: &AppHandle, shared: &Arc<Shared>, champion_id: i64, lane: Option<String>) {
    // A última escolhida vence: as buscas anteriores param de escrever e de importar.
    let generation = next_gen(&shared.build_gen);
    let apply = should_auto_apply(shared, champion_id).await;
    load_build(app, shared, champion_id, lane, apply, generation).await;
}

/// Campeão do build em tela (as páginas próprias são por campeão).
async fn view_champion(shared: &Shared) -> Option<i64> {
    let s = shared.snapshot.lock().await;
    (s.build.is_some() && s.champion_id > 0).then_some(s.champion_id)
}

/// Grava `next` e só então troca as páginas da memória (se a gravação falhar, memória e arquivo
/// continuam iguais); devolve a lista para a janela.
fn store_own(
    shared: &Shared,
    pages: &mut OwnPages,
    next: OwnPages,
) -> Result<Vec<OwnEntry>, String> {
    next.save(&shared.own_runes_path)
        .map_err(|e| crate::t!("own.saveFailed", error = e))?;
    *pages = next;
    Ok(pages.entries())
}

pub async fn own_pages(shared: &Shared) -> Vec<OwnEntry> {
    shared.own_runes.lock().await.entries()
}

/// Salva a página e a deixa escolhida; se for o campeão em tela, leva para o cliente
/// (quando a janela pode: `apply_change` confere cliente aberto e campeão do jogador).
async fn store_page(
    app: &AppHandle,
    shared: &Arc<Shared>,
    champion_id: i64,
    page_id: Option<u32>,
    page: OwnPage,
    import: bool,
) -> Result<Vec<OwnEntry>, String> {
    let list = {
        let mut pages = shared.own_runes.lock().await;
        let mut next = pages.clone();
        next.upsert(champion_id, page_id, page);
        store_own(shared, &mut pages, next)?
    };
    if import && view_champion(shared).await == Some(champion_id) {
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move { apply_change(&app, &shared, Scope::RUNES).await });
    }
    Ok(list)
}

/// Copia a página atual do cliente (só GET) como página sua do campeão: nova (`page_id` vazio,
/// com o nome que ela tem no cliente) ou por cima de uma sua.
pub async fn save_own_runes(
    app: &AppHandle,
    shared: &Arc<Shared>,
    champion_id: i64,
    page_id: Option<u32>,
    name: Option<String>,
) -> Result<Vec<OwnEntry>, String> {
    let lcu = shared
        .lcu
        .read()
        .await
        .clone()
        .ok_or_else(|| crate::t!("own.needClient"))?;
    let current = lcu
        .get("/lol-perks/v1/currentpage")
        .await
        .map_err(|e| e.to_string())?;
    let mut page = OwnPage::from_client(&current, own_runes::now_secs())?;
    if let Some(n) = name.as_deref().and_then(own_runes::clean_name) {
        page.name = n;
    }
    if let Some(a) = shared.assets.read().await.as_ref() {
        page.check(a)?;
    }
    // A página já está no cliente (é a atual): nada a importar, só o aviso.
    let list = store_page(app, shared, champion_id, page_id, page, false).await?;
    if view_champion(shared).await == Some(champion_id) {
        update(app, shared, |s| {
            s.import.get_or_insert_with(Default::default).runes =
                Some(Outcome::ok(crate::t!("own.savedCurrent")));
        })
        .await;
    }
    Ok(list)
}

/// Página montada na janela (funciona sem o cliente: só grava `rune-pages.json`).
#[allow(clippy::too_many_arguments)]
pub async fn save_own_runes_page(
    app: &AppHandle,
    shared: &Arc<Shared>,
    champion_id: i64,
    page_id: Option<u32>,
    name: String,
    primary_style: i64,
    sub_style: i64,
    perks: Vec<i64>,
) -> Result<Vec<OwnEntry>, String> {
    let mut page = OwnPage::from_editor(primary_style, sub_style, perks, own_runes::now_secs())?;
    page.name = own_runes::clean_name(&name).unwrap_or_default();
    let assets = shared
        .assets
        .read()
        .await
        .clone()
        .ok_or_else(|| crate::t!("own.noAssets"))?;
    page.check(&assets)?;
    store_page(app, shared, champion_id, page_id, page, true).await
}

/// Apaga uma página sua; se era a escolhida do campeão em tela, a importação volta ao lolalytics.
pub async fn delete_own_runes(
    app: &AppHandle,
    shared: &Arc<Shared>,
    champion_id: i64,
    page_id: u32,
) -> Result<Vec<OwnEntry>, String> {
    let (list, was_active) = {
        let mut pages = shared.own_runes.lock().await;
        let mut next = pages.clone();
        let old = next.remove(champion_id, page_id);
        (
            store_own(shared, &mut pages, next)?,
            old.is_some_and(|(_, was)| was),
        )
    };
    if was_active && view_champion(shared).await == Some(champion_id) {
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move { apply_change(&app, &shared, Scope::RUNES).await });
    }
    Ok(list)
}

/// Mais jogada / Maior WR / uma página sua (`page_id`) no campeão em tela. A página sua vale só
/// para esse campeão; Mais jogada e Maior WR também viram a escolha geral (`runeChoice`).
pub async fn choose_runes(
    app: &AppHandle,
    shared: &Arc<Shared>,
    choice: String,
    page_id: Option<u32>,
) -> Result<Vec<OwnEntry>, String> {
    if !matches!(choice.as_str(), "pick" | "win" | "own") {
        return Err(crate::t!("own.unknownChoice", choice = choice));
    }
    let champion = view_champion(shared).await;
    let mut changed;
    let list = {
        let mut pages = shared.own_runes.lock().await;
        let target = if choice == "own" { page_id } else { None };
        if let Some(id) = target
            && !champion.is_some_and(|c| pages.get(c, id).is_some())
        {
            return Err(crate::t!("own.pageGone"));
        }
        let mut next = pages.clone();
        changed = champion.is_some_and(|c| next.set_active(c, target));
        if changed {
            store_own(shared, &mut pages, next)?
        } else {
            pages.entries()
        }
    };
    if choice != "own" {
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
        .ok_or_else(|| crate::t!("err.clientClosed"))?;
    ban::apply(&lcu, champion_id).await
}
