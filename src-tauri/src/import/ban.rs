//! Ban no clique: o jogador clica num campeão da sugestão de ban **na própria vez de banir** e o
//! app marca e trava esse ban no cliente. Única ação de seleção que o app faz, e só a pedido
//! (clique); nada de pick, hover de pick, aceitar fila ou trocar posição.
//!
//! A sessão é lida de novo na hora do clique: o ban só sai se a ação em andamento é um ban do
//! próprio `localPlayerCellId` e se o campeão não foi banido, escolhido nem declarado.

use crate::champselect;
use crate::lcu::Lcu;
use serde_json::{Value, json};

/// Caminho e corpo do PATCH que trava o ban, ou o motivo para não banir.
pub fn ban_request(session: &Value, champion_id: i64) -> Result<(String, Value), String> {
    if champion_id <= 0 {
        return Err("campeão inválido".into());
    }
    let action =
        champselect::my_ban_action(session).ok_or("não é a sua vez de banir no cliente")?;
    let draft = champselect::parse_draft(session).unwrap_or_default();
    if draft.unavailable.contains(&champion_id) {
        return Err("esse campeão já foi banido, escolhido ou declarado".into());
    }
    Ok((
        format!("/lol-champ-select/v1/session/actions/{action}"),
        json!({"championId": champion_id, "completed": true}),
    ))
}

pub async fn apply(lcu: &Lcu, champion_id: i64) -> Result<(), String> {
    let session = lcu
        .get("/lol-champ-select/v1/session")
        .await
        .map_err(|_| "fora da seleção de campeões".to_string())?;
    let (path, body) = ban_request(&session, champion_id)?;
    lcu.patch(&path, &body).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Você (célula 2) com o ban em andamento; um aliado declarou o Lux (99); o outro time baniu 238.
    fn session() -> Value {
        json!({
            "localPlayerCellId": 2,
            "myTeam": [
                {"cellId": 1, "championId": 0, "championPickIntent": 99},
                {"cellId": 2, "championId": 0, "championPickIntent": 31}
            ],
            "bans": {"myTeamBans": [], "theirTeamBans": [238]},
            "actions": [[
                {"id": 7, "actorCellId": 1, "type": "ban", "isInProgress": true, "completed": false, "championId": 0},
                {"id": 8, "actorCellId": 2, "type": "ban", "isInProgress": true, "completed": false, "championId": 0}
            ], [
                {"id": 12, "actorCellId": 2, "type": "pick", "isInProgress": false, "completed": false, "championId": 0}
            ]]
        })
    }

    #[test]
    fn bans_on_my_turn() {
        let (path, body) = ban_request(&session(), 234).unwrap();
        assert_eq!(path, "/lol-champ-select/v1/session/actions/8");
        assert_eq!(body, json!({"championId": 234, "completed": true}));
        assert!(crate::lcu::write_allowed("PATCH", &path));
    }

    #[test]
    fn refuses_outside_my_ban_turn() {
        let mut s = session();
        s["actions"][0][1]["isInProgress"] = json!(false);
        assert!(ban_request(&s, 234).is_err(), "vez de outro jogador");
        let mut s = session();
        s["actions"][0][1]["completed"] = json!(true);
        assert!(ban_request(&s, 234).is_err(), "já baniu");
        // a vez de escolher nunca vira ban nem pick
        let mut s = session();
        s["actions"][0][1]["isInProgress"] = json!(false);
        s["actions"][1][0]["isInProgress"] = json!(true);
        assert!(ban_request(&s, 234).is_err());
        assert!(ban_request(&json!({}), 234).is_err());
    }

    #[test]
    fn refuses_unavailable_champions() {
        assert!(ban_request(&session(), 238).is_err(), "já banido");
        assert!(ban_request(&session(), 99).is_err(), "declarado por aliado");
        assert!(ban_request(&session(), 31).is_err(), "o seu");
        assert!(ban_request(&session(), 0).is_err());
    }
}
