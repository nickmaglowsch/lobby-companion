//! Lobby Companion: importa runas, feitiços e conjuntos de itens do lolalytics na seleção de campeões.
//! Sem overlay: a janela abre na seleção e fecha quando a partida começa; fora disso, só o ícone na bandeja.

pub mod app;
pub mod assets;
pub mod bans;
pub mod champselect;
pub mod composition;
pub mod import;
pub mod lcu;
pub mod lola;
pub mod picks;
pub mod settings;
pub mod update;

use app::{ImportReport, Shared, Snapshot};
use settings::Settings;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

const MAIN: &str = "main";

/// Tamanho da janela pelo layout escolhido (padrão se as configurações estiverem ocupadas).
fn window_size(app: &AppHandle) -> (f64, f64, f64, f64) {
    app.try_state::<Arc<Shared>>()
        .and_then(|s| s.settings.try_lock().ok().map(|s| s.window_size()))
        .unwrap_or_else(|| Settings::default().window_size())
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
    let open = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    match update {
        Some(v) => {
            let label = format!("Atualizar para a v{}…", v.trim_start_matches('v'));
            let up = MenuItem::with_id(app, "update", label, true, None::<&str>)?;
            Menu::with_items(app, &[&up, &open, &quit])
        }
        None => Menu::with_items(app, &[&open, &quit]),
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
        Some(v) => format!("Lobby Companion: v{} disponível", v.trim_start_matches('v')),
        None => "Lobby Companion".into(),
    };
    let _ = tray.set_tooltip(Some(tip));
}

fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = tray_menu(app, None)?;
    let mut tray = TrayIconBuilder::with_id("tray")
        .tooltip("Lobby Companion")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "open" | "update" => show_main(app),
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
    settings: Settings,
) -> Result<(), String> {
    settings
        .save(&shared.settings_path)
        .map_err(|e| e.to_string())?;
    let (filters_changed, runes_changed, items_changed, bans_changed, picks_changed) = {
        let mut cur = shared.settings.lock().await;
        let changed = (
            cur.tier != settings.tier
                || cur.region != settings.region
                || cur.patch != settings.patch,
            cur.rune_choice != settings.rune_choice,
            cur.min_games != settings.min_games,
            cur.suggest_bans != settings.suggest_bans,
            cur.suggest_picks != settings.suggest_picks || cur.pick_pool != settings.pick_pool,
        );
        if cur.window_size() != settings.window_size() {
            apply_window_size(&app, settings.window_size());
        }
        *cur = settings;
        changed
    };
    let (champion_id, lane, in_select) = {
        let s = shared.snapshot.lock().await;
        (s.champion_id, s.lane.clone(), s.in_champ_select)
    };
    let shared = shared.inner().clone();
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
    let report = app::run_import(&shared).await?;
    let snap = {
        let mut s = shared.snapshot.lock().await;
        s.import = Some(report.clone());
        s.clone()
    };
    let _ = app.emit("snapshot", snap);
    Ok(report)
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
        build.as_ref(),
        assets.as_deref(),
        min_games,
    ))
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
            app.manage(Arc::new(Shared::new(
                config_dir.join("settings.json"),
                cache_dir,
            )));
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
        _ => false,
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
                let team = bans::suggest(&[(me.clone(), &b)], &[], 5);
                let in_team: Vec<i64> = team.iter().map(|p| p.champion_id).collect();
                let lane = bans::lane_suggest(&me, &b, &[], &in_team, 5);
                for (kind, list) in [("ban (time)", team), ("ban (lane)", lane)] {
                    for p in list {
                        let r = p.reasons.first();
                        println!(
                            "{kind}: campeão {} +{:.2} pts ({})",
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
    for p in picks::suggest(
        &lane,
        &list,
        &candidates,
        &enemy_inputs,
        &mate_inputs,
        &[],
        5,
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
            "pick #{}: {:+.2} (base {:+.2}, WR {:.2}% em {}) {}",
            p.champion_id,
            p.total,
            p.base,
            p.wr,
            p.games,
            parts.join(" | ")
        );
    }
    for b in picks::tier_bans(&list, &[], 5) {
        let t = b.tier.as_ref();
        println!(
            "ban pela tier list: #{} +{:.3} (WR {:.2}%, PR {:.2}%, BR {:.2}%)",
            b.champion_id,
            b.gain,
            t.map(|t| t.wr).unwrap_or(0.0),
            t.map(|t| t.pr).unwrap_or(0.0),
            t.map(|t| t.br).unwrap_or(0.0)
        );
    }
}
