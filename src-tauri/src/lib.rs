//! Lobby Companion: importa runas, feitiços e conjuntos de itens do lolalytics na seleção de campeões.
//! Sem overlay: a janela abre na seleção e fecha quando a partida começa; fora disso, só o ícone na bandeja.

pub mod app;
pub mod assets;
pub mod bans;
pub mod champselect;
pub mod import;
pub mod lcu;
pub mod lola;
pub mod settings;

use app::{ImportReport, Shared, Snapshot};
use settings::Settings;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

const MAIN: &str = "main";

/// Abre (ou traz para frente) a janela. Cria o WebView só quando precisa.
pub fn show_main(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(MAIN) {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
            return;
        }
        let _ = WebviewWindowBuilder::new(&handle, MAIN, WebviewUrl::App("index.html".into()))
            .title("Lobby Companion")
            .inner_size(480.0, 780.0)
            .min_inner_size(380.0, 480.0)
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

fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::with_id("tray")
        .tooltip("Lobby Companion")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "open" => show_main(app),
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
    let (filters_changed, runes_changed, items_changed, bans_changed) = {
        let mut cur = shared.settings.lock().await;
        let changed = (
            cur.tier != settings.tier
                || cur.region != settings.region
                || cur.patch != settings.patch,
            cur.rune_choice != settings.rune_choice,
            cur.min_games != settings.min_games,
            cur.suggest_bans != settings.suggest_bans,
        );
        *cur = settings;
        changed
    };
    let (champion_id, lane, in_select) = {
        let s = shared.snapshot.lock().await;
        (s.champion_id, s.lane.clone(), s.in_champ_select)
    };
    let shared = shared.inner().clone();
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_assets,
            get_settings,
            save_settings,
            lookup,
            set_vs,
            reimport
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

/// Diagnóstico pelo terminal (`cargo run -- --diag` / `cargo run -- --lola chogath jungle [viego]`).
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
                for p in bans::suggest(&[(me, &b)], &[], 5) {
                    let r = p.reasons.first();
                    println!(
                        "ban: campeão {} +{:.2} pts ({})",
                        p.champion_id,
                        p.gain,
                        r.map(|r| format!("{:.1}% em {} partidas, {}", r.wr, r.n, r.lane))
                            .unwrap_or_default()
                    );
                }
            }
            println!("página: {}", q.page_url());
        }
    }
}
