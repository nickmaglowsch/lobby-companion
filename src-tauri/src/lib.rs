//! Lobby Companion: importa runas, feitiços e conjuntos de itens do lolalytics na seleção de campeões.
//! A janela abre na seleção e fecha quando a partida começa; na partida, só o overlay (`overlay.rs`)
//! e o ícone na bandeja.

pub mod app;
pub mod assets;
pub mod bans;
pub mod champselect;
pub mod composition;
pub mod hud;
pub mod i18n;
pub mod import;
pub mod lcu;
pub mod live;
pub mod lola;
pub mod metric;
pub mod overlay;
pub mod own_runes;
pub mod picks;
pub mod settings;
pub mod update;

use app::{ImportReport, Shared, Snapshot};
use settings::Settings;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};

const MAIN: &str = "main";

/// Tamanho da janela pelo layout escolhido. Lê a cópia atômica do layout: com `try_lock` nas
/// configurações, a janela abria no tamanho padrão quando outra tarefa as estava lendo.
fn window_size(app: &AppHandle) -> (f64, f64, f64, f64) {
    let classic = app
        .try_state::<Arc<Shared>>()
        .is_some_and(|s| s.classic_layout.load(Ordering::Relaxed));
    Settings::window_size_for(classic)
}

/// Troca o tamanho da janela aberta quando o layout muda.
fn apply_window_size(app: &AppHandle, size: (f64, f64, f64, f64)) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(MAIN) {
            let (width, height, min_w, min_h) = size;
            let _ = w.set_min_size(Some(tauri::LogicalSize::new(min_w, min_h)));
            let _ = w.set_size(tauri::LogicalSize::new(width, height));
        }
    });
}

/// Abre (ou traz para frente) a janela. Cria o WebView só quando precisa.
pub fn show_main(app: &AppHandle) {
    let handle = app.clone();
    let (width, height, min_w, min_h) = window_size(app);
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(MAIN) {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
            return;
        }
        let _ = WebviewWindowBuilder::new(&handle, MAIN, WebviewUrl::App("index.html".into()))
            .title("Lobby Companion")
            .inner_size(width, height)
            .min_inner_size(min_w, min_h)
            .build();
    });
}

/// Destrói a janela (libera o WebView); o app continua na bandeja ouvindo o cliente.
pub fn close_main(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(MAIN) {
            let _ = w.destroy();
        }
    });
}

/// Menu da bandeja; com versão nova, o primeiro item avisa (e abre a janela, onde fica o botão).
fn tray_menu(app: &AppHandle, update: Option<&str>) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", t!("tray.open"), true, None::<&str>)?;
    let edit = MenuItem::with_id(
        app,
        "overlay_edit",
        t!("tray.overlayEdit"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", t!("tray.quit"), true, None::<&str>)?;
    match update {
        Some(v) => {
            let label = t!("tray.update", version = v.trim_start_matches('v'));
            let up = MenuItem::with_id(app, "update", label, true, None::<&str>)?;
            Menu::with_items(app, &[&up, &open, &edit, &quit])
        }
        None => Menu::with_items(app, &[&open, &edit, &quit]),
    }
}

/// Mostra (ou tira) o aviso de versão nova no menu e na dica da bandeja.
pub fn set_tray_update(app: &AppHandle, version: Option<&str>) {
    let Some(tray) = app.tray_by_id("tray") else {
        return;
    };
    if let Ok(menu) = tray_menu(app, version) {
        let _ = tray.set_menu(Some(menu));
    }
    let tip = match version {
        Some(v) => t!("tray.tipUpdate", version = v.trim_start_matches('v')),
        None => "Lobby Companion".into(),
    };
    let _ = tray.set_tooltip(Some(tip));
}

/// Menu e dica da bandeja no idioma atual (depois de trocar o idioma).
pub async fn refresh_tray(app: &AppHandle) {
    let version = update::pending(app).await.map(|u| u.version);
    set_tray_update(app, version.as_deref());
}

fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = tray_menu(app, None)?;
    let mut tray = TrayIconBuilder::with_id("tray")
        .tooltip("Lobby Companion")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "open" | "update" => show_main(app),
            "overlay_edit" => {
                let on = app
                    .try_state::<Arc<overlay::OverlayState>>()
                    .is_some_and(|s| !s.editing());
                overlay::set_edit(app, on);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, ev| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = ev
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

#[tauri::command]
async fn get_snapshot(shared: State<'_, Arc<Shared>>) -> Result<Snapshot, String> {
    Ok(shared.snapshot.lock().await.clone())
}

#[tauri::command]
async fn get_assets(shared: State<'_, Arc<Shared>>) -> Result<Option<assets::Assets>, String> {
    Ok(shared.assets.read().await.as_deref().cloned())
}

#[tauri::command]
async fn get_settings(shared: State<'_, Arc<Shared>>) -> Result<Settings, String> {
    Ok(shared.settings.lock().await.clone())
}

#[tauri::command]
async fn save_settings(
    app: AppHandle,
    shared: State<'_, Arc<Shared>>,
    mut settings: Settings,
) -> Result<(), String> {
    let (filters_changed, runes_changed, items_changed, bans_changed, picks_changed) = {
        let mut cur = shared.settings.lock().await;
        // O locale do cliente é do backend (`detect_locale`): a cópia da janela pode estar velha.
        settings.client_locale.clone_from(&cur.client_locale);
        // Posições do overlay são salvas pela janela do overlay (`save_overlay_pos`).
        settings.overlay.cs_pos = cur.overlay.cs_pos;
        settings.overlay.item_pos = cur.overlay.item_pos;
        settings
            .save(&shared.settings_path)
            .map_err(|e| e.to_string())?;
        let changed = (
            cur.tier != settings.tier
                || cur.region != settings.region
                || cur.patch != settings.patch,
            cur.rune_choice != settings.rune_choice,
            cur.min_games != settings.min_games || cur.metric != settings.metric,
            cur.suggest_bans != settings.suggest_bans || cur.metric != settings.metric,
            cur.suggest_picks != settings.suggest_picks
                || cur.pick_pool != settings.pick_pool
                || cur.metric != settings.metric,
        );
        if cur.window_size() != settings.window_size() {
            apply_window_size(&app, settings.window_size());
        }
        shared
            .classic_layout
            .store(settings.is_classic(), Ordering::Relaxed);
        *cur = settings;
        changed
    };
    let (champion_id, lane, in_select) = {
        let s = shared.snapshot.lock().await;
        (s.champion_id, s.lane.clone(), s.in_champ_select)
    };
    let shared = shared.inner().clone();
    // Idioma: a janela recebe o locale novo no snapshot; bandeja e notas mudam junto.
    app::apply_locale(&app, &shared).await;
    if picks_changed || filters_changed {
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move { app::refresh_picks(&app, &shared).await });
    }
    // Os bans dependem do filtro. Fora da seleção, com campeão em tela, o lookup abaixo já recalcula.
    let lookup_redoes_bans = filters_changed && champion_id > 0 && !in_select;
    if bans_changed || (filters_changed && !lookup_redoes_bans) {
        let (app, shared) = (app.clone(), shared.clone());
        tauri::async_runtime::spawn(async move { app::refresh_bans(&app, &shared).await });
    }
    // Filtro mudou: recarrega o build em tela, que já vai para o cliente.
    if filters_changed && champion_id > 0 {
        tauri::async_runtime::spawn(
            async move { app::lookup(&app, &shared, champion_id, lane).await },
        );
    } else if runes_changed || items_changed {
        let scope = app::Scope {
            runes: runes_changed,
            items: items_changed,
            spells: false,
        };
        tauri::async_runtime::spawn(async move { app::apply_change(&app, &shared, scope).await });
    }
    Ok(())
}

#[tauri::command]
async fn lookup(
    app: AppHandle,
    shared: State<'_, Arc<Shared>>,
    champion_id: i64,
    lane: Option<String>,
) -> Result<(), String> {
    let shared = shared.inner().clone();
    tauri::async_runtime::spawn(async move { app::lookup(&app, &shared, champion_id, lane).await });
    Ok(())
}

/// Campeão do outro lado (`None` = build geral). Só muda o que a janela mostra e importa.
#[tauri::command]
async fn set_vs(
    app: AppHandle,
    shared: State<'_, Arc<Shared>>,
    vs_champion_id: Option<i64>,
    vs_lane: Option<String>,
) -> Result<(), String> {
    let shared = shared.inner().clone();
    tauri::async_runtime::spawn(async move {
        app::set_vs(&app, &shared, vs_champion_id, vs_lane).await
    });
    Ok(())
}

#[tauri::command]
async fn reimport(app: AppHandle, shared: State<'_, Arc<Shared>>) -> Result<ImportReport, String> {
    app::reimport(&app, &shared).await
}

/// Composição de dano dos dois times e itens defensivos do build em tela (só exibição).
#[tauri::command]
async fn get_composition(
    shared: State<'_, Arc<Shared>>,
    allies: Vec<i64>,
    enemies: Vec<i64>,
) -> Result<composition::Composition, String> {
    let lcu = shared.lcu.read().await.clone();
    let ids: Vec<i64> = allies.iter().chain(&enemies).copied().collect();
    let types = composition::damage_types(lcu.as_ref(), &ids).await;
    let assets = shared.assets.read().await.clone();
    let build = shared.snapshot.lock().await.build.clone();
    let configured = shared.settings.lock().await.min_games;
    let min_games = build
        .as_ref()
        .map_or(configured, |b| b.min_games(configured));
    Ok(composition::summarize(
        &allies,
        &enemies,
        &types,
        build.as_deref(),
        assets.as_deref(),
        min_games,
        i18n::Lang::current(),
    ))
}

/// Clique numa sugestão de ban na sua vez: marca e trava o ban no cliente.
#[tauri::command]
async fn ban_champion(shared: State<'_, Arc<Shared>>, champion_id: i64) -> Result<(), String> {
    app::ban_champion(&shared, champion_id).await
}

/// Páginas de runas próprias (todas, para a janela e as configurações).
#[tauri::command]
async fn get_own_pages(shared: State<'_, Arc<Shared>>) -> Result<Vec<own_runes::OwnEntry>, String> {
    Ok(app::own_pages(&shared).await)
}

/// Copia a página atual do cliente (GET) como página sua do campeão (nova ou por cima de `page_id`).
#[tauri::command]
async fn save_own_runes(
    app: AppHandle,
    shared: State<'_, Arc<Shared>>,
    champion_id: i64,
    page_id: Option<u32>,
    name: Option<String>,
) -> Result<Vec<own_runes::OwnEntry>, String> {
    app::save_own_runes(&app, shared.inner(), champion_id, page_id, name).await
}

/// Página montada na janela (editor): funciona sem o cliente aberto.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
async fn save_own_runes_page(
    app: AppHandle,
    shared: State<'_, Arc<Shared>>,
    champion_id: i64,
    page_id: Option<u32>,
    name: String,
    primary_style: i64,
    sub_style: i64,
    perks: Vec<i64>,
) -> Result<Vec<own_runes::OwnEntry>, String> {
    app::save_own_runes_page(
        &app,
        shared.inner(),
        champion_id,
        page_id,
        name,
        primary_style,
        sub_style,
        perks,
    )
    .await
}

#[tauri::command]
async fn delete_own_runes(
    app: AppHandle,
    shared: State<'_, Arc<Shared>>,
    champion_id: i64,
    page_id: u32,
) -> Result<Vec<own_runes::OwnEntry>, String> {
    app::delete_own_runes(&app, shared.inner(), champion_id, page_id).await
}

/// "pick" | "win" | "own" (com `page_id`) para o campeão em tela.
#[tauri::command]
async fn choose_runes(
    app: AppHandle,
    shared: State<'_, Arc<Shared>>,
    choice: String,
    page_id: Option<u32>,
) -> Result<Vec<own_runes::OwnEntry>, String> {
    app::choose_runes(&app, shared.inner(), choice, page_id).await
}

/// Último quadro do overlay (a janela pede ao carregar; depois vem pelo evento `overlay`).
#[tauri::command]
fn overlay_frame() -> Option<overlay::Frame> {
    overlay::last_frame()
}

/// Liga/desliga o modo de ajuste do overlay (arrastar as janelinhas).
#[tauri::command]
fn overlay_edit(app: AppHandle, on: bool) {
    overlay::set_edit(&app, on);
}

/// Posição de uma janelinha do overlay ("cs" ou "item"), em fração da tela do jogo.
#[tauri::command]
async fn save_overlay_pos(
    shared: State<'_, Arc<Shared>>,
    widget: String,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let pos = Some((x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)));
    let mut s = shared.settings.lock().await;
    match widget.as_str() {
        "cs" => s.overlay.cs_pos = pos,
        "item" => s.overlay.item_pos = pos,
        _ => return Err(format!("janelinha desconhecida: {widget}")),
    }
    s.save(&shared.settings_path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_update(app: AppHandle) -> Result<Option<update::UpdateInfo>, String> {
    Ok(update::pending(&app).await)
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<update::UpdateInfo>, String> {
    update::check(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    update::install(&app).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(update::Pending::default())
        .register_asynchronous_uri_scheme_protocol("lcu", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            let path = request.uri().path().to_string();
            tauri::async_runtime::spawn(async move {
                let got = match app.try_state::<Arc<Shared>>() {
                    Some(shared) => app::fetch_icon(&shared, &path).await,
                    None => Err("iniciando".into()),
                };
                let resp = match got {
                    Ok((bytes, ct)) => tauri::http::Response::builder()
                        .header("Content-Type", ct)
                        .header("Cache-Control", "max-age=86400")
                        .body(bytes),
                    Err(_) => tauri::http::Response::builder()
                        .status(404)
                        .body(Vec::new()),
                };
                if let Ok(r) = resp {
                    responder.respond(r);
                }
            });
        })
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let cache_dir = app.path().app_cache_dir()?.join("lolalytics");
            let shared = Arc::new(Shared::new(config_dir.join("settings.json"), cache_dir));
            app.manage(shared.overlay.clone());
            app.manage(shared);
            create_tray(app.handle())?;
            show_main(app.handle());
            tauri::async_runtime::spawn(app::worker(app.handle().clone()));
            tauri::async_runtime::spawn(update::watcher(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_assets,
            get_settings,
            save_settings,
            lookup,
            set_vs,
            reimport,
            get_composition,
            ban_champion,
            get_own_pages,
            save_own_runes,
            save_own_runes_page,
            delete_own_runes,
            choose_runes,
            overlay_frame,
            overlay_edit,
            save_overlay_pos,
            get_update,
            check_update,
            install_update
        ])
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Lobby Companion");
    app.run(|_app, event| {
        // Fechar a janela não encerra o app (fica na bandeja); "Sair" no menu encerra.
        if let tauri::RunEvent::ExitRequested { api, code, .. } = event
            && code.is_none()
        {
            api.prevent_exit();
        }
    });
}

/// Diagnóstico pelo terminal (`cargo run -- --diag` / `cargo run -- --lola chogath jungle [viego]` /
/// `cargo run -- --picks jungle [inimigo,…] [aliado:lane,…]` / `cargo run -- --maestria`).
/// Devolve `true` se tratou os argumentos (e o app não deve abrir).
pub fn cli(args: &[String]) -> bool {
    match args.first().map(String::as_str) {
        Some("--diag") => {
            tauri::async_runtime::block_on(diag());
            true
        }
        Some("--lola") => {
            let slug = args.get(1).cloned().unwrap_or_else(|| "chogath".into());
            let lane = args.get(2).cloned();
            let vs = args.get(3).cloned();
            tauri::async_runtime::block_on(lola_cli(slug, lane, vs));
            true
        }
        Some("--picks") => {
            let lane = args.get(1).cloned().unwrap_or_else(|| "jungle".into());
            let list = |i: usize| -> Vec<String> {
                args.get(i)
                    .map(|s| {
                        s.split(',')
                            .filter(|x| !x.is_empty())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default()
            };
            tauri::async_runtime::block_on(picks_cli(lane, list(2), list(3)));
            true
        }
        Some("--maestria") => {
            tauri::async_runtime::block_on(mastery_cli());
            true
        }
        Some("--hud") => {
            hud_cli();
            true
        }
        Some("--live") => {
            tauri::async_runtime::block_on(live::diag());
            true
        }
        _ => false,
    }
}

/// Configuração do HUD do jogo e onde o overlay desenharia (só leitura de arquivo).
fn hud_cli() {
    let Some(path) = lcu::game_cfg_path() else {
        println!("game.cfg: não encontrado (instalação do League não achada)");
        return;
    };
    println!("game.cfg: {}", path.display());
    let Ok(text) = std::fs::read_to_string(&path) else {
        println!("game.cfg: não deu para ler");
        return;
    };
    // Só as seções que importam para o overlay (o arquivo não tem nada de outros jogadores).
    let mut section = String::new();
    for line in text.lines() {
        let l = line.trim();
        if let Some(s) = l.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
            section = s.to_ascii_lowercase();
        }
        if section == "hud" || section == "general" {
            println!("  {l}");
        }
    }
    let cfg = hud::parse_game_cfg(&text);
    println!("lido: {cfg:?}");
    let (w, h) = (
        f64::from(cfg.width.unwrap_or(1920)),
        f64::from(cfg.height.unwrap_or(1080)),
    );
    println!("escala ({w}×{h}): {:.4}", hud::ui_scale(w, h));
    println!("minimapa: {:?}", hud::minimap(w, h, &cfg.hud));
    println!("placar: {:?}", hud::scoreboard(w, h));
    for row in 0..5 {
        println!("linha {row}: {:?}", hud::scoreboard_row_center(row, w, h));
    }
}

async fn diag() {
    let Some(creds) = lcu::discover() else {
        println!("cliente: fechado (lockfile não encontrado)");
        return;
    };
    println!("cliente: porta {}", creds.port);
    let Ok(client) = lcu::Lcu::new(&creds) else {
        return;
    };
    match client.get("/lol-gameflow/v1/gameflow-phase").await {
        Ok(p) => println!("fase: {p}"),
        Err(e) => println!("fase: erro {e}"),
    }
    // Idioma do cliente (o app usa `locale`; os outros campos só para conferir o formato).
    match client.get("/riotclient/region-locale").await {
        Ok(v) => println!(
            "idioma: {v} → {}",
            v.get("locale")
                .and_then(|l| l.as_str())
                .and_then(i18n::from_tag)
                .unwrap_or("(não reconhecido)")
        ),
        Err(e) => println!("idioma: erro {e}"),
    }
    match client.get("/lol-perks/v1/inventory").await {
        Ok(v) => println!("runas: {v}"),
        Err(e) => println!("runas: erro {e}"),
    }
    match client.get("/lol-champ-select/v1/session").await {
        Ok(s) => println!("seleção: {:?}", champselect::parse(&s)),
        Err(_) => println!("seleção: fora da seleção de campeões"),
    }
    match assets::Assets::load(&client).await {
        Ok(a) => println!(
            "assets: {} campeões, {} itens, {} runas, {} árvores, {} feitiços",
            a.champions.len(),
            a.items.len(),
            a.perks.len(),
            a.styles.len(),
            a.spells.len()
        ),
        Err(e) => println!("assets: erro {e}"),
    }
}

async fn lola_cli(slug: String, lane: Option<String>, vs: Option<String>) {
    let s = Settings::default();
    let q = lola::Query {
        slug,
        vs_lane: vs.as_ref().and(lane.clone()),
        lane,
        tier: s.tier,
        region: s.region,
        vs,
        ..Default::default()
    };
    let client = lola::Lolalytics::new(std::env::temp_dir().join("lobby-companion-cli"));
    match client
        .build_full(&q)
        .await
        .and_then(|f| lola::model::parse_build_full(&f.data))
    {
        Err(e) => println!("erro: {e}"),
        Ok(b) => {
            println!(
                "{} {} patch {}: WR {:.2}% PR {:.2}% ({} partidas)",
                q.slug, b.lane, b.patch, b.wr, b.pr, b.n
            );
            if let Some(r) = &b.runes_pick {
                println!(
                    "runas (mais jogada): {:?} WR {:.2}%",
                    r.selected_perk_ids(),
                    r.wr
                );
            }
            println!("feitiços: {:?}", b.spells.first().map(|c| &c.ids));
            println!("skills: {:?}", b.skill_order);
            for (i, slot) in b.slots.iter().enumerate() {
                let top: Vec<String> = slot
                    .iter()
                    .take(3)
                    .map(|x| format!("{} {:.1}% ({})", x.id, x.wr, x.n))
                    .collect();
                println!("{}º item: {}", i + 1, top.join(" | "));
            }
            if let Some(v) = &b.vs {
                println!(
                    "confronto: contra o campeão {} ({}), WR geral {:.2}%; corte de itens {}",
                    v.champion_id,
                    v.lane,
                    v.usual_wr,
                    b.min_games(s.min_games)
                );
            } else {
                let me = champselect::Ally {
                    champion_id: 0,
                    lane: Some(b.lane.clone()),
                    me: true,
                };
                let metric = metric::Metric::from_setting(&s.metric);
                let team = bans::suggest(&[(me.clone(), &b)], &[], 5, metric);
                let in_team: Vec<i64> = team.iter().map(|p| p.champion_id).collect();
                let lane = bans::lane_suggest(&me, &b, &[], &in_team, 5, metric);
                for (kind, list) in [("ban (time)", team), ("ban (lane)", lane)] {
                    for p in list {
                        let r = p.reasons.first();
                        println!(
                            "{kind} ({metric:?}): campeão {} +{:.2} pts ({})",
                            p.champion_id,
                            p.gain,
                            r.map(|r| format!("{:.1}% em {} partidas, {}", r.wr, r.n, r.lane))
                                .unwrap_or_default()
                        );
                    }
                }
            }
            println!("página: {}", q.page_url());
        }
    }
}

/// Formato da maestria no cliente (só GET): quantos campeões, os campos e os 5 com mais pontos.
/// Não imprime o `puuid` (é o seu, mas não precisa aparecer).
async fn mastery_cli() {
    let Some(client) = lcu::discover().and_then(|c| lcu::Lcu::new(&c).ok()) else {
        println!("cliente: fechado (lockfile não encontrado)");
        return;
    };
    match client
        .get("/lol-champion-mastery/v1/local-player/champion-mastery")
        .await
    {
        Err(e) => println!("maestria: erro {e}"),
        Ok(v) => {
            let kind = if v.is_array() { "lista" } else { "objeto" };
            let first = v
                .as_array()
                .and_then(|a| a.first())
                .or_else(|| v.pointer("/masteries/0"));
            let mut keys: Vec<&String> = first
                .and_then(|f| f.as_object())
                .map(|o| o.keys().collect())
                .unwrap_or_default();
            keys.sort();
            let m = lola::extra::parse_mastery(&v);
            println!(
                "maestria: {kind} com {} campeões; campos: {keys:?}",
                m.len()
            );
            println!("5 com mais pontos (id, pontos): {:?}", &m[..m.len().min(5)]);
        }
    }
}

/// Sugestão de pick com dados reais, sem o cliente: candidatos da tier list, inimigos e aliados
/// por slug (`aliado:lane`). Cada inimigo/aliado custa uma requisição (cache em
/// `%TEMP%\lobby-companion-cli`). Sem `lane`, o lolalytics usa a lane mais jogada do inimigo.
async fn picks_cli(lane: String, enemies: Vec<String>, allies: Vec<String>) {
    use lola::extra;
    use lola::{Ep, Query};
    let s = Settings::default();
    let q = |ep: Ep, slug: &str, lane: Option<String>, vs_lane: Option<String>| Query {
        ep,
        slug: slug.to_string(),
        lane,
        vs_lane,
        tier: s.tier.clone(),
        region: s.region.clone(),
        ..Default::default()
    };
    let client = lola::Lolalytics::new(std::env::temp_dir().join("lobby-companion-cli"));
    let list = match client
        .get(&q(Ep::List, "", Some(lane.clone()), None))
        .await
        .and_then(|f| extra::parse_list(&f.data))
    {
        Ok(l) => l,
        Err(e) => return println!("tier list: erro {e}"),
    };
    println!(
        "tier list {}: {} campeões, média {:.2}%",
        list.lane,
        list.entries.len(),
        list.avg_wr
    );
    let mut counters = Vec::new();
    for e in &enemies {
        let r = client
            .get(&q(Ep::Counter, e, None, Some(lane.clone())))
            .await
            .and_then(|f| extra::parse_counter(&f.data));
        match r {
            Ok(c) => {
                println!(
                    "inimigo {e} (#{}) {}: WR {:.2}%, {} oponentes na lane {}",
                    c.champion_id,
                    c.lane,
                    c.wr,
                    c.rows.len(),
                    c.vs_lane
                );
                counters.push(c);
            }
            Err(err) => println!("inimigo {e}: erro {err}"),
        }
    }
    let mut teams = Vec::new();
    for a in &allies {
        let (slug, a_lane) = a.split_once(':').unwrap_or((a.as_str(), "middle"));
        let r = client
            .get(&q(Ep::BuildTeam, slug, Some(a_lane.to_string()), None))
            .await
            .and_then(|f| extra::parse_build_team(&f.data));
        match r {
            Ok(t) => {
                println!("aliado {slug} ({a_lane}): {} lanes", t.lanes.len());
                teams.push((a_lane.to_string(), t));
            }
            Err(err) => println!("aliado {slug}: erro {err}"),
        }
    }
    let enemy_inputs: Vec<picks::Enemy> = counters
        .iter()
        .map(|c| picks::Enemy {
            champion_id: c.champion_id,
            lane: c.lane.clone(),
            counters: c,
        })
        .collect();
    let mate_inputs: Vec<picks::Mate> = teams
        .iter()
        .enumerate()
        .map(|(i, (l, t))| picks::Mate {
            champion_id: -(i as i64) - 1,
            lane: l.clone(),
            team: t,
        })
        .collect();
    let candidates = picks::pool_from_tier(&list, &[], picks::POOL_SIZE);
    for metric in [metric::Metric::Wr, metric::Metric::Wpa] {
        for p in picks::suggest(
            &lane,
            &list,
            &candidates,
            &enemy_inputs,
            &mate_inputs,
            &[],
            5,
            metric,
        ) {
            let parts: Vec<String> = p
                .parts
                .iter()
                .map(|x| {
                    format!(
                        "{:+.2} {:?} #{} ({}, WR {:.1}%, {} partidas)",
                        x.delta, x.kind, x.champion_id, x.lane, x.wr, x.n
                    )
                })
                .collect();
            println!(
                "pick ({metric:?}) #{}: {:+.2} (base {:+.2}, WR {:.2}% em {}) {}",
                p.champion_id,
                p.total,
                p.base,
                p.wr,
                p.games,
                parts.join(" | ")
            );
        }
        for b in picks::tier_bans(&list, &[], 5, metric) {
            let t = b.tier.as_ref();
            println!(
                "ban pela tier list ({metric:?}): #{} +{:.3} (WR {:.2}%, PR {:.2}%, BR {:.2}%)",
                b.champion_id,
                b.gain,
                t.map(|t| t.wr).unwrap_or(0.0),
                t.map(|t| t.pr).unwrap_or(0.0),
                t.map(|t| t.br).unwrap_or(0.0)
            );
        }
    }
}
