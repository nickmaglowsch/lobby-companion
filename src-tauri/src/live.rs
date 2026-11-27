//! API de dados da partida ao vivo do jogo (`https://127.0.0.1:2999/liveclientdata/…`), base do
//! overlay. Só GET, só durante a partida, certificado autoassinado aceito só para `127.0.0.1`.
//!
//! A resposta traz nomes e Riot IDs de todos os jogadores (`riotId`, `summonerName`, nas mortes
//! `KillerName`/`VictimName`/`Assisters`…). Regra do app: **nunca** ler, mostrar ou gravar nomes
//! de outros jogadores. Por isso tudo aqui passa por listas de campos permitidos: o que não está
//! na lista é descartado (no diagnóstico só aparece o nome do campo, nunca o valor).

use serde_json::{Map, Value};
use std::time::Duration;

pub const BASE: &str = "https://127.0.0.1:2999/liveclientdata";

/// Campos de cada jogador (`allPlayers[]`) que o app pode usar.
const PLAYER_FIELDS: &[&str] = &[
    "championName",
    // "game_character_displayname_Chogath": nome interno do campeão (não é do jogador).
    "rawChampionName",
    "team",
    "position",
    "level",
    "isDead",
    "respawnTimer",
    "isBot",
    "items",
    "scores",
];
/// Campos de cada item de um jogador.
const ITEM_FIELDS: &[&str] = &["itemID", "count", "slot", "price", "consumable", "canUse"];
/// Campos do seu jogador (`activePlayer`): ouro, nível, atributos.
const ACTIVE_FIELDS: &[&str] = &["currentGold", "level", "teamRelativeColors"];
/// Campos dos eventos (`events.Events[]`). Nomes de quem matou ficam de fora.
const EVENT_FIELDS: &[&str] = &[
    "EventID",
    "EventName",
    "EventTime",
    "InhibKilled",
    "InhibRespawned",
    "InhibRespawningSoon",
    "TurretKilled",
    "DragonType",
    "Stolen",
    "AcingTeam",
    "KillStreak",
];

/// Só os campos permitidos do objeto (sem cópia: o resto é descartado).
fn keep(v: Value, allowed: &[&str]) -> Value {
    match v {
        Value::Object(mut obj) => {
            obj.retain(|k, _| allowed.contains(&k.as_str()));
            Value::Object(obj)
        }
        _ => Value::Null,
    }
}

/// Tira de `v` o valor no caminho `ptr`, sem copiar.
fn take(v: &mut Value, ptr: &str) -> Option<Value> {
    v.pointer_mut(ptr).map(Value::take)
}

fn field_names(v: &Value) -> Vec<&str> {
    v.as_object()
        .map(|o| o.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

/// Versão da resposta de `allgamedata` sem nada que identifique jogadores. Consome a resposta:
/// os campos permitidos mudam de lugar sem cópia e o resto é descartado.
pub fn sanitize(mut all: Value) -> Value {
    let players: Vec<Value> = match take(&mut all, "/allPlayers") {
        Some(Value::Array(ps)) => ps
            .into_iter()
            .map(|mut p| {
                let items = take(&mut p, "/items");
                let mut q = keep(p, PLAYER_FIELDS);
                if let (Some(Value::Array(items)), Some(obj)) = (items, q.as_object_mut()) {
                    obj.insert(
                        "items".into(),
                        items.into_iter().map(|i| keep(i, ITEM_FIELDS)).collect(),
                    );
                }
                q
            })
            .collect(),
        _ => Vec::new(),
    };
    let events: Vec<Value> = match take(&mut all, "/events/Events") {
        Some(Value::Array(es)) => es.into_iter().map(|e| keep(e, EVENT_FIELDS)).collect(),
        _ => Vec::new(),
    };
    let mut active = take(&mut all, "/activePlayer").unwrap_or(Value::Null);
    let stats = take(&mut active, "/championStats");
    let abilities = take(&mut active, "/abilities");
    let mut active = keep(active, ACTIVE_FIELDS);
    if let Some(obj) = active.as_object_mut() {
        if let Some(stats) = stats {
            obj.insert("championStats".into(), stats);
        }
        // Só o `id` de cada habilidade ("ChogathQ"): é do campeão, não do jogador. Serve para
        // achar você na lista quando o app não viu a seleção (aberto no meio da partida).
        if let Some(Value::Object(abilities)) = abilities {
            let ids: Map<String, Value> = abilities
                .into_iter()
                .map(|(slot, a)| (slot, keep(a, &["id"])))
                .collect();
            obj.insert("abilities".into(), Value::Object(ids));
        }
    }
    serde_json::json!({
        "gameData": take(&mut all, "/gameData").unwrap_or(Value::Null),
        "activePlayer": active,
        "allPlayers": players,
        "events": events,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    /// Azul (time 100, embaixo à esquerda).
    Order,
    /// Vermelho (time 200).
    Chaos,
}

#[derive(Debug)]
pub struct Player {
    /// Alias do campeão (`rawChampionName` sem o prefixo, ex. "Chogath"); comparar sem caixa.
    pub alias: String,
    pub team: Team,
    /// "TOP" | "JUNGLE" | "MIDDLE" | "BOTTOM" | "UTILITY" | "" (fora da ranqueada pode vir vazio).
    pub position: String,
    /// `(itemID, count)`.
    pub items: Vec<(i64, u32)>,
    pub cs: u32,
}

#[derive(Debug)]
pub struct Event {
    pub name: String,
    pub time: f64,
    /// Estrutura do evento de inibidor (`InhibKilled`/`InhibRespawned`: "Barracks_T1_L1").
    pub target: Option<String>,
}

/// O que o overlay usa da partida (já sem nomes de jogadores).
#[derive(Debug)]
pub struct Game {
    pub time: f64,
    pub map: i64,
    /// Ouro na sua mochila.
    pub gold: f64,
    /// Ids das suas habilidades ("ChogathQ", "ChogathPassive").
    pub ability_ids: Vec<String>,
    pub players: Vec<Player>,
    pub events: Vec<Event>,
}

fn str_of<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

fn alias_of(p: &Value) -> String {
    match str_of(p, "rawChampionName").rsplit_once("displayname_") {
        Some((_, a)) if !a.is_empty() => a,
        _ => str_of(p, "championName"),
    }
    .to_string()
}

fn u32_of(v: Option<&Value>) -> u32 {
    v.and_then(Value::as_u64)
        .and_then(|x| u32::try_from(x).ok())
        .unwrap_or(0)
}

/// Lista dentro da resposta já limpa (vazia se não veio).
fn array(v: &Value) -> &[Value] {
    v.as_array().map_or(&[], Vec::as_slice)
}

/// Lê a resposta de `allgamedata` (passa por `sanitize` antes: nada de nomes).
pub fn parse(all: Value) -> Option<Game> {
    let s = sanitize(all);
    let time = s.pointer("/gameData/gameTime").and_then(Value::as_f64)?;
    let players = array(&s["allPlayers"])
        .iter()
        .filter_map(|p| {
            let team = match p.get("team").and_then(Value::as_str)? {
                "ORDER" => Team::Order,
                "CHAOS" => Team::Chaos,
                _ => return None,
            };
            Some(Player {
                alias: alias_of(p),
                team,
                position: str_of(p, "position").to_ascii_uppercase(),
                items: array(&p["items"])
                    .iter()
                    .filter_map(|i| {
                        let id = i.get("itemID").and_then(Value::as_i64)?;
                        Some((id, u32_of(i.get("count")).max(1)))
                    })
                    .collect(),
                cs: u32_of(p.pointer("/scores/creepScore")),
            })
        })
        .collect();
    let events = array(&s["events"])
        .iter()
        .filter_map(|e| {
            let name = e.get("EventName").and_then(Value::as_str)?;
            Some(Event {
                time: e.get("EventTime").and_then(Value::as_f64).unwrap_or(0.0),
                target: e.get(name).and_then(Value::as_str).map(String::from),
                name: name.to_string(),
            })
        })
        .collect();
    Some(Game {
        time,
        map: s
            .pointer("/gameData/mapNumber")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        gold: s
            .pointer("/activePlayer/currentGold")
            .and_then(Value::as_f64)
            .unwrap_or(0.0),
        ability_ids: s
            .pointer("/activePlayer/abilities")
            .and_then(Value::as_object)
            .map(|a| {
                a.values()
                    .map(|v| str_of(v, "id"))
                    .filter(|id| !id.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default(),
        players,
        events,
    })
}

/// Cliente da API da partida (certificado autoassinado aceito: só 127.0.0.1).
fn builder(timeout: Duration) -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(timeout)
        .build()
}

pub fn client() -> Option<reqwest::Client> {
    builder(Duration::from_secs(2)).ok()
}

/// `allgamedata` (só GET). `None` fora da partida ou na tela de carregamento.
pub async fn fetch(http: &reqwest::Client) -> Option<Value> {
    let r = http.get(format!("{BASE}/allgamedata")).send().await.ok()?;
    if !r.status().is_success() {
        return None;
    }
    r.json().await.ok()
}

/// `cargo run -- --live`: formato da API durante uma partida, sem nomes de jogadores.
pub async fn diag() {
    let http = match builder(Duration::from_secs(5)) {
        Ok(h) => h,
        Err(e) => {
            println!("erro: {e}");
            return;
        }
    };
    let all: Value = match http.get(format!("{BASE}/allgamedata")).send().await {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or(Value::Null),
        Ok(r) => {
            println!("API da partida: HTTP {} (fora de partida?)", r.status());
            return;
        }
        Err(_) => {
            println!("API da partida: fechada (só responde durante a partida)");
            return;
        }
    };
    // Nomes dos campos (sem valores) para conferir o formato.
    println!("campos (raiz): {:?}", field_names(&all));
    if let Some(a) = all.get("activePlayer") {
        println!("campos (activePlayer): {:?}", field_names(a));
    }
    if let Some(p) = all.pointer("/allPlayers/0") {
        println!("campos (allPlayers[]): {:?}", field_names(p));
        if let Some(i) = p.pointer("/items/0") {
            println!("campos (items[]): {:?}", field_names(i));
        }
        if let Some(s) = p.get("scores") {
            println!("campos (scores): {:?}", field_names(s));
        }
    }
    if let Some(es) = all.pointer("/events/Events").and_then(Value::as_array) {
        let mut seen: Vec<String> = Vec::new();
        for e in es {
            let name = e.get("EventName").and_then(Value::as_str).unwrap_or("?");
            let line = format!("{name}: {:?}", field_names(e));
            if !seen.contains(&line) {
                seen.push(line);
            }
        }
        println!("eventos (nome: campos):");
        for l in seen {
            println!("  {l}");
        }
    }
    match serde_json::to_string_pretty(&sanitize(all)) {
        Ok(s) => println!("{s}"),
        Err(e) => println!("erro: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sanitize_drops_every_name() {
        // Formato da documentação da Riot, valores inventados.
        let all = json!({
            "activePlayer": {
                "riotId": "Fulano#BR1", "riotIdGameName": "Fulano", "riotIdTagLine": "BR1",
                "summonerName": "Fulano#BR1", "currentGold": 671.5, "level": 11,
                "championStats": {"attackDamage": 100.0},
                "abilities": {
                    "Q": {"abilityLevel": 5, "displayName": "Ruptura", "id": "ChogathQ",
                          "rawDescription": "x", "rawDisplayName": "y"},
                    "Passive": {"displayName": "Carnivoro", "id": "ChogathPassive"}
                }
            },
            "allPlayers": [{
                "championName": "Cho'Gath", "team": "ORDER", "position": "JUNGLE", "level": 11,
                "isDead": false, "riotId": "Fulano#BR1", "riotIdGameName": "Fulano",
                "summonerName": "Fulano#BR1", "rawChampionName": "game_character_displayname_Chogath",
                "items": [{"itemID": 3068, "count": 1, "slot": 0, "price": 900,
                           "displayName": "Égide Solar", "rawDisplayName": "x"}],
                "scores": {"creepScore": 85, "kills": 8, "deaths": 2, "assists": 1, "wardScore": 3.0}
            }],
            "events": {"Events": [
                {"EventID": 7, "EventName": "ChampionKill", "EventTime": 300.1,
                 "KillerName": "Ciclano", "VictimName": "Beltrano", "Assisters": ["Fulano"]},
                {"EventID": 9, "EventName": "InhibKilled", "EventTime": 1500.0,
                 "InhibKilled": "Barracks_T2_L1", "KillerName": "Ciclano", "Assisters": []}
            ]},
            "gameData": {"gameMode": "CLASSIC", "gameTime": 341.2, "mapNumber": 11}
        });
        let s = sanitize(all.clone());
        let text = s.to_string();
        for name in ["Fulano", "Ciclano", "Beltrano", "BR1"] {
            assert!(!text.contains(name), "vazou {name}: {text}");
        }
        assert_eq!(s["activePlayer"]["currentGold"], json!(671.5));
        assert_eq!(
            s["activePlayer"]["championStats"]["attackDamage"],
            json!(100.0)
        );
        assert_eq!(s["allPlayers"][0]["championName"], json!("Cho'Gath"));
        assert_eq!(
            s["allPlayers"][0]["items"][0],
            json!({"itemID": 3068, "count": 1, "slot": 0, "price": 900})
        );
        assert_eq!(s["allPlayers"][0]["scores"]["creepScore"], json!(85));
        assert_eq!(s["events"][1]["InhibKilled"], json!("Barracks_T2_L1"));
        assert!(s["events"][0].get("KillerName").is_none());
        assert_eq!(s["gameData"]["gameTime"], json!(341.2));
        let g = parse(all).unwrap();
        assert_eq!(g.time, 341.2);
        assert_eq!(g.map, 11);
        assert_eq!(g.gold, 671.5);
        assert_eq!(
            s["activePlayer"]["abilities"]["Q"],
            json!({"id": "ChogathQ"})
        );
        let mut ids = g.ability_ids.clone();
        ids.sort();
        assert_eq!(ids, vec!["ChogathPassive", "ChogathQ"]);
        assert_eq!(g.players.len(), 1);
        let p = &g.players[0];
        assert_eq!(p.alias, "Chogath");
        assert_eq!(p.team, Team::Order);
        assert_eq!(p.position, "JUNGLE");
        assert_eq!(p.items, vec![(3068, 1)]);
        assert_eq!(p.cs, 85);
        assert_eq!(g.events[1].target.as_deref(), Some("Barracks_T2_L1"));
        assert_eq!(g.events[0].target, None);
        // Fora da partida (sem tempo de jogo): nada.
        assert!(parse(json!({})).is_none());
    }
}
