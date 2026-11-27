//! Overlay na partida: janela transparente, sempre por cima, que deixa o clique passar, do tamanho
//! da área de desenho do jogo. Lê só a API da partida (`live.rs`, GET em 127.0.0.1:2999) e o
//! `game.cfg` (`hud.rs`); nunca toca no processo do jogo. A tecla do placar é lida com
//! `GetAsyncKeyState` (só leitura; nunca mandamos tecla).
//!
//! Recursos (cada um ligável em `OverlaySettings`): timer de volta dos inibidores no minimapa,
//! próximo item do build importado quando o ouro dá, CS por minuto, diferença de ouro em itens por
//! rota e gráfico do ouro em itens (os dois com a tecla do placar segurada).
//!
//! As contas ficam aqui (testáveis); a janela (`src/lib/overlay/`) só desenha o `Frame`.

use crate::app::Shared;
use crate::assets::Assets;
use crate::hud::{self, HudConfig, Rect};
use crate::live::{self, Game, Player, Team};
use crate::settings::OverlaySettings;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "overlay";
/// O inibidor volta 5 min depois de destruído.
pub const INHIB_RESPAWN: f64 = 300.0;
/// Lado do Summoner's Rift no minimapa (`mCustomMinimapBackgrounds.mSize` do `map11.bin`).
const MAP_SIZE: f64 = 14850.0;
/// Um ponto do gráfico a cada 10 s de jogo.
const GRAPH_STEP: f64 = 10.0;
const POSITIONS: [&str; 5] = ["TOP", "JUNGLE", "MIDDLE", "BOTTOM", "UTILITY"];

/// Estado do overlay entre as voltas do laço (fora do `Shared` para não misturar com a seleção).
#[derive(Default)]
pub struct OverlayState {
    running: AtomicBool,
    /// Modo de ajuste: a janela aceita clique e as janelinhas se arrastam.
    edit: AtomicBool,
    tab: AtomicBool,
    /// Tecla do placar (código virtual).
    key: AtomicU32,
    /// Jogo em primeiro plano (o overlay some quando você troca de janela).
    foreground: AtomicBool,
    /// Seu campeão (alias do cliente) e a ordem de compra do build importado.
    me: Mutex<Me>,
    history: Mutex<GoldHistory>,
}

#[derive(Debug, Clone, Default)]
pub struct Me {
    pub alias: Option<String>,
    pub path: Vec<i64>,
}

impl OverlayState {
    /// Campeão travado na seleção (chamado pelo `app.rs`).
    pub fn set_champion(&self, alias: Option<String>) {
        if let Ok(mut me) = self.me.lock()
            && alias.is_some()
        {
            me.alias = alias;
        }
    }
    /// Ordem de compra do build importado (chamado na importação dos itens).
    pub fn set_path(&self, alias: Option<String>, path: Vec<i64>) {
        if let Ok(mut me) = self.me.lock() {
            if alias.is_some() {
                me.alias = alias;
            }
            me.path = path;
        }
    }
    pub fn editing(&self) -> bool {
        self.edit.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------- contas (sem janela)

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InhibTimer {
    pub x: f64,
    pub y: f64,
    /// Segundos até voltar.
    pub seconds: f64,
    /// Inibidor do seu time.
    pub ally: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NextItem {
    pub id: i64,
    /// Ouro que falta (componentes que você tem já descontados).
    pub cost: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cs {
    pub total: u32,
    pub per_min: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LaneDiff {
    /// Centro entre as duas colunas do placar, na altura da linha.
    pub x: f64,
    pub y: f64,
    /// Diferença (sempre positiva).
    pub gold: u32,
    /// O seu lado (coluna da esquerda) está na frente.
    pub ally_ahead: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    /// Sempre do mesmo tamanho (embaixo do placar).
    pub rect: Rect,
    /// `(segundos, seu time − inimigo)` em ouro de itens.
    pub points: Vec<(f64, f64)>,
}

/// Tudo o que a janela desenha, em pixels físicos da área de desenho do jogo.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    /// Dados de exemplo (modo de ajuste sem partida).
    pub preview: bool,
    pub edit: bool,
    pub visible: bool,
    pub tab: bool,
    pub width: f64,
    pub height: f64,
    /// Escala do HUD (para o tamanho das janelinhas acompanhar o do jogo).
    pub scale: f64,
    pub game_time: f64,
    pub minimap: Option<Rect>,
    pub scoreboard: Option<Rect>,
    pub inhibitors: Vec<InhibTimer>,
    pub next_item: Option<NextItem>,
    pub cs: Option<Cs>,
    pub lanes: Vec<LaneDiff>,
    pub graph: Option<Graph>,
    pub settings: OverlaySettings,
}

/// "Barracks_T1_L1" → (time, rota: L = topo, C = meio, R = robô).
fn parse_barracks(s: &str) -> Option<(Team, char)> {
    let rest = s.strip_prefix("Barracks_T")?;
    let mut c = rest.chars();
    let team = match c.next()? {
        '1' => Team::Order,
        '2' => Team::Chaos,
        _ => return None,
    };
    if c.next()? != '_' {
        return None;
    }
    let lane = c.next()?;
    matches!(lane, 'L' | 'C' | 'R').then_some((team, lane))
}

/// Posição dos inibidores no mapa (coordenadas do jogo, as dos eventos de estrutura das partidas).
fn inhib_world(team: Team, lane: char) -> Option<(f64, f64)> {
    Some(match (team, lane) {
        (Team::Order, 'L') => (1171.0, 3571.0),
        (Team::Order, 'C') => (3203.0, 3208.0),
        (Team::Order, 'R') => (3452.0, 1236.0),
        (Team::Chaos, 'L') => (11261.0, 13676.0),
        (Team::Chaos, 'C') => (11598.0, 11667.0),
        (Team::Chaos, 'R') => (13604.0, 11316.0),
        _ => return None,
    })
}

/// Ponto do mapa (x para a direita, y para cima) na tela, dentro do minimapa.
pub fn minimap_point(r: &Rect, (x, y): (f64, f64)) -> (f64, f64) {
    (r.x + x / MAP_SIZE * r.w, r.y + (1.0 - y / MAP_SIZE) * r.h)
}

/// Inibidores caídos e quanto falta para voltarem (o jogo anuncia a queda para todos).
pub fn inhib_timers(game: &Game, my_team: Team, minimap: &Rect) -> Vec<InhibTimer> {
    let mut out: Vec<((Team, char), f64)> = Vec::new();
    for e in &game.events {
        let Some(key) = e.target.as_deref().and_then(parse_barracks) else {
            continue;
        };
        match e.name.as_str() {
            "InhibKilled" => {
                out.retain(|(k, _)| *k != key);
                out.push((key, e.time));
            }
            "InhibRespawned" => out.retain(|(k, t)| *k != key || *t > e.time),
            _ => {}
        }
    }
    out.into_iter()
        .filter_map(|((team, lane), killed)| {
            let seconds = killed + INHIB_RESPAWN - game.time;
            if seconds <= 0.0 {
                return None;
            }
            let (x, y) = minimap_point(minimap, inhib_world(team, lane)?);
            Some(InhibTimer {
                x,
                y,
                seconds,
                ally: team == my_team,
            })
        })
        .collect()
}

/// Ouro em itens: preço total de cada item (sem consumíveis e berloques).
pub fn items_value(items: &[(i64, u32)], assets: &Assets) -> u32 {
    items
        .iter()
        .filter(|(id, _)| !assets.item_has(*id, "Consumable") && !assets.item_has(*id, "Trinket"))
        .map(|(id, _)| assets.items.get(id).map_or(0, |i| i.price))
        .sum()
}

/// Ouro que falta para `target` descontando os componentes que você tem (cada um usado uma vez).
pub fn remaining_cost(target: i64, owned: &[i64], assets: &Assets) -> u32 {
    fn go(id: i64, owned: &mut Vec<i64>, assets: &Assets, depth: u8) -> u32 {
        if let Some(pos) = owned.iter().position(|x| *x == id) {
            owned.remove(pos);
            return 0;
        }
        let Some(item) = assets.items.get(&id) else {
            return 0;
        };
        if depth > 6 || item.from.is_empty() {
            return item.price;
        }
        let parts: u32 = item
            .from
            .iter()
            .map(|c| assets.items.get(c).map_or(0, |i| i.price))
            .sum();
        let combine = item.price.saturating_sub(parts);
        combine
            + item
                .from
                .iter()
                .map(|c| go(*c, owned, assets, depth + 1))
                .sum::<u32>()
    }
    let Some(item) = assets.items.get(&target) else {
        return 0;
    };
    let mut owned = owned.to_vec();
    // O próprio item não conta como componente dele.
    if item.from.is_empty() {
        return item.price;
    }
    let parts: u32 = item
        .from
        .iter()
        .map(|c| assets.items.get(c).map_or(0, |i| i.price))
        .sum();
    item.price.saturating_sub(parts)
        + item
            .from
            .iter()
            .map(|c| go(*c, &mut owned, assets, 1))
            .sum::<u32>()
}

/// Próximo item do build que você ainda não tem, só quando o ouro da mochila dá.
pub fn next_item(path: &[i64], me: &Player, gold: f64, assets: &Assets) -> Option<NextItem> {
    let owned: Vec<i64> = me.items.iter().map(|(id, _)| *id).collect();
    let target = *path.iter().find(|id| !owned.contains(id))?;
    let cost = remaining_cost(target, &owned, assets);
    (cost > 0 && gold >= f64::from(cost)).then_some(NextItem { id: target, cost })
}

/// Ordem das linhas do placar: por posição quando todos têm uma (ranqueada), senão a da API.
fn rows<'a>(players: &[&'a Player]) -> Vec<&'a Player> {
    let idx = |p: &Player| POSITIONS.iter().position(|x| *x == p.position);
    let mut v: Vec<&Player> = players.to_vec();
    let all_known = v.iter().all(|p| idx(p).is_some());
    let mut seen: Vec<usize> = v.iter().filter_map(|p| idx(p)).collect();
    seen.sort_unstable();
    seen.dedup();
    if all_known && seen.len() == v.len() {
        v.sort_by_key(|p| idx(p));
    }
    v
}

/// Seu time e o inimigo, por linha do placar (aliados à esquerda).
fn teams(game: &Game, my_team: Team) -> (Vec<&Player>, Vec<&Player>) {
    let allies: Vec<&Player> = game.players.iter().filter(|p| p.team == my_team).collect();
    let enemies: Vec<&Player> = game.players.iter().filter(|p| p.team != my_team).collect();
    (rows(&allies), rows(&enemies))
}

/// Diferença de ouro em itens de cada par de rotas, no meio do placar.
pub fn lane_diffs(game: &Game, my_team: Team, assets: &Assets, w: f64, h: f64) -> Vec<LaneDiff> {
    let (allies, enemies) = teams(game, my_team);
    allies
        .iter()
        .zip(&enemies)
        .take(5)
        .enumerate()
        .filter_map(|(row, (a, e))| {
            let diff =
                i64::from(items_value(&a.items, assets)) - i64::from(items_value(&e.items, assets));
            if diff == 0 {
                return None;
            }
            let (x, y) = hud::scoreboard_row_center(row, w, h);
            Some(LaneDiff {
                x,
                y,
                gold: u32::try_from(diff.unsigned_abs()).unwrap_or(u32::MAX),
                ally_ahead: diff > 0,
            })
        })
        .collect()
}

/// Ouro em itens do seu time menos o do inimigo.
pub fn team_diff(game: &Game, my_team: Team, assets: &Assets) -> f64 {
    let (allies, enemies) = teams(game, my_team);
    let sum = |v: &[&Player]| -> f64 {
        v.iter()
            .map(|p| f64::from(items_value(&p.items, assets)))
            .sum()
    };
    sum(&allies) - sum(&enemies)
}

/// Pontos do gráfico (um a cada 10 s de jogo, desde que o app viu a partida).
#[derive(Debug, Default, Clone)]
pub struct GoldHistory {
    points: Vec<(f64, f64)>,
}

impl GoldHistory {
    pub fn push(&mut self, time: f64, diff: f64) {
        // Partida nova (ou o tempo voltou): começa de novo.
        if self.points.last().is_some_and(|(t, _)| time + 1.0 < *t) {
            self.points.clear();
        }
        match self.points.last() {
            Some((t, _)) if time - t < GRAPH_STEP => {}
            _ => self.points.push((time, diff)),
        }
    }
    /// Os pontos guardados mais o de agora.
    pub fn with_now(&self, time: f64, diff: f64) -> Vec<(f64, f64)> {
        let mut v = self.points.clone();
        if v.last().is_none_or(|(t, _)| time > *t) {
            v.push((time, diff));
        }
        v
    }
    pub fn clear(&mut self) {
        self.points.clear();
    }
}

/// Gráfico embaixo do placar, sempre do mesmo tamanho para a resolução.
pub fn graph_rect(w: f64, h: f64) -> Rect {
    let sb = hud::scoreboard(w, h);
    let k = hud::ui_scale(w, h);
    let (gw, gh) = (620.0 * k, 150.0 * k);
    Rect {
        x: sb.x + (sb.w - gw) / 2.0,
        y: sb.y + sb.h + 10.0 * k,
        w: gw,
        h: gh,
    }
}

/// Você na lista de jogadores: pelo campeão (o app não lê nomes).
fn find_me<'a>(game: &'a Game, alias: Option<&str>) -> Option<&'a Player> {
    let alias = alias?;
    game.players
        .iter()
        .find(|p| p.alias.eq_ignore_ascii_case(alias))
}

pub struct Inputs<'a> {
    pub game: &'a Game,
    pub me: &'a Me,
    pub assets: Option<&'a Assets>,
    pub hud: HudConfig,
    pub settings: &'a OverlaySettings,
    pub w: f64,
    pub h: f64,
}

/// Monta o quadro da janela. `history` recebe o ponto de agora.
pub fn build_frame(i: &Inputs, history: &mut GoldHistory) -> Frame {
    let (w, h, s) = (i.w, i.h, i.settings);
    let me = find_me(i.game, i.me.alias.as_deref());
    let my_team = me.map_or(Team::Order, |p| p.team);
    let minimap = hud::minimap(w, h, &i.hud);
    let mut f = Frame {
        width: w,
        height: h,
        scale: hud::ui_scale(w, h),
        game_time: i.game.time,
        minimap: Some(minimap),
        scoreboard: Some(hud::scoreboard(w, h)),
        settings: s.clone(),
        ..Default::default()
    };
    // Inibidores: posições do Summoner's Rift.
    if s.inhibitors && i.game.map == 11 {
        f.inhibitors = inhib_timers(i.game, my_team, &minimap);
    }
    if s.cs_per_min
        && let Some(p) = me
        && i.game.time >= 60.0
    {
        f.cs = Some(Cs {
            total: p.cs,
            per_min: f64::from(p.cs) / (i.game.time / 60.0),
        });
    }
    if let Some(assets) = i.assets {
        if s.next_item
            && let Some(p) = me
        {
            f.next_item = next_item(&i.me.path, p, i.game.gold, assets);
        }
        if s.gold_diff && me.is_some() {
            f.lanes = lane_diffs(i.game, my_team, assets, w, h);
        }
        if me.is_some() {
            let diff = team_diff(i.game, my_team, assets);
            history.push(i.game.time, diff);
            if s.gold_graph {
                f.graph = Some(Graph {
                    rect: graph_rect(w, h),
                    points: history.with_now(i.game.time, diff),
                });
            }
        }
    }
    f
}

/// Quadro de exemplo para o modo de ajuste fora da partida.
pub fn preview_frame(settings: &OverlaySettings, hud_cfg: &HudConfig, w: f64, h: f64) -> Frame {
    let minimap = hud::minimap(w, h, hud_cfg);
    let (ix, iy) = minimap_point(&minimap, (11598.0, 11667.0));
    let points: Vec<(f64, f64)> = (0..=90)
        .map(|i| {
            let t = f64::from(i) * 10.0;
            (t, (t / 60.0).sin() * 900.0 + t * 6.0)
        })
        .collect();
    Frame {
        preview: true,
        edit: true,
        visible: true,
        tab: true,
        width: w,
        height: h,
        scale: hud::ui_scale(w, h),
        game_time: 900.0,
        minimap: Some(minimap),
        scoreboard: Some(hud::scoreboard(w, h)),
        inhibitors: vec![InhibTimer {
            x: ix,
            y: iy,
            seconds: 154.0,
            ally: false,
        }],
        next_item: Some(NextItem {
            id: 3068,
            cost: 2700,
        }),
        cs: Some(Cs {
            total: 112,
            per_min: 7.5,
        }),
        lanes: (0..5)
            .map(|row| {
                let (x, y) = hud::scoreboard_row_center(row, w, h);
                LaneDiff {
                    x,
                    y,
                    gold: [200, 1350, 75, 900, 410][row],
                    ally_ahead: row % 2 == 0,
                }
            })
            .collect(),
        graph: Some(Graph {
            rect: graph_rect(w, h),
            points,
        }),
        settings: settings.clone(),
    }
}

// ---------------------------------------------------------------- janela

/// Área de desenho do jogo em pixels físicos da tela e se ele está em primeiro plano.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameRect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub foreground: bool,
}

#[cfg(windows)]
mod win {
    use super::GameRect;
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetClientRect, GetForegroundWindow, IsIconic,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// Janela do jogo (classe `RiotWindowClass`, título "League of Legends (TM) Client").
    pub fn game_rect() -> Option<GameRect> {
        let title = wide("League of Legends (TM) Client");
        // SAFETY: chamadas Win32 só de leitura, com buffers válidos durante a chamada.
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
            if hwnd.is_null() || IsIconic(hwnd) != 0 {
                return None;
            }
            let mut r = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if GetClientRect(hwnd, &mut r) == 0 {
                return None;
            }
            let mut p = POINT { x: 0, y: 0 };
            if ClientToScreen(hwnd, &mut p) == 0 {
                return None;
            }
            let (w, h) = ((r.right - r.left).max(0), (r.bottom - r.top).max(0));
            if w == 0 || h == 0 {
                return None;
            }
            Some(GameRect {
                x: p.x,
                y: p.y,
                w: w.unsigned_abs(),
                h: h.unsigned_abs(),
                foreground: GetForegroundWindow() == hwnd,
            })
        }
    }

    /// Tecla segurada agora (só leitura do estado do teclado).
    pub fn key_down(vk: u32) -> bool {
        let Ok(vk) = i32::try_from(vk) else {
            return false;
        };
        // SAFETY: só lê o estado da tecla.
        // Bit mais alto ligado (valor negativo) = tecla segurada.
        unsafe { GetAsyncKeyState(vk) < 0 }
    }
}

#[cfg(not(windows))]
mod win {
    use super::GameRect;
    /// Fora do Windows (desenvolvimento) não há jogo: o overlay só abre no modo de ajuste.
    pub fn game_rect() -> Option<GameRect> {
        None
    }
    pub fn key_down(_vk: u32) -> bool {
        false
    }
}

/// Cria a janela (se preciso) e põe sobre a área do jogo. Na thread principal.
fn place_window(app: &AppHandle, x: i32, y: i32, w: u32, h: u32, interactive: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let win = match handle.get_webview_window(LABEL) {
            Some(win) => win,
            None => {
                let built = WebviewWindowBuilder::new(
                    &handle,
                    LABEL,
                    WebviewUrl::App("index.html#overlay".into()),
                )
                .title("Lobby Companion Overlay")
                // `transparent` liga o foco: o `focused(false)` tem que vir depois.
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .shadow(false)
                .resizable(false)
                .focused(false)
                .focusable(false)
                .build();
                match built {
                    Ok(win) => win,
                    Err(_) => return,
                }
            }
        };
        let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = win.set_size(tauri::PhysicalSize::new(w, h));
        let _ = win.set_focusable(interactive);
        let _ = win.set_ignore_cursor_events(!interactive);
        if interactive {
            let _ = win.set_focus();
        }
    });
}

pub fn close_window(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(w) = handle.get_webview_window(LABEL) {
            let _ = w.destroy();
        }
    });
}

/// Área do monitor principal (modo de ajuste sem partida).
fn primary_rect(app: &AppHandle) -> GameRect {
    let m = app.primary_monitor().ok().flatten();
    let (pos, size) = m.map_or(((0, 0), (1920, 1080)), |m| {
        (
            (m.position().x, m.position().y),
            (m.size().width, m.size().height),
        )
    });
    GameRect {
        x: pos.0,
        y: pos.1,
        w: size.0,
        h: size.1,
        foreground: true,
    }
}

fn read_hud() -> HudConfig {
    crate::lcu::game_cfg_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| hud::parse_game_cfg(&t).hud)
        .unwrap_or_default()
}

fn in_game(phase: &str) -> bool {
    matches!(phase, "GameStart" | "InProgress" | "Reconnect")
}

/// Liga ou desliga o modo de ajuste (bandeja, configurações ou o botão "Concluir" do overlay).
pub fn set_edit(app: &AppHandle, on: bool) {
    let Some(st) = app.try_state::<Arc<OverlayState>>() else {
        return;
    };
    st.edit.store(on, Ordering::Relaxed);
    let st = st.inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let shared = app.state::<Arc<Shared>>().inner().clone();
        let phase = shared.snapshot.lock().await.phase.clone();
        if in_game(&phase) && st.running.load(Ordering::Relaxed) {
            // O laço da partida ajusta a janela na próxima volta; manda já o quadro novo.
            return;
        }
        if on {
            let settings = shared.settings.lock().await.overlay.clone();
            let r = primary_rect(&app);
            place_window(&app, r.x, r.y, r.w, r.h, true);
            let frame = preview_frame(&settings, &read_hud(), f64::from(r.w), f64::from(r.h));
            // A janela recém-criada pede o quadro com `overlay_frame` quando carrega.
            if let Ok(mut last) = LAST.lock() {
                *last = Some(frame.clone());
            }
            let _ = app.emit_to(LABEL, "overlay", frame);
        } else {
            close_window(&app);
        }
    });
}

/// Último quadro (a janela pede ao carregar).
static LAST: Mutex<Option<Frame>> = Mutex::new(None);

pub fn last_frame() -> Option<Frame> {
    LAST.lock().ok().and_then(|l| l.clone())
}

/// Laço da partida: começa em `InProgress` e termina quando a partida acaba.
pub async fn run(app: AppHandle, shared: Arc<Shared>) {
    let Some(st) = app
        .try_state::<Arc<OverlayState>>()
        .map(|s| s.inner().clone())
    else {
        return;
    };
    if st.running.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Ok(mut h) = st.history.lock() {
        h.clear();
    }
    let Some(http) = live::client() else {
        st.running.store(false, Ordering::SeqCst);
        return;
    };
    // Tecla do placar: lida a cada 30 ms em outra thread (o laço principal é de 1 s).
    {
        let (st, app) = (st.clone(), app.clone());
        std::thread::spawn(move || {
            while st.running.load(Ordering::Relaxed) {
                let down = st.foreground.load(Ordering::Relaxed)
                    && win::key_down(st.key.load(Ordering::Relaxed));
                if st.tab.swap(down, Ordering::Relaxed) != down {
                    let _ = app.emit_to(LABEL, "overlay-tab", down);
                }
                std::thread::sleep(Duration::from_millis(30));
            }
        });
    }
    let mut placed: Option<(GameRect, bool)> = None;
    let mut hud_cfg = read_hud();
    let mut ticks: u32 = 0;
    loop {
        let phase = shared.snapshot.lock().await.phase.clone();
        if !in_game(&phase) {
            break;
        }
        let settings = shared.settings.lock().await.overlay.clone();
        let edit = st.editing();
        st.key.store(settings.scoreboard_key, Ordering::Relaxed);
        if !settings.enabled && !edit {
            if placed.take().is_some() {
                close_window(&app);
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }
        // O game.cfg muda quando você mexe nas opções do jogo: relê a cada 5 s.
        ticks = ticks.wrapping_add(1);
        if ticks.is_multiple_of(5) {
            hud_cfg = read_hud();
        }
        let game = live::fetch(&http).await.as_ref().and_then(live::parse);
        let rect = win::game_rect();
        let (Some(game), Some(rect)) = (game, rect) else {
            st.foreground.store(false, Ordering::Relaxed);
            if placed.is_some() {
                let _ = app.emit_to(
                    LABEL,
                    "overlay",
                    Frame {
                        settings,
                        edit,
                        ..Default::default()
                    },
                );
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };
        st.foreground.store(rect.foreground, Ordering::Relaxed);
        let geometry = GameRect {
            foreground: false,
            ..rect
        };
        if placed != Some((geometry, edit)) {
            place_window(&app, rect.x, rect.y, rect.w, rect.h, edit);
            placed = Some((geometry, edit));
        }
        let me = st.me.lock().map(|m| m.clone()).unwrap_or_default();
        let assets = shared.assets.read().await.clone();
        let mut frame = {
            let mut history = st.history.lock().unwrap_or_else(|e| e.into_inner());
            build_frame(
                &Inputs {
                    game: &game,
                    me: &me,
                    assets: assets.as_deref(),
                    hud: hud_cfg,
                    settings: &settings,
                    w: f64::from(rect.w),
                    h: f64::from(rect.h),
                },
                &mut history,
            )
        };
        frame.edit = edit;
        frame.visible = rect.foreground || edit;
        frame.tab = st.tab.load(Ordering::Relaxed) || edit;
        if let Ok(mut last) = LAST.lock() {
            *last = Some(frame.clone());
        }
        let _ = app.emit_to(LABEL, "overlay", frame);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    st.running.store(false, Ordering::SeqCst);
    st.edit.store(false, Ordering::Relaxed);
    if let Ok(mut last) = LAST.lock() {
        *last = None;
    }
    close_window(&app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Named;
    use crate::live::Event;
    use std::collections::HashMap;

    fn item(price: u32, from: &[i64], cats: &[&str]) -> Named {
        Named {
            name: String::new(),
            icon: String::new(),
            categories: cats.iter().map(|c| c.to_string()).collect(),
            price,
            from: from.to_vec(),
        }
    }

    /// Itens inventados com a estrutura do `items.json` (preço total e componentes).
    fn assets() -> Assets {
        let mut items = HashMap::new();
        items.insert(1036, item(350, &[], &[])); // espada longa
        items.insert(1037, item(875, &[], &[])); // picareta
        items.insert(3133, item(1100, &[1036, 1036], &[])); // martelo: 400 de receita
        items.insert(3071, item(3000, &[3133, 1037], &[])); // final: 1025 de receita
        items.insert(2003, item(50, &[], &["Consumable"]));
        items.insert(3340, item(0, &[], &["Trinket"]));
        items.insert(1001, item(300, &[], &["Boots"]));
        Assets {
            items,
            ..Default::default()
        }
    }

    fn player(alias: &str, team: Team, pos: &str, items: &[i64], cs: u32) -> Player {
        Player {
            alias: alias.into(),
            team,
            position: pos.into(),
            items: items.iter().map(|i| (*i, 1)).collect(),
            cs,
            level: 10,
        }
    }

    fn game(players: Vec<Player>, events: Vec<Event>, time: f64) -> Game {
        Game {
            time,
            map: 11,
            gold: 1500.0,
            level: 10,
            players,
            events,
        }
    }

    #[test]
    fn remaining_cost_discounts_owned_components() {
        let a = assets();
        assert_eq!(remaining_cost(3071, &[], &a), 3000);
        // Com o martelo: falta a receita (1025) + a picareta (875).
        assert_eq!(remaining_cost(3071, &[3133], &a), 1900);
        // Com uma espada longa: 3000 − 350.
        assert_eq!(remaining_cost(3071, &[1036], &a), 2650);
        // Duas espadas e a picareta: 1025 + 400.
        assert_eq!(remaining_cost(3071, &[1036, 1036, 1037], &a), 1425);
        // Item sem componentes: o preço cheio.
        assert_eq!(remaining_cost(1037, &[1037], &a), 875);
    }

    #[test]
    fn next_item_only_when_gold_is_enough() {
        let a = assets();
        let me = player("Chogath", Team::Order, "JUNGLE", &[1001, 3133], 80);
        let path = [1001, 3071, 1037];
        // Botas já tem; o próximo é o 3071, faltando 1900.
        assert_eq!(next_item(&path, &me, 1899.0, &a), None);
        assert_eq!(
            next_item(&path, &me, 1900.0, &a),
            Some(NextItem {
                id: 3071,
                cost: 1900
            })
        );
        // Build completo: nada.
        let done = player("Chogath", Team::Order, "JUNGLE", &[1001, 3071, 1037], 80);
        assert_eq!(next_item(&path, &done, 9999.0, &a), None);
    }

    #[test]
    fn items_value_skips_consumables_and_trinkets() {
        let a = assets();
        assert_eq!(items_value(&[(3071, 1), (2003, 2), (3340, 1)], &a), 3000);
        assert_eq!(items_value(&[(9999, 1)], &a), 0);
    }

    #[test]
    fn lane_diffs_pair_rows_by_position_and_point_to_the_leader() {
        let a = assets();
        // Inimigos fora de ordem na API: a posição manda.
        let g = game(
            vec![
                player("A", Team::Chaos, "JUNGLE", &[3071], 0),
                player("B", Team::Order, "TOP", &[1036], 0),
                player("C", Team::Order, "JUNGLE", &[3133], 0),
                player("D", Team::Chaos, "TOP", &[1037], 0),
                player("E", Team::Order, "MIDDLE", &[1037], 0),
                player("F", Team::Chaos, "MIDDLE", &[1037], 0),
                player("G", Team::Order, "BOTTOM", &[], 0),
                player("H", Team::Chaos, "BOTTOM", &[], 0),
                player("I", Team::Order, "UTILITY", &[], 0),
                player("J", Team::Chaos, "UTILITY", &[], 0),
            ],
            vec![],
            600.0,
        );
        let d = lane_diffs(&g, Team::Order, &a, 1920.0, 1080.0);
        // Topo: 350 × 875 → inimigo +525. Selva: 1100 × 3000 → inimigo +1900. Meio igual: sem seta.
        assert_eq!(d.len(), 2);
        assert_eq!((d[0].gold, d[0].ally_ahead), (525, false));
        assert_eq!((d[1].gold, d[1].ally_ahead), (1900, false));
        assert_eq!(
            (d[1].x, d[1].y),
            hud::scoreboard_row_center(1, 1920.0, 1080.0)
        );
        // Do outro lado da partida, as setas viram.
        let d = lane_diffs(&g, Team::Chaos, &a, 1920.0, 1080.0);
        assert!(d.iter().all(|l| l.ally_ahead));
        assert_eq!(team_diff(&g, Team::Chaos, &a), 1900.0 + 525.0);
    }

    #[test]
    fn inhibitor_timers_follow_kill_and_respawn_events() {
        let ev = |name: &str, t: f64, target: &str| Event {
            name: name.into(),
            time: t,
            target: Some(target.into()),
        };
        let mm = hud::minimap(1920.0, 1080.0, &HudConfig::default());
        let g = game(
            vec![],
            vec![
                ev("InhibKilled", 1500.0, "Barracks_T2_C1"),
                ev("InhibKilled", 1000.0, "Barracks_T1_L1"),
                ev("InhibRespawned", 1300.0, "Barracks_T1_L1"),
                ev("TurretKilled", 900.0, "Turret_T1_L_03_A"),
            ],
            1560.0,
        );
        let t = inhib_timers(&g, Team::Order, &mm);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].seconds, 240.0);
        assert!(!t[0].ally);
        // Inibidor do meio vermelho: canto de cima à direita do minimapa.
        assert!(t[0].x > mm.x + mm.w * 0.7 && t[0].y < mm.y + mm.h * 0.3);
        // Depois dos 5 min, some.
        let later = Game { time: 1800.0, ..g };
        assert!(inhib_timers(&later, Team::Order, &mm).is_empty());
        assert_eq!(parse_barracks("Barracks_T1_R1"), Some((Team::Order, 'R')));
        assert_eq!(parse_barracks("Turret_T1_L_03_A"), None);
    }

    #[test]
    fn history_keeps_one_point_every_ten_seconds_and_resets() {
        let mut h = GoldHistory::default();
        h.push(0.0, 0.0);
        h.push(4.0, 10.0);
        h.push(10.0, 50.0);
        assert_eq!(
            h.with_now(12.0, 60.0),
            vec![(0.0, 0.0), (10.0, 50.0), (12.0, 60.0)]
        );
        // Tempo voltou: partida nova.
        h.push(3.0, 5.0);
        assert_eq!(h.with_now(3.0, 5.0), vec![(3.0, 5.0)]);
    }

    #[test]
    fn frame_uses_my_champion_and_the_settings() {
        let a = assets();
        let g = game(
            vec![
                player("Chogath", Team::Chaos, "JUNGLE", &[1001], 90),
                player("Viego", Team::Order, "JUNGLE", &[3071], 100),
            ],
            vec![],
            600.0,
        );
        let me = Me {
            alias: Some("chogath".into()),
            path: vec![1001, 1037],
        };
        let settings = OverlaySettings::default();
        let mut h = GoldHistory::default();
        let inputs = Inputs {
            game: &g,
            me: &me,
            assets: Some(&a),
            hud: HudConfig::default(),
            settings: &settings,
            w: 1920.0,
            h: 1080.0,
        };
        let f = build_frame(&inputs, &mut h);
        assert_eq!(
            f.cs,
            Some(Cs {
                total: 90,
                per_min: 9.0
            })
        );
        assert_eq!(
            f.next_item,
            Some(NextItem {
                id: 1037,
                cost: 875
            })
        );
        assert_eq!(f.lanes.len(), 1);
        assert!(!f.lanes[0].ally_ahead);
        let graph = f.graph.unwrap();
        assert_eq!(graph.points, vec![(600.0, 300.0 - 3000.0)]);
        let fewer = OverlaySettings {
            gold_graph: false,
            cs_per_min: false,
            ..OverlaySettings::default()
        };
        let f = build_frame(
            &Inputs {
                settings: &fewer,
                ..inputs
            },
            &mut h,
        );
        assert!(f.graph.is_none() && f.cs.is_none());
        // Sem saber quem é você: nada que dependa disso.
        let nobody = Me::default();
        let f = build_frame(
            &Inputs {
                me: &nobody,
                settings: &OverlaySettings::default(),
                ..inputs
            },
            &mut h,
        );
        assert!(f.cs.is_none() && f.next_item.is_none() && f.lanes.is_empty());
    }

    #[test]
    fn preview_fits_the_screen() {
        let f = preview_frame(
            &OverlaySettings::default(),
            &HudConfig::default(),
            2560.0,
            1080.0,
        );
        let g = f.graph.unwrap();
        assert!(g.rect.x > 0.0 && g.rect.x + g.rect.w < 2560.0);
        assert!(g.rect.y + g.rect.h < 1080.0);
        assert_eq!(f.lanes.len(), 5);
    }
}
