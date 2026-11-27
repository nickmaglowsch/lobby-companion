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

fn keep(v: &Value, allowed: &[&str]) -> Value {
    let Some(obj) = v.as_object() else {
        return Value::Null;
    };
    let out: Map<String, Value> = obj
        .iter()
        .filter(|(k, _)| allowed.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Value::Object(out)
}

fn field_names(v: &Value) -> Vec<String> {
    v.as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

/// Versão da resposta de `allgamedata` sem nada que identifique jogadores.
pub fn sanitize(all: &Value) -> Value {
    let players: Vec<Value> = all
        .get("allPlayers")
        .and_then(Value::as_array)
        .map(|ps| {
            ps.iter()
                .map(|p| {
                    let mut q = keep(p, PLAYER_FIELDS);
                    if let Some(items) = p.get("items").and_then(Value::as_array) {
                        q["items"] = items.iter().map(|i| keep(i, ITEM_FIELDS)).collect();
                    }
                    q
                })
                .collect()
        })
        .unwrap_or_default();
    let events: Vec<Value> = all
        .pointer("/events/Events")
        .and_then(Value::as_array)
        .map(|es| es.iter().map(|e| keep(e, EVENT_FIELDS)).collect())
        .unwrap_or_default();
    let mut active = all
        .get("activePlayer")
        .map(|a| keep(a, ACTIVE_FIELDS))
        .unwrap_or(Value::Null);
    if let (Some(stats), Some(obj)) = (
        all.pointer("/activePlayer/championStats"),
        active.as_object_mut(),
    ) {
        obj.insert("championStats".into(), stats.clone());
    }
    serde_json::json!({
        "gameData": all.get("gameData").cloned().unwrap_or(Value::Null),
        "activePlayer": active,
        "allPlayers": players,
        "events": events,
    })
}

/// `cargo run -- --live`: formato da API durante uma partida, sem nomes de jogadores.
pub async fn diag() {
    let http = match reqwest::Client::builder()
        // Só 127.0.0.1: o certificado do jogo é autoassinado.
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
    {
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
    match serde_json::to_string_pretty(&sanitize(&all)) {
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
                "championStats": {"attackDamage": 100.0}
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
        let s = sanitize(&all);
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
    }
}
