//! Página de runas: uma só, com prefixo [`PREFIX`]. Nunca apaga páginas do usuário,
//! a não ser que ele ligue "substituir a página atual quando não houver espaço".

use crate::assets::Assets;
use crate::lcu::Lcu;
use crate::lola::model::RuneSet;
use crate::own_runes::OwnPage;
use serde_json::{Value, json};

pub const PREFIX: &str = "LC:";
/// O cliente corta nomes longos; mantemos com folga.
const MAX_NAME: usize = 25;

pub fn page_name(champion: &str, lane_label: &str) -> String {
    let full = format!("{PREFIX} {champion} {lane_label}");
    full.chars()
        .take(MAX_NAME)
        .collect::<String>()
        .trim_end()
        .to_string()
}

pub fn page_body(name: &str, set: &RuneSet, assets: &Assets) -> Result<Value, String> {
    if !set.is_complete() {
        return Err("página de runas incompleta no lolalytics".into());
    }
    let primary = assets
        .style_of_perk(set.primary[0], None)
        .ok_or("árvore primária desconhecida (cliente desatualizado?)")?;
    let sub = assets
        .secondary_style(&set.secondary, primary)
        .ok_or("árvore secundária desconhecida")?;
    Ok(body(name, primary, sub, set.selected_perk_ids()))
}

/// A página própria do jogador (`own_runes`): as árvores vêm salvas junto, sem deduzir.
pub fn own_page_body(name: &str, page: &OwnPage, assets: &Assets) -> Result<Value, String> {
    page.check(assets)?;
    Ok(body(
        name,
        page.primary_style,
        page.sub_style,
        page.perks.clone(),
    ))
}

fn body(name: &str, primary: i64, sub: i64, perks: Vec<i64>) -> Value {
    json!({
        "name": name,
        "primaryStyleId": primary,
        "subStyleId": sub,
        "selectedPerkIds": perks,
        "current": true,
    })
}

#[derive(Debug, PartialEq)]
pub enum Plan {
    /// Apaga a nossa página antiga (id) e cria de novo.
    ReplaceOurs(i64),
    Create,
    /// Sem espaço e o usuário permitiu: apaga a página atual (id) e cria.
    ReplaceCurrent(i64),
    NoRoom,
}

fn is_ours(p: &Value) -> bool {
    p.get("name")
        .and_then(Value::as_str)
        .is_some_and(|n| n.starts_with(PREFIX))
        && p.get("isDeletable")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

pub fn plan(pages: &Value, inventory: &Value, replace_current: bool) -> Plan {
    let list = pages.as_array().cloned().unwrap_or_default();
    if let Some(id) = list
        .iter()
        .find(|p| is_ours(p))
        .and_then(|p| p.get("id")?.as_i64())
    {
        return Plan::ReplaceOurs(id);
    }
    if inventory
        .get("canAddCustomPage")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Plan::Create;
    }
    if replace_current
        && let Some(id) = list
            .iter()
            .find(|p| {
                p.get("current").and_then(Value::as_bool).unwrap_or(false)
                    && p.get("isDeletable")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
            })
            .and_then(|p| p.get("id")?.as_i64())
    {
        return Plan::ReplaceCurrent(id);
    }
    Plan::NoRoom
}

/// A nossa página já é igual a `body`? Devolve (id, é a atual).
/// Evita apagar e recriar a página a cada clique na janela.
pub fn identical_page(pages: &Value, body: &Value) -> Option<(i64, bool)> {
    let same = |p: &Value, k: &str| p.get(k).is_some() && p.get(k) == body.get(k);
    let page = pages.as_array()?.iter().find(|p| {
        is_ours(p)
            && same(p, "name")
            && same(p, "primaryStyleId")
            && same(p, "subStyleId")
            && same(p, "selectedPerkIds")
    })?;
    let current = page
        .get("current")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Some((page.get("id")?.as_i64()?, current))
}

pub async fn apply(lcu: &Lcu, body: &Value, replace_current: bool) -> Result<String, String> {
    let pages = lcu
        .get("/lol-perks/v1/pages")
        .await
        .map_err(|e| e.to_string())?;
    if let Some((id, current)) = identical_page(&pages, body) {
        if !current {
            lcu.put("/lol-perks/v1/currentpage", &json!(id))
                .await
                .map_err(|e| e.to_string())?;
        }
        return Ok("página já estava aplicada".to_string());
    }
    let inventory = lcu
        .get("/lol-perks/v1/inventory")
        .await
        .map_err(|e| e.to_string())?;
    let msg = match plan(&pages, &inventory, replace_current) {
        Plan::ReplaceOurs(id) => {
            lcu.delete(&format!("/lol-perks/v1/pages/{id}")).await.map_err(|e| e.to_string())?;
            "página atualizada"
        }
        Plan::Create => "página criada",
        Plan::ReplaceCurrent(id) => {
            lcu.delete(&format!("/lol-perks/v1/pages/{id}")).await.map_err(|e| e.to_string())?;
            "página atual substituída"
        }
        Plan::NoRoom => {
            return Err("sem espaço para página de runas: apague uma página ou ative \"substituir a página atual\"".into())
        }
    };
    let created = lcu
        .post("/lol-perks/v1/pages", body)
        .await
        .map_err(|e| e.to_string())?;
    // `current: true` já seleciona; garantimos para clientes que ignoram o campo.
    if let Some(id) = created.get("id").and_then(Value::as_i64) {
        let _ = lcu.put("/lol-perks/v1/currentpage", &json!(id)).await;
    }
    Ok(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::sample_assets;
    use crate::lola::model::{parse_build_full, tests::fixture};

    #[test]
    fn body_from_fixture() {
        let b = parse_build_full(&fixture()).unwrap();
        let body = page_body(
            "LC: Cho'Gath Selva",
            b.runes_pick.as_ref().unwrap(),
            &sample_assets(),
        )
        .unwrap();
        assert_eq!(body["primaryStyleId"], 8100);
        assert_eq!(body["subStyleId"], 8200);
        assert_eq!(body["selectedPerkIds"].as_array().unwrap().len(), 9);
        let win = page_body("x", b.runes_win.as_ref().unwrap(), &sample_assets()).unwrap();
        assert_eq!(win["subStyleId"], 8400);
    }

    #[test]
    fn own_body_keeps_saved_styles() {
        let own = OwnPage {
            primary_style: 8100,
            sub_style: 8200,
            perks: vec![9923, 8126, 8140, 8106, 8224, 8234, 5008, 5010, 5011],
            active: true,
            saved_at: 0,
        };
        let body = own_page_body("LC: Cho'Gath Minha", &own, &sample_assets()).unwrap();
        assert_eq!(body["primaryStyleId"], 8100);
        assert_eq!(body["subStyleId"], 8200);
        assert_eq!(body["selectedPerkIds"], json!(own.perks));
        assert_eq!(body["current"], true);
        let stale = OwnPage {
            perks: vec![1, 2, 3, 4, 5, 6, 7, 8, 9],
            ..own
        };
        assert!(own_page_body("x", &stale, &sample_assets()).is_err());
    }

    #[test]
    fn name_is_short() {
        assert_eq!(page_name("Cho'Gath", "Selva"), "LC: Cho'Gath Selva");
        assert!(
            page_name("Nunu e Willump", "Suporte inválido longo")
                .chars()
                .count()
                <= MAX_NAME
        );
    }

    #[test]
    fn identical_page_skips_rewrite() {
        let body = json!({"name": "LC: Ahri Meio", "primaryStyleId": 8100, "subStyleId": 8200,
            "selectedPerkIds": [1, 2, 3], "current": true});
        let page = |perks: Value, current: bool| {
            json!([{"id": 7, "name": "LC: Ahri Meio", "primaryStyleId": 8100, "subStyleId": 8200,
                "selectedPerkIds": perks, "current": current, "isDeletable": true}])
        };
        assert_eq!(
            identical_page(&page(json!([1, 2, 3]), true), &body),
            Some((7, true))
        );
        assert_eq!(
            identical_page(&page(json!([1, 2, 3]), false), &body),
            Some((7, false))
        );
        assert_eq!(identical_page(&page(json!([1, 2, 9]), true), &body), None);
        // página do usuário com as mesmas runas não conta como nossa
        let mine = json!([{"id": 8, "name": "LC: Ahri Meio", "primaryStyleId": 8100, "subStyleId": 8200,
            "selectedPerkIds": [1, 2, 3], "current": true, "isDeletable": false}]);
        assert_eq!(identical_page(&mine, &body), None);
    }

    #[test]
    fn plans() {
        let pages = json!([
            {"id": 1, "name": "Minha", "current": true, "isDeletable": true},
            {"id": 2, "name": "LC: Ahri Meio", "current": false, "isDeletable": true},
            {"id": 3, "name": "Predefinida", "current": false, "isDeletable": false}
        ]);
        let full = json!({"canAddCustomPage": false});
        let room = json!({"canAddCustomPage": true});
        assert_eq!(plan(&pages, &full, false), Plan::ReplaceOurs(2));
        let mine = json!([{"id": 1, "name": "Minha", "current": true, "isDeletable": true}]);
        assert_eq!(plan(&mine, &room, false), Plan::Create);
        assert_eq!(plan(&mine, &full, false), Plan::NoRoom);
        assert_eq!(plan(&mine, &full, true), Plan::ReplaceCurrent(1));
    }
}
