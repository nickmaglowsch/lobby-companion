//! Conjunto de itens com o WR de cada item no título do bloco (é o que aparece na loja).
//!
//! O PUT de `/lol-item-sets/v1/item-sets/{summonerId}/sets` substitui TODOS os conjuntos:
//! lemos, trocamos só os nossos (uid `lc-`/título `LC:`) e regravamos o resto intacto.

use crate::assets::Assets;
use crate::i18n::Lang;
use crate::lcu::Lcu;
use crate::lola::model::{Build, Combo, ItemStat};
use crate::metric::{self, Metric};
use serde_json::{Value, json};
use std::collections::HashMap;

pub const UID_PREFIX: &str = "lc-";
pub const TITLE_PREFIX: &str = "LC:";
const SR_MAP: i64 = 11;
const CONSUMABLES: &[i64] = &[2055, 2003]; // Sentinela de Controle, Poção de Vida
/// Categorias do cliente que não são item de build (poções, sentinelas, berloques, itens de selva).
const NOT_BUILD: &[&str] = &["Consumable", "Trinket", "Jungle"];
/// Quantos itens entram na primeira volta e nos situacionais.
const EXTRA_ITEMS: usize = 4;

/// `0.84` → `"+0,8"`, `-0.4` → `"-0.4"` (uma casa, separador decimal do idioma; hífen comum,
/// que a loja mostra bem).
pub fn pp(lang: Lang, x: f64) -> String {
    let r = (x * 10.0).round() / 10.0;
    let sign = if r < 0.0 { "-" } else { "+" };
    format!("{sign}{:.1}", r.abs()).replace('.', &lang.t("fmt.decimal"))
}

/// WPA (pp) de cada item de uma lista do build (`metric::list_wpa`).
pub fn item_wpa(list: &[ItemStat], champ_wr: f64, one_per_game: bool) -> HashMap<i64, f64> {
    let rows: Vec<(f64, f64, u64)> = list.iter().map(|i| (i.wr, i.pr, i.n)).collect();
    list.iter()
        .map(|i| i.id)
        .zip(metric::list_wpa(&rows, champ_wr, one_per_game))
        .collect()
}

/// WPA (pp) de cada combinação (inícios, núcleos), na ordem da lista: uma por partida.
pub fn combo_wpa(list: &[Combo], champ_wr: f64) -> Vec<f64> {
    let rows: Vec<(f64, f64, u64)> = list.iter().map(|c| (c.wr, c.pr, c.n)).collect();
    metric::list_wpa(&rows, champ_wr, true)
}

/// Texto do título do bloco: `WR 53,5% / 53,2%` ou `+0,8 / -0,4 pp` (unidade traduzida).
fn tag(lang: Lang, metric: Metric, values: &[(f64, f64)]) -> String {
    match metric {
        Metric::Wr => {
            let v: Vec<String> = values.iter().map(|(wr, _)| lang.pct(*wr)).collect();
            format!("WR {}", v.join(" / "))
        }
        Metric::Wpa => {
            let v: Vec<String> = values.iter().map(|(_, w)| pp(lang, *w)).collect();
            format!("{} {}", v.join(" / "), lang.t("metric.pp"))
        }
    }
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

/// Item conhecido do cliente que faz parte de uma build (nada de poção, sentinela, berloque).
pub(crate) fn is_build_item(assets: &Assets, id: i64) -> bool {
    assets.items.contains_key(&id) && !NOT_BUILD.iter().any(|c| assets.item_has(id, c))
}

/// Núcleo (3 itens com botas, ordem de compra) mais jogado com amostra mínima, só com itens conhecidos.
/// Mesma regra na janela (`coreSets` em `api.ts`).
pub fn core_set<'a>(
    build: &'a Build,
    assets: &Assets,
    min_games: u64,
) -> Option<(&'a Combo, Vec<i64>)> {
    build
        .core
        .iter()
        .filter(|c| c.n >= min_games)
        .find_map(|c| {
            let ids: Vec<i64> = c
                .ids
                .iter()
                .filter_map(|s| s.parse().ok())
                .filter(|id| assets.items.contains_key(id))
                .collect();
            (!ids.is_empty()).then_some((c, ids))
        })
}

/// Primeira volta: os itens de `earlyItem` mais comprados com amostra mínima, sem consumíveis,
/// berloques e o que já está nos inícios. Mesma regra na janela (`firstBack` em `api.ts`).
pub fn first_back(build: &Build, assets: &Assets, min_games: u64) -> Vec<ItemStat> {
    let starts: Vec<i64> = build
        .start_sets
        .iter()
        .take(2)
        .flat_map(|s| s.ids.iter().filter_map(|x| x.parse().ok()))
        .collect();
    build
        .early_items
        .iter()
        .filter(|i| i.n >= min_games && !starts.contains(&i.id) && is_build_item(assets, i.id))
        .take(EXTRA_ITEMS)
        .cloned()
        .collect()
}

/// Situacionais: fora dos mais jogados dos slots, das botas e do núcleo (botas ficam de fora: já
/// têm bloco próprio), sem consumíveis.
/// - WR: os de maior WR (`winningItem`) com amostra mínima;
/// - WPA: os de maior WPA contra o WR do campeão (a amostra pequena já pesa pouco, sem corte),
///   só os positivos.
///
/// Mesma regra na janela (`situationalItems` em `api.ts`).
pub fn situational(
    build: &Build,
    assets: &Assets,
    min_games: u64,
    metric: Metric,
) -> Vec<ItemStat> {
    let mut usual: Vec<i64> = build
        .slots
        .iter()
        .chain(std::iter::once(&build.boots))
        .flat_map(|slot| pick_items(slot, assets, min_games))
        .map(|i| i.id)
        .collect();
    if let Some((_, ids)) = core_set(build, assets, min_games) {
        usual.extend(ids);
    }
    let open = |i: &&ItemStat| {
        !usual.contains(&i.id) && is_build_item(assets, i.id) && !assets.item_has(i.id, "Boots")
    };
    match metric {
        Metric::Wr => build
            .winning_items
            .iter()
            .filter(|i| i.n >= min_games)
            .filter(open)
            .take(EXTRA_ITEMS)
            .cloned()
            .collect(),
        Metric::Wpa => {
            let score = item_wpa(&build.winning_items, build.wr, false);
            let s = |i: &ItemStat| score.get(&i.id).copied().unwrap_or(0.0);
            let mut c: Vec<&ItemStat> = build
                .winning_items
                .iter()
                .filter(open)
                .filter(|i| s(i) > 0.0)
                .collect();
            c.sort_by(|a, b| s(b).total_cmp(&s(a)).then(a.id.cmp(&b.id)));
            c.into_iter().take(EXTRA_ITEMS).cloned().collect()
        }
    }
}

/// Bloco com o WR (ou o WPA de `scores`) de cada item, na mesma ordem dos ícones.
fn wr_block(
    lang: Lang,
    label: &str,
    items: &[ItemStat],
    scores: &HashMap<i64, f64>,
    metric: Metric,
) -> Option<Value> {
    if items.is_empty() {
        return None;
    }
    let values: Vec<(f64, f64)> = items
        .iter()
        .map(|i| (i.wr, scores.get(&i.id).copied().unwrap_or(0.0)))
        .collect();
    Some(json!({
        "type": format!("{label} · {}", tag(lang, metric, &values)),
        "hideIfSummonerSpell": "",
        "showIfSummonerSpell": "",
        "items": items.iter().map(|i| item(i.id, 1)).collect::<Vec<_>>(),
    }))
}

#[allow(clippy::too_many_arguments)]
pub fn build_item_set(
    build: &Build,
    champion_id: i64,
    champion_name: &str,
    lane_label: &str,
    assets: &Assets,
    min_games: u64,
    lang: Lang,
    metric: Metric,
) -> Value {
    let mut blocks: Vec<Value> = Vec::new();
    let champ = build.wr;
    // Inícios: o mais jogado e uma alternativa.
    let start_wpa = combo_wpa(&build.start_sets, champ);
    for (i, (start, wpa)) in build
        .start_sets
        .iter()
        .zip(start_wpa)
        .filter(|(s, _)| s.n >= 1)
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
        let label = lang.t(if i == 0 {
            "itemset.start"
        } else {
            "itemset.startAlt"
        });
        blocks.push(json!({
            "type": format!("{label} · {}", tag(lang, metric, &[(start.wr, wpa)])),
            "hideIfSummonerSpell": "",
            "showIfSummonerSpell": "",
            "items": counts.iter().map(|(id, c)| item(*id, *c)).collect::<Vec<_>>(),
        }));
    }
    // Núcleo logo depois dos inícios: é o que a maioria compra primeiro.
    if let Some((core, ids)) = core_set(build, assets, min_games) {
        let wpa = build
            .core
            .iter()
            .position(|c| c.ids == core.ids)
            .and_then(|k| combo_wpa(&build.core, champ).get(k).copied())
            .unwrap_or(0.0);
        blocks.push(json!({
            "type": format!("{} · {}", lang.t("itemset.core"), tag(lang, metric, &[(core.wr, wpa)])),
            "hideIfSummonerSpell": "",
            "showIfSummonerSpell": "",
            "items": ids.iter().map(|id| item(*id, 1)).collect::<Vec<_>>(),
        }));
    }
    // Primeira volta e situacionais: vários por partida, contra o WR do campeão; botas e
    // posições: contra o item típico da posição.
    blocks.extend(wr_block(
        lang,
        &lang.t("itemset.firstBack"),
        &first_back(build, assets, min_games),
        &item_wpa(&build.early_items, champ, false),
        metric,
    ));
    blocks.extend(wr_block(
        lang,
        &lang.t("itemset.boots"),
        &pick_items(&build.boots, assets, min_games),
        &item_wpa(&build.boots, champ, true),
        metric,
    ));
    for (i, slot) in build.slots.iter().enumerate() {
        blocks.extend(wr_block(
            lang,
            &lang.tf("itemset.itemN", &[("n", &(i + 1))]),
            &pick_items(slot, assets, min_games),
            &item_wpa(slot, champ, true),
            metric,
        ));
    }
    blocks.extend(wr_block(
        lang,
        &lang.t("itemset.situational"),
        &situational(build, assets, min_games, metric),
        &item_wpa(&build.winning_items, champ, false),
        metric,
    ));
    let cons: Vec<Value> = CONSUMABLES
        .iter()
        .filter(|id| assets.items.contains_key(id))
        .map(|id| item(*id, 1))
        .collect();
    if !cons.is_empty() {
        blocks.push(json!({"type": lang.t("itemset.consumables"), "hideIfSummonerSpell": "", "showIfSummonerSpell": "", "items": cons}));
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
        .ok_or_else(|| crate::t!("items.noSummoner"))?;
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
    Ok(crate::t!("items.saved", count = n_blocks))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::sample_assets;
    use crate::lola::model::{parse_build_full, tests::fixture};

    const PT: Lang = Lang { locale: "pt_BR" };

    #[test]
    fn block_titles_follow_the_language() {
        let b = parse_build_full(&fixture()).unwrap();
        let en = Lang { locale: "en_US" };
        let set = build_item_set(
            &b,
            31,
            "Cho'Gath",
            "Jungle",
            &sample_assets(),
            200,
            en,
            Metric::Wr,
        );
        let titles: Vec<&str> = set["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["type"].as_str().unwrap())
            .collect();
        assert_eq!(titles[0], "Start · WR 51.8%");
        assert!(titles.contains(&"Item 1 · WR 53.5% / 53.2%"));
        assert!(titles.contains(&"Consumables"));
        // O prefixo técnico não muda com o idioma: é por ele que achamos o nosso conjunto.
        assert_eq!(set["title"], "LC: Cho'Gath Jungle · 16.23");
        assert_eq!(set["uid"], "lc-31");
    }

    #[test]
    fn builds_blocks_with_wr() {
        let b = parse_build_full(&fixture()).unwrap();
        let set = build_item_set(
            &b,
            31,
            "Cho'Gath",
            "Selva",
            &sample_assets(),
            200,
            PT,
            Metric::Wr,
        );
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
    fn core_first_back_and_situational_blocks() {
        let b = parse_build_full(&fixture()).unwrap();
        let assets = sample_assets();
        let set = build_item_set(&b, 31, "Cho'Gath", "Selva", &assets, 200, PT, Metric::Wr);
        let blocks = set["blocks"].as_array().unwrap();
        let titles: Vec<&str> = blocks.iter().map(|b| b["type"].as_str().unwrap()).collect();
        // ordem: inícios, núcleo, primeira volta, botas, itens, situacionais, consumíveis
        assert_eq!(
            titles,
            vec![
                "Início · WR 51,8%",
                "Início alt. · WR 53,6%",
                "Núcleo · WR 57,1%",
                "Primeira volta · WR 54,0% / 54,5% / 58,0%",
                "Botas · WR 55,4%",
                "1º item · WR 53,5% / 53,2%",
                "2º item · WR 55,7%",
                "Situacionais · WR 61,0% / 58,0%",
                "Consumíveis",
            ]
        );
        // núcleo na ordem de compra (3068 › 3047 › 2502)
        assert_eq!(
            blocks[2]["items"],
            json!([{"id": "3068", "count": 1}, {"id": "3047", "count": 1}, {"id": "2502", "count": 1}])
        );
        // primeira volta: sem berloque (3340), sentinela (2055), pet (1102, também no início) e desconhecido
        let fb: Vec<i64> = first_back(&b, &assets, 200).iter().map(|i| i.id).collect();
        assert_eq!(fb, vec![1028, 1001, 3068]);
        // situacionais (por WR): 2003 consumível, 2525 abaixo do mínimo, 3009 botas,
        // 3742 já é o 2º item e 3152 o 1º → sobram 3075 e 3065
        let sit: Vec<i64> = situational(&b, &assets, 200, Metric::Wr)
            .iter()
            .map(|i| i.id)
            .collect();
        assert_eq!(sit, vec![3075, 3065]);
    }

    #[test]
    fn wpa_titles_and_situational() {
        let b = parse_build_full(&fixture()).unwrap();
        let assets = sample_assets();
        let set = build_item_set(&b, 31, "Cho'Gath", "Selva", &assets, 200, PT, Metric::Wpa);
        let titles: Vec<String> = set["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["type"].as_str().unwrap().to_string())
            .collect();
        // mesmos blocos e itens; o número é o WPA em pp contra o típico da posição
        let slot1 = item_wpa(&b.slots[0], b.wr, true);
        let want = format!(
            "1º item · {} / {} pp",
            pp(PT, slot1[&3084]),
            pp(PT, slot1[&3152])
        );
        assert!(titles.contains(&want), "{titles:?}");
        assert!(
            titles
                .iter()
                .all(|t| t == "Consumíveis" || t.ends_with(" pp"))
        );
        // situacionais: só WPA positivo, do maior para o menor, sem os de sempre e sem botas
        let sit = situational(&b, &assets, 200, Metric::Wpa);
        let score = item_wpa(&b.winning_items, b.wr, false);
        assert!(!sit.is_empty());
        assert!(sit.iter().all(|i| score[&i.id] > 0.0));
        assert!(sit.windows(2).all(|w| score[&w[0].id] >= score[&w[1].id]));
        assert!(sit.iter().all(|i| ![3152, 3742, 3009].contains(&i.id)));
        assert_eq!(pp(PT, -0.04), "+0,0");
        assert_eq!(pp(PT, -0.36), "-0,4");
    }

    #[test]
    fn core_needs_min_games() {
        let b = parse_build_full(&fixture()).unwrap();
        let assets = sample_assets();
        // 1300 partidas no núcleo mais jogado: com corte 2000 ele não entra
        assert!(core_set(&b, &assets, 2000).is_none());
        let (c, ids) = core_set(&b, &assets, 1000).unwrap();
        assert_eq!((c.n, ids), (1300, vec![3068, 3047, 2502]));
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
