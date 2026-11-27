//! Partida gravada, para conferir o overlay sem jogar de novo.
//!
//! - `--gravar [arquivo]`: durante uma partida, guarda a resposta de `allgamedata` (só GET) já
//!   passada por `live::sanitize`, então **sem nomes nem Riot IDs**: uma linha JSON a cada 5 s de
//!   jogo e outra sempre que chega um evento novo (a queda de um inibidor fica no segundo certo).
//!   Para sozinho quando a API some (fim da partida).
//! - `--replay <arquivo> [campeão] [item,item,…]`: passa a gravação pelas mesmas contas do overlay
//!   (`inhib_timers`, `find_me`, `next_item`, com o HUD padrão em 1920×1080 e os itens do
//!   `assets.json`) e mostra, linha a linha, os eventos de estrutura e o que o overlay desenharia.

use crate::assets::Assets;
use crate::hud::{self, HudConfig};
use crate::live::{self, Team};
use crate::overlay::{self, InhibTimer, NextItem};
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Segundos de jogo entre duas linhas sem evento novo.
const STEP: f64 = 5.0;
/// Segundos sem resposta da API, depois de começar, para dar a partida por encerrada.
const END_AFTER: u32 = 30;

/// Pasta de dados do app (a mesma do `assets.json`: `%LOCALAPPDATA%\<identifier>`).
pub fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("dev.nickmaglowsch.lobbycompanion")
}

fn mmss(sec: f64) -> String {
    let s = sec.max(0.0).round() as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn events_of(v: &Value) -> &[Value] {
    v["events"].as_array().map_or(&[], Vec::as_slice)
}

/// `--gravar`: grava a partida em andamento (ou a próxima) até ela acabar.
pub async fn record(out: Option<PathBuf>) {
    let Some(http) = live::client() else {
        println!("erro ao criar o cliente HTTP");
        return;
    };
    let path = out.unwrap_or_else(|| {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        data_dir()
            .join("partidas")
            .join(format!("partida-{secs}.jsonl"))
    });
    println!("esperando a partida (a API só responde com o jogo aberto)…");
    let mut file: Option<std::io::BufWriter<std::fs::File>> = None;
    let (mut last_time, mut last_events, mut misses) = (f64::NEG_INFINITY, 0usize, 0u32);
    loop {
        match live::fetch(&http).await {
            Some(all) => {
                misses = 0;
                let mut s = live::sanitize(all);
                // Atributos do campeão: o overlay não usa e são metade da linha.
                if let Some(a) = s.get_mut("activePlayer").and_then(Value::as_object_mut) {
                    a.remove("championStats");
                }
                let time = s
                    .pointer("/gameData/gameTime")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let events = events_of(&s);
                if time - last_time < STEP && events.len() == last_events {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                for e in events.iter().skip(last_events) {
                    let name = e["EventName"].as_str().unwrap_or("?");
                    let target = e.get(name).and_then(Value::as_str).unwrap_or("");
                    println!(
                        "[{}] {name} {target}",
                        mmss(e["EventTime"].as_f64().unwrap_or(0.0))
                    );
                }
                last_events = events.len();
                last_time = time;
                if file.is_none() {
                    if let Some(dir) = path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    match std::fs::File::create(&path) {
                        Ok(f) => {
                            println!("gravando em {}", path.display());
                            file = Some(std::io::BufWriter::new(f));
                        }
                        Err(e) => {
                            println!("não deu para criar {}: {e}", path.display());
                            return;
                        }
                    }
                }
                if let Some(f) = &mut file {
                    // Linha a linha e já no disco: fechar o terminal no meio não perde o que veio.
                    let ok = serde_json::to_writer(&mut *f, &s).is_ok()
                        && f.write_all(b"\n").is_ok()
                        && f.flush().is_ok();
                    if !ok {
                        println!("erro ao gravar {}", path.display());
                        return;
                    }
                }
            }
            None if file.is_some() => {
                misses += 1;
                if misses >= END_AFTER {
                    println!("partida encerrada: {}", path.display());
                    return;
                }
            }
            None => {}
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Lado e rota de "Barracks_T1_L1" por extenso (ou o aviso de formato desconhecido).
fn describe_barracks(s: &str) -> String {
    match overlay::parse_barracks(s) {
        Some((team, lane)) => {
            let team = if team == Team::Order {
                "azul"
            } else {
                "vermelho"
            };
            let lane = match lane {
                'L' => "topo",
                'C' => "meio",
                _ => "robô",
            };
            format!("inibidor {team} do {lane}")
        }
        None => "formato desconhecido: o overlay ignora".into(),
    }
}

fn describe_timers(t: &[InhibTimer]) -> String {
    if t.is_empty() {
        return "nenhum".into();
    }
    t.iter()
        .map(|t| {
            let side = if t.ally { "aliado" } else { "inimigo" };
            format!("{side} {} em ({:.0}, {:.0})", mmss(t.seconds), t.x, t.y)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_next(n: Option<&NextItem>, assets: Option<&Assets>) -> String {
    match (n, assets) {
        (Some(n), Some(a)) => format!("{} (faltam {})", a.item_name(n.id), n.cost),
        (Some(n), None) => format!("#{} (faltam {})", n.id, n.cost),
        (None, _) => "nenhum".into(),
    }
}

/// `--replay`: o que o overlay faria em cada momento da partida gravada.
pub fn replay(file: &Path, alias: Option<&str>, path: &[i64]) {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(e) => {
            println!("não deu para ler {}: {e}", file.display());
            return;
        }
    };
    let assets = Assets::load_cached(&data_dir().join("assets.json"));
    if assets.is_none() {
        println!("sem assets.json (abra o app com o cliente uma vez): sem nomes de itens");
    }
    if path.is_empty() {
        println!("sem itens do build (3º argumento): o próximo item fica desligado");
    }
    let (w, h) = (1920.0, 1080.0);
    let minimap = hud::minimap(w, h, &HudConfig::default());
    println!(
        "minimapa em {w}×{h}: x {:.0}, y {:.0}, {:.0}×{:.0}",
        minimap.x, minimap.y, minimap.w, minimap.h
    );
    let (mut seen, mut last_state, mut map) = (0usize, String::new(), None);
    for line in text.lines() {
        let Some(game) = serde_json::from_str::<Value>(line)
            .ok()
            .as_ref()
            .and_then(live::parse_clean)
        else {
            continue;
        };
        if map != Some(game.map) {
            map = Some(game.map);
            let note = if game.map == 11 {
                ""
            } else {
                " (inibidores só no mapa 11)"
            };
            println!("mapa {}{note}", game.map);
        }
        for e in game.events.iter().skip(seen) {
            if let Some(target) = &e.target {
                let what = if e.name.starts_with("Inhib") {
                    describe_barracks(target)
                } else {
                    String::new()
                };
                println!("[{}] {} {target} {what}", mmss(e.time), e.name);
            }
        }
        seen = game.events.len();
        let me = overlay::find_me(&game, alias);
        let team = me.map_or(Team::Order, |p| p.team);
        let timers = if game.map == 11 {
            overlay::inhib_timers(&game, team, &minimap)
        } else {
            Vec::new()
        };
        let next = me.and_then(|p| overlay::next_item(path, p, game.gold, assets.as_ref()?));
        // Uma linha quando muda o que aparece (a contagem dos timers não conta como mudança).
        let me_text = me.map_or("não achado".to_string(), |p| {
            format!("{} ({:?})", p.alias, p.team)
        });
        let state = format!(
            "{me_text}|{}|{:?}",
            timers
                .iter()
                .map(|t| format!("{}{:.0}{:.0}", t.ally, t.x, t.y))
                .collect::<String>(),
            next.as_ref().map(|n| (n.id, n.cost))
        );
        if state != last_state {
            println!(
                "[{}] você: {me_text}; inibidores: {}; próximo item: {}; ouro {:.0}",
                mmss(game.time),
                describe_timers(&timers),
                describe_next(next.as_ref(), assets.as_ref()),
                game.gold
            );
            last_state = state;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recorded_lines_replay_without_names() {
        // Linha no formato gravado (já limpa): eventos como lista, sem `Events`.
        let raw = json!({
            "gameData": {"gameTime": 1400.0, "mapNumber": 11},
            "activePlayer": {"currentGold": 500.0, "riotId": "Fulano#BR1"},
            "allPlayers": [{"rawChampionName": "game_character_displayname_Lulu",
                            "team": "ORDER", "riotId": "Fulano#BR1", "items": []}],
            "events": {"Events": [{"EventID": 1, "EventName": "InhibKilled", "EventTime": 1300.0,
                                   "InhibKilled": "Barracks_T2_C1", "KillerName": "Fulano"}]}
        });
        let line = live::sanitize(raw);
        let text = line.to_string();
        assert!(!text.contains("Fulano"));
        let g = live::parse_clean(&line).unwrap();
        assert_eq!(g.events[0].target.as_deref(), Some("Barracks_T2_C1"));
        assert_eq!(g.players[0].alias, "Lulu");
        assert_eq!(
            describe_barracks("Barracks_T2_C1"),
            "inibidor vermelho do meio"
        );
        assert!(describe_barracks("Barracks_T200_C1").starts_with("formato desconhecido"));
        let mm = hud::minimap(1920.0, 1080.0, &HudConfig::default());
        assert_eq!(overlay::inhib_timers(&g, Team::Order, &mm).len(), 1);
    }
}
