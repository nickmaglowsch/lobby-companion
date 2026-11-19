//! Leitura da sessão da seleção de campeões: só o próprio jogador.
//! Não lemos nomes nem dados de outros jogadores (regra da Riot para a seleção ranqueada).

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub cell_id: i64,
    /// Campeão escolhido (0 = nenhum ainda).
    pub champion_id: i64,
    /// Campeão em destaque (hover) antes de travar.
    pub hover_id: i64,
    /// `assignedPosition` do cliente (top/jungle/middle/bottom/utility); vazio em às cegas/ARAM.
    pub position: Option<String>,
    pub spell1: i64,
    pub spell2: i64,
    pub locked: bool,
}

fn i(v: Option<&Value>) -> i64 {
    v.and_then(Value::as_i64).unwrap_or(0)
}

pub fn parse(session: &Value) -> Option<Me> {
    let cell = session.get("localPlayerCellId")?.as_i64()?;
    let me = session
        .get("myTeam")?
        .as_array()?
        .iter()
        .find(|m| m.get("cellId").and_then(Value::as_i64) == Some(cell))?;
    let champion_id = i(me.get("championId"));
    let position = me
        .get("assignedPosition")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase());
    let my_picks: Vec<&Value> = session
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .flatten()
        .filter(|a| {
            a.get("actorCellId").and_then(Value::as_i64) == Some(cell)
                && a.get("type").and_then(Value::as_str) == Some("pick")
        })
        .collect();
    let phase = session
        .pointer("/timer/phase")
        .and_then(Value::as_str)
        .unwrap_or("");
    let locked = champion_id > 0
        && (my_picks.is_empty() // ARAM/sem ações: campeão atribuído
            || my_picks.iter().any(|a| a.get("completed").and_then(Value::as_bool).unwrap_or(false))
            || matches!(phase, "FINALIZATION" | "GAME_STARTING"));
    Some(Me {
        cell_id: cell,
        champion_id,
        hover_id: i(me.get("championPickIntent")),
        position,
        spell1: i(me.get("spell1Id")),
        spell2: i(me.get("spell2Id")),
        locked,
    })
}

/// Posição do cliente → lane do lolalytics.
pub fn lolalytics_lane(position: &str) -> Option<&'static str> {
    match position {
        "top" => Some("top"),
        "jungle" => Some("jungle"),
        "middle" | "mid" => Some("middle"),
        "bottom" | "bot" | "adc" => Some("bottom"),
        "utility" | "support" => Some("support"),
        _ => None,
    }
}

pub fn lane_label(lane: &str) -> &'static str {
    match lane {
        "top" => "Topo",
        "jungle" => "Selva",
        "middle" => "Meio",
        "bottom" => "Atirador",
        "support" => "Suporte",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn session(completed: bool, phase: &str) -> Value {
        json!({
            "localPlayerCellId": 2,
            "timer": {"phase": phase},
            "myTeam": [
                {"cellId": 1, "championId": 99, "assignedPosition": "top"},
                {"cellId": 2, "championId": 31, "championPickIntent": 0, "assignedPosition": "JUNGLE", "spell1Id": 4, "spell2Id": 11}
            ],
            "actions": [[{"actorCellId": 2, "type": "ban", "completed": true, "championId": 1}],
                        [{"actorCellId": 2, "type": "pick", "completed": completed, "championId": 31}]]
        })
    }

    #[test]
    fn hover_vs_locked() {
        let me = parse(&session(false, "BAN_PICK")).unwrap();
        assert_eq!(me.champion_id, 31);
        assert_eq!(me.position.as_deref(), Some("jungle"));
        assert!(!me.locked);
        assert!(parse(&session(true, "BAN_PICK")).unwrap().locked);
        assert!(parse(&session(false, "FINALIZATION")).unwrap().locked);
    }

    #[test]
    fn aram_without_actions_is_locked() {
        let s = json!({"localPlayerCellId": 0, "myTeam": [{"cellId": 0, "championId": 157, "assignedPosition": ""}], "actions": []});
        let me = parse(&s).unwrap();
        assert!(me.locked);
        assert_eq!(me.position, None);
    }

    #[test]
    fn lanes() {
        assert_eq!(lolalytics_lane("utility"), Some("support"));
        assert_eq!(lolalytics_lane(""), None);
        assert_eq!(lane_label("jungle"), "Selva");
    }
}
