//! Conjunto de itens com o WR de cada item no título do bloco (é o que aparece na loja).
//!
//! O PUT de `/lol-item-sets/v1/item-sets/{summonerId}/sets` substitui TODOS os conjuntos:
//! lemos, trocamos só os nossos (uid `lc-`/título `LC:`) e regravamos o resto intacto.

use crate::assets::Assets;
use crate::lcu::Lcu;
use crate::lola::model::{Build, ItemStat};
use serde_json::{Value, json};

pub const UID_PREFIX: &str = "lc-";
pub const TITLE_PREFIX: &str = "LC:";
const SR_MAP: i64 = 11;
const CONSUMABLES: &[i64] = &[2055, 2003]; // Sentinela de Controle, Poção de Vida

/// `53.456` → `"53,5%"` (pt-BR).
pub fn pct(x: f64) -> String {
    format!("{:.1}%", x).replace('.', ",")
}

fn item(id: i64, count: u32) -> Value {
    json!({"id": id.to_string(), "count": count})
}

/// Até 3 itens mais jogados com amostra mínima (se nenhum alcança, o mais jogado).
fn pick_items(list: &[ItemStat], assets: &Assets, min_games: u64) -> Vec<ItemStat> {
    let known: Vec<&ItemStat> = list
        .iter()
        .filter(|i| assets.items.contains_key(&i.id))
        .collect();
    let mut out: Vec<ItemStat> = known
        .iter()
        .filter(|i| i.n >= min_games)
        .take(3)
        .map(|i| (*i).clone())
        .collect();
    if out.is_empty() {
        out.extend(known.first().map(|i| (*i).clone()));
    }
    out
}

fn wr_block(label: &str, items: &[ItemStat]) -> Option<Value> {
    if items.is_empty() {
        return None;
    }
    // WR na mesma ordem dos ícones do bloco.
    let wrs: Vec<String> = items.iter().map(|i| pct(i.wr)).collect();
    Some(json!({
        "type": format!("{label} · WR {}", wrs.join(" / ")),
        "hideIfSummonerSpell": "",
        "showIfSummonerSpell": "",
        "items": items.iter().map(|i| item(i.id, 1)).collect::<Vec<_>>(),
    }))
}

pub fn build_item_set(
    build: &Build,
    champion_id: i64,
    champion_name: &str,
    lane_label: &str,
    assets: &Assets,
    min_games: u64,
) -> Value {
    let mut blocks: Vec<Value> = Vec::new();
    // Inícios: o mais jogado e uma alternativa.
    for (i, start) in build
        .start_sets
        .iter()
        .filter(|s| s.n >= 1)
        .take(2)
        .enumerate()
    {
        let mut counts: Vec<(i64, u32)> = Vec::new();
        for id in start.ids.iter().filter_map(|s| s.parse::<i64>().ok()) {
            if !assets.items.contains_key(&id) {
                continue;
            }
            match counts.iter_mut().find(|(x, _)| *x == id) {
                Some((_, c)) => *c += 1,
                None => counts.push((id, 1)),
            }
        }
        if counts.is_empty() {
            continue;
        }
        let label = if i == 0 { "Início" } else { "Início alt." };
        blocks.push(json!({
            "type": format!("{label} · WR {}", pct(start.wr)),
            "hideIfSummonerSpell": "",
            "showIfSummonerSpell": "",
            "items": counts.iter().map(|(id, c)| item(*id, *c)).collect::<Vec<_>>(),
        }));
    }
    blocks.extend(wr_block(
        "Botas",
        &pick_items(&build.boots, assets, min_games),
    ));
    for (i, slot) in build.slots.iter().enumerate() {
        blocks.extend(wr_block(
            &format!("{}º item", i + 1),
            &pick_items(slot, assets, min_games),
        ));
    }
    let cons: Vec<Value> = CONSUMABLES
        .iter()
        .filter(|id| assets.items.contains_key(id))
        .map(|id| item(*id, 1))
        .collect();
    if !cons.is_empty() {
        blocks.push(json!({"type": "Consumíveis", "hideIfSummonerSpell": "", "showIfSummonerSpell": "", "items": cons}));
    }
    let patch = if build.patch.is_empty() {
        String::new()
    } else {
        format!(" · {}", build.patch)
    };
    json!({
        "uid": format!("{UID_PREFIX}{champion_id}"),
        "title": format!("{TITLE_PREFIX} {champion_name} {lane_label}{patch}"),
        "mode": "any",
        "map": "any",
        "type": "custom",
        "sortrank": 0,
        "startedFrom": "blank",
        "associatedChampions": [champion_id],
        "associatedMaps": [SR_MAP],
        "preferredItemSlots": [],
        "blocks": blocks,
    })
}

fn is_ours(set: &Value) -> bool {
    set.get("uid")
        .and_then(Value::as_str)
        .is_some_and(|u| u.starts_with(UID_PREFIX))
        || set
            .get("title")
            .and_then(Value::as_str)
            .is_some_and(|t| t.starts_with(TITLE_PREFIX))
}

/// Corpo do PUT: conjuntos do usuário intactos + o nosso novo (os nossos antigos saem).
pub fn merge(existing: &Value, new_set: Value, now_ms: u64) -> Value {
    let mut sets: Vec<Value> = existing
        .get("itemSets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|s| !is_ours(s))
        .cloned()
        .collect();
    sets.push(new_set);
    json!({
        "accountId": existing.get("accountId").cloned().unwrap_or(json!(0)),
        "itemSets": sets,
        "timestamp": now_ms,
    })
}

pub async fn apply(lcu: &Lcu, new_set: Value) -> Result<String, String> {
    let me = lcu
        .get("/lol-summoner/v1/current-summoner")
        .await
        .map_err(|e| e.to_string())?;
    let sid = me
        .get("summonerId")
        .and_then(Value::as_u64)
        .ok_or("summonerId ausente")?;
    let path = format!("/lol-item-sets/v1/item-sets/{sid}/sets");
    let existing = lcu.get(&path).await.map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let n_blocks = new_set
        .get("blocks")
        .and_then(Value::as_array)
        .map_or(0, |b| b.len());
    lcu.put(&path, &merge(&existing, new_set, now))
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!("conjunto gravado ({n_blocks} blocos)"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::sample_assets;
    use crate::lola::model::{parse_build_full, tests::fixture};

    #[test]
    fn pct_ptbr() {
        assert_eq!(pct(53.456), "53,5%");
    }

    #[test]
    fn builds_blocks_with_wr() {
        let b = parse_build_full(&fixture()).unwrap();
        let set = build_item_set(&b, 31, "Cho'Gath", "Selva", &sample_assets(), 200);
        assert_eq!(set["uid"], "lc-31");
        assert_eq!(set["title"], "LC: Cho'Gath Selva · 16.23");
        assert_eq!(set["associatedMaps"], json!([11]));
        let blocks = set["blocks"].as_array().unwrap();
        let titles: Vec<&str> = blocks.iter().map(|b| b["type"].as_str().unwrap()).collect();
        assert_eq!(titles[0], "Início · WR 51,8%");
        assert_eq!(
            blocks[0]["items"],
            json!([{"id": "1102", "count": 1}, {"id": "2003", "count": 1}])
        );
        // botas: 9999 não existe no cliente; 3111 não existe no sample → só 3009
        assert!(titles.contains(&"Botas · WR 55,4%"));
        // 1º item: 3084 (9100 partidas) antes de 3152; 6664 abaixo do mínimo e desconhecido
        assert!(titles.contains(&"1º item · WR 53,5% / 53,2%"));
        assert!(titles.contains(&"Consumíveis"));
    }

    #[test]
    fn merge_keeps_user_sets() {
        let existing = json!({"accountId": 99, "timestamp": 1, "itemSets": [
            {"uid": "abc", "title": "Meu conjunto"},
            {"uid": "lc-31", "title": "LC: Cho'Gath Selva"},
            {"uid": "zzz", "title": "LC: antigo sem uid nosso"}
        ]});
        let out = merge(&existing, json!({"uid": "lc-62", "title": "LC: Wukong"}), 5);
        let sets = out["itemSets"].as_array().unwrap();
        assert_eq!(sets.len(), 2);
        assert_eq!(sets[0]["title"], "Meu conjunto");
        assert_eq!(sets[1]["uid"], "lc-62");
        assert_eq!(out["accountId"], 99);
        assert_eq!(out["timestamp"], 5);
    }
}
