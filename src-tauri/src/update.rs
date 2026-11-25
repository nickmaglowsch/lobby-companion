//! Atualização pelo GitHub Releases (`tauri-plugin-updater`).
//!
//! Procura versão nova uns segundos depois de abrir e a cada 6 h (configuração `checkUpdates`),
//! avisa na janela (evento `update-available`) e no menu da bandeja, e **só instala quando o
//! usuário clica**. O instalador baixado é conferido pela assinatura do updater (chave pública em
//! `tauri.conf.json`, a privada fica só no secret do GitHub Actions) antes de rodar. No Windows o
//! plugin roda o instalador em modo `passive` (barra de progresso, sem perguntas), fecha o app e o
//! instalador abre a versão nova no fim.
//!
//! A consulta é um GET de `latest.json` no GitHub; não manda nada além da requisição em si.

use crate::app::Shared;
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::Mutex;

const FIRST_CHECK: Duration = Duration::from_secs(8);
const EVERY: Duration = Duration::from_secs(6 * 3600);
const RELEASES: &str = "https://github.com/nickmaglowsch/lobby-companion/releases";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    /// Notas da release (markdown), se vierem no `latest.json`.
    pub notes: Option<String>,
    /// Página da release no GitHub, para ler as novidades.
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Atualização encontrada na última consulta (a que o botão instala).
#[derive(Default)]
pub struct Pending(pub Mutex<Option<Update>>);

pub fn release_url(version: &str) -> String {
    format!("{RELEASES}/tag/v{}", version.trim_start_matches('v'))
}

fn info(u: &Update) -> UpdateInfo {
    UpdateInfo {
        version: u.version.clone(),
        current: u.current_version.clone(),
        notes: u.body.clone().filter(|b| !b.trim().is_empty()),
        url: release_url(&u.version),
    }
}

/// Consulta o GitHub agora. Guarda o resultado, avisa a janela e a bandeja.
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| format!("não deu para consultar o GitHub: {e}"))?;
    let found = update.as_ref().map(info);
    *app.state::<Pending>().0.lock().await = update;
    crate::set_tray_update(app, found.as_ref().map(|i| i.version.as_str()));
    if let Some(i) = &found {
        let _ = app.emit("update-available", i);
    }
    Ok(found)
}

pub async fn pending(app: &AppHandle) -> Option<UpdateInfo> {
    app.state::<Pending>().0.lock().await.as_ref().map(info)
}

/// Laço da bandeja: consulta ao abrir e a cada 6 h, se a configuração deixar. Erro de rede é
/// silencioso (tenta de novo na próxima volta).
pub async fn watcher(app: AppHandle) {
    tokio::time::sleep(FIRST_CHECK).await;
    loop {
        let enabled = app
            .state::<Arc<Shared>>()
            .settings
            .lock()
            .await
            .check_updates;
        if enabled {
            let _ = check(&app).await;
        }
        tokio::time::sleep(EVERY).await;
    }
}

/// Baixa, confere a assinatura e roda o instalador. No Windows o processo termina aqui
/// (o instalador reabre o app); só volta se der erro.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .state::<Pending>()
        .0
        .lock()
        .await
        .clone()
        .ok_or("nenhuma atualização encontrada")?;
    let emitter = app.clone();
    let mut downloaded: u64 = 0;
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = emitter.emit("update-progress", Progress { downloaded, total });
            },
            || {},
        )
        .await
        .map_err(|e| format!("a atualização falhou: {e}"))?;
    // Fora do Windows o plugin não reinicia sozinho.
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_url_points_to_the_tag() {
        assert_eq!(
            release_url("0.4.0"),
            "https://github.com/nickmaglowsch/lobby-companion/releases/tag/v0.4.0"
        );
        assert_eq!(release_url("v0.4.0"), release_url("0.4.0"));
    }
}
