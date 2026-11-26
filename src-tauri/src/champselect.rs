//! Leitura da sessão da seleção de campeões.
//! Do próprio jogador: campeão, posição, feitiços. Dos outros: **só campeões e posições**
//! (o que o próprio cliente mostra na tela); nunca nomes, Riot IDs ou PUUIDs (regra da Riot).

use crate::i18n::Lang;
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

impl Me {
    /// Campeão do jogador agora: o escolhido ou, antes disso, o do hover.
    pub fn current_champion(&self) -> i64 {
        if self.champion_id > 0 {
            self.champion_id
        } else {
            self.hover_id
        }
    }
}

/// Uma mudança feita na janela pode ir sozinha para o cliente?
/// Nunca durante a partida; na seleção, só se a janela mostra o campeão do próprio jogador
/// (olhar o build de outro campeão não pode trocar as runas de quem já escolheu).
pub fn may_apply(phase: &str, in_select: bool, me: Option<&Me>, viewed: i64) -> bool {
    if viewed <= 0 || matches!(phase, "GameStart" | "InProgress" | "Reconnect") {
        return false;
    }
    !in_select || me.is_some_and(|m| m.current_champion() == viewed)
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

/// Um campeão do seu time (escolhido ou declarado) e a posição dele.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ally {
    pub champion_id: i64,
    /// Lane do lolalytics (`None` em às cegas).
    pub lane: Option<String>,
    pub me: bool,
}

/// O que a tela da seleção mostra dos dois times (sem nomes).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Draft {
    pub allies: Vec<Ally>,
    /// Campeões já visíveis do outro time.
    pub enemies: Vec<i64>,
    /// Banidos, escolhidos ou declarados por alguém: não faz sentido sugerir ban.
    pub unavailable: Vec<i64>,
    /// Ainda há ban por fazer (ou a fase de declarar, antes dos bans): a janela mostra os bans.
    pub banning: bool,
}

pub fn parse_draft(session: &Value) -> Option<Draft> {
    let cell = session.get("localPlayerCellId")?.as_i64()?;
    let team = |k: &str| -> Vec<Value> {
        session
            .get(k)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let mut unavailable: Vec<i64> = Vec::new();
    let mut allies = Vec::new();
    for m in team("myTeam") {
        let champ = i(m.get("championId"));
        let shown = if champ > 0 {
            champ
        } else {
            i(m.get("championPickIntent"))
        };
        if shown <= 0 {
            continue;
        }
        unavailable.push(shown);
        allies.push(Ally {
            champion_id: shown,
            lane: m
                .get("assignedPosition")
                .and_then(Value::as_str)
                .map(str::to_ascii_lowercase)
                .as_deref()
                .and_then(lolalytics_lane)
                .map(String::from),
            me: i(m.get("cellId")) == cell,
        });
    }
    let enemies: Vec<i64> = team("theirTeam")
        .iter()
        .map(|m| i(m.get("championId")))
        .filter(|c| *c > 0)
        .collect();
    unavailable.extend(&enemies);
    for k in ["myTeamBans", "theirTeamBans"] {
        unavailable.extend(
            session
                .pointer(&format!("/bans/{k}"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_i64),
        );
    }
    unavailable.extend(
        session
            .get("actions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_array)
            .flatten()
            .filter(|a| a.get("completed").and_then(Value::as_bool) == Some(true))
            .map(|a| i(a.get("championId"))),
    );
    unavailable.retain(|c| *c > 0);
    unavailable.sort_unstable();
    unavailable.dedup();
    let banning = session
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .flatten()
        .any(|a| {
            a.get("type").and_then(Value::as_str) == Some("ban")
                && a.get("completed").and_then(Value::as_bool) != Some(true)
        });
    Some(Draft {
        allies,
        enemies,
        unavailable,
        banning,
    })
}

/// Id da sua ação de ban em andamento (`isInProgress`, não concluída), se for a sua vez de banir.
/// Só `type == "ban"` do `localPlayerCellId`: a vez de escolher nunca entra aqui.
pub fn my_ban_action(session: &Value) -> Option<i64> {
    let cell = session.get("localPlayerCellId")?.as_i64()?;
    session
        .get("actions")?
        .as_array()?
        .iter()
        .filter_map(Value::as_array)
        .flatten()
        .find(|a| {
            a.get("actorCellId").and_then(Value::as_i64) == Some(cell)
                && a.get("type").and_then(Value::as_str) == Some("ban")
                && a.get("isInProgress").and_then(Value::as_bool) == Some(true)
                && a.get("completed").and_then(Value::as_bool) != Some(true)
        })?
        .get("id")?
        .as_i64()
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

/// Lanes do lolalytics.
pub const LANES: &[&str] = &["top", "jungle", "middle", "bottom", "support"];

/// Nome da lane no idioma do app ("jungle" → "Selva"); vazio se a lane não existe.
pub fn lane_label(lang: Lang, lane: &str) -> String {
    if LANES.contains(&lane) {
        lang.t(&format!("lane.{lane}"))
    } else {
        String::new()
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
    fn window_changes_reach_the_client_only_when_safe() {
        let me = parse(&session(true, "BAN_PICK")).unwrap();
        // na seleção: só o próprio campeão
        assert!(may_apply("ChampSelect", true, Some(&me), 31));
        assert!(!may_apply("ChampSelect", true, Some(&me), 62));
        assert!(!may_apply("ChampSelect", true, None, 31));
        // no lobby: o que está na tela vai para o cliente
        assert!(may_apply("Lobby", false, None, 62));
        // durante a partida, nada
        assert!(!may_apply("InProgress", false, None, 31));
        assert!(!may_apply("Lobby", false, None, 0));
        // antes de travar vale o hover
        let hover = Me {
            champion_id: 0,
            hover_id: 62,
            ..me
        };
        assert!(may_apply("ChampSelect", true, Some(&hover), 62));
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
    fn draft_has_champions_and_lanes_only() {
        let mut s = session(true, "BAN_PICK");
        s["myTeam"][0]["championId"] = json!(0);
        s["myTeam"][0]["championPickIntent"] = json!(99);
        s["myTeam"][0]["summonerName"] = json!("não deve aparecer");
        s["theirTeam"] = json!([{"cellId": 5, "championId": 0}, {"cellId": 6, "championId": 64}]);
        s["bans"] = json!({"myTeamBans": [], "theirTeamBans": [238]});
        let d = parse_draft(&s).unwrap();
        assert_eq!(
            d.allies,
            vec![
                Ally {
                    champion_id: 99,
                    lane: Some("top".into()),
                    me: false
                },
                Ally {
                    champion_id: 31,
                    lane: Some("jungle".into()),
                    me: true
                },
            ]
        );
        assert_eq!(d.enemies, vec![64]);
        // ban da ação (1), ban do outro time (238), escolhidos e declarados
        assert_eq!(d.unavailable, vec![1, 31, 64, 99, 238]);
        assert!(!format!("{d:?}").contains("não deve"));
        // o único ban já foi feito
        assert!(!d.banning);
    }

    #[test]
    fn banning_while_a_ban_is_pending() {
        let mut s = session(false, "BAN_PICK");
        s["actions"][0][0]["completed"] = json!(false);
        assert!(parse_draft(&s).unwrap().banning);
        // às cegas: sem ações de ban
        s["actions"] = json!([[{"actorCellId": 2, "type": "pick", "completed": false}]]);
        assert!(!parse_draft(&s).unwrap().banning);
    }

    #[test]
    fn aram_without_actions_is_locked() {
        let s = json!({"localPlayerCellId": 0, "myTeam": [{"cellId": 0, "championId": 157, "assignedPosition": ""}], "actions": []});
        let me = parse(&s).unwrap();
        assert!(me.locked);
        assert_eq!(me.position, None);
    }

    #[test]
    fn my_ban_turn_only() {
        let mut s = session(false, "BAN_PICK");
        assert_eq!(my_ban_action(&s), None, "ban já concluído");
        s["actions"][0][0] = json!({"id": 4, "actorCellId": 2, "type": "ban", "isInProgress": true, "completed": false});
        assert_eq!(my_ban_action(&s), Some(4));
        s["actions"][0][0]["actorCellId"] = json!(1);
        assert_eq!(my_ban_action(&s), None, "vez de outro jogador");
        s["actions"][0][0] = json!({"id": 5, "actorCellId": 2, "type": "pick", "isInProgress": true, "completed": false});
        assert_eq!(my_ban_action(&s), None, "vez de escolher não conta");
    }

    #[test]
    fn lanes() {
        assert_eq!(lolalytics_lane("utility"), Some("support"));
        assert_eq!(lolalytics_lane(""), None);
        let pt = Lang { locale: "pt_BR" };
        assert_eq!(lane_label(pt, "jungle"), "Selva");
        assert!(!lane_label(Lang { locale: "ja_JP" }, "jungle").is_empty());
        assert_eq!(lane_label(pt, "aram"), "");
    }
}
