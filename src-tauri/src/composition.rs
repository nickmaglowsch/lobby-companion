//! Composição de dano dos dois times e itens defensivos do build em tela (só exibição).
//!
//! O tipo de dano vem do próprio cliente, sem lolalytics: `champion-summary.json` não traz,
//! mas `/lol-game-data/assets/v1/champions/{id}.json` tem `tacticalInfo.damageType`
//! (`kPhysical`, `kMagic`, `kMixed`; conferido em 2026-11-24, patch 16.23). Cada arquivo tem
//! 35–95 KB, então buscamos sob demanda só os campeões da seleção, com cache em memória.
//! Se um item dá armadura/resistência mágica sai do campo estruturado `categories` do
//! `items.json` ("Armor"; "SpellBlock" ou "MagicResist").

use crate::assets::Assets;
use crate::import::itemset::pct;
use crate::lcu::Lcu;
use crate::lola::model::Build;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DamageType {
    Physical,
    Magic,
    Mixed,
}

/// `tacticalInfo.damageType` do arquivo do campeão.
pub fn parse_damage_type(champion: &Value) -> Option<DamageType> {
    match champion.pointer("/tacticalInfo/damageType")?.as_str()? {
        "kPhysical" => Some(DamageType::Physical),
        "kMagic" => Some(DamageType::Magic),
        "kMixed" => Some(DamageType::Mixed),
        _ => None,
    }
}

/// O tipo de dano não muda durante a sessão do app: guardamos o que já foi lido.
static CACHE: LazyLock<Mutex<HashMap<i64, DamageType>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Tipo de dano de cada campeão (só GET; no máximo os 10 da seleção, os do cache não repetem).
pub async fn damage_types(lcu: Option<&Lcu>, ids: &[i64]) -> HashMap<i64, DamageType> {
    let ids: Vec<i64> = ids
        .iter()
        .copied()
        .filter(|id| (1..=9999).contains(id))
        .take(10)
        .collect();
    let missing: Vec<i64> = {
        let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        ids.iter()
            .copied()
            .filter(|id| !cache.contains_key(id))
            .collect()
    };
    if let Some(lcu) = lcu {
        for id in missing {
            let path = format!("/lol-game-data/assets/v1/champions/{id}.json");
            if let Some(t) = lcu
                .get(&path)
                .await
                .ok()
                .as_ref()
                .and_then(parse_damage_type)
            {
                CACHE
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(id, t);
            }
        }
    }
    let cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    ids.iter()
        .filter_map(|id| cache.get(id).map(|t| (*id, *t)))
        .collect()
}

/// Quantos campeões de cada tipo num time.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamMix {
    pub physical: u32,
    pub magic: u32,
    pub mixed: u32,
    /// Sem tipo conhecido (cliente não respondeu).
    pub unknown: u32,
    /// Parte física do dano do time, 0–1 (misto conta meio para cada lado).
    pub physical_share: f64,
    pub magic_share: f64,
}

pub fn team_mix(types: &[Option<DamageType>]) -> TeamMix {
    let mut m = TeamMix::default();
    for t in types {
        match t {
            Some(DamageType::Physical) => m.physical += 1,
            Some(DamageType::Magic) => m.magic += 1,
            Some(DamageType::Mixed) => m.mixed += 1,
            None => m.unknown += 1,
        }
    }
    let known = f64::from(m.physical + m.magic + m.mixed);
    if known > 0.0 {
        let half = f64::from(m.mixed) / 2.0;
        m.physical_share = (f64::from(m.physical) + half) / known;
        m.magic_share = (f64::from(m.magic) + half) / known;
    }
    m
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Defense {
    Armor,
    MagicResist,
}

/// A partir de quantos campeões inimigos conhecidos vale sugerir.
const MIN_KNOWN: u32 = 3;
/// Parte do dano inimigo de um tipo para destacar a defesa contra ele.
const SHARE: f64 = 0.6;

/// Time inimigo com ≥ 60% do dano de um tipo (e pelo menos 3 campeões conhecidos)?
pub fn defense_need(enemies: &TeamMix) -> Option<Defense> {
    if enemies.physical + enemies.magic + enemies.mixed < MIN_KNOWN {
        return None;
    }
    if enemies.physical_share >= SHARE {
        Some(Defense::Armor)
    } else if enemies.magic_share >= SHARE {
        Some(Defense::MagicResist)
    } else {
        None
    }
}

/// O item dá essa defesa? (categorias do `items.json`: "Armor"; "SpellBlock"/"MagicResist").
pub fn gives(assets: &Assets, id: i64, need: Defense) -> bool {
    match need {
        Defense::Armor => assets.item_has(id, "Armor"),
        Defense::MagicResist => {
            assets.item_has(id, "SpellBlock") || assets.item_has(id, "MagicResist")
        }
    }
}

/// Item do build que dá a defesa pedida, com o WR que o lolalytics mostra nessa posição.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DefItem {
    pub id: i64,
    pub wr: f64,
    pub pr: f64,
    pub n: u64,
    /// "Botas", "1º item"…
    pub slot: String,
}

/// Até 3 itens por posição (botas e slots) que dão a defesa, com amostra mínima; cada item
/// aparece uma vez (na primeira posição em que surge).
pub fn defensive_items(
    build: &Build,
    assets: &Assets,
    need: Defense,
    min_games: u64,
) -> Vec<DefItem> {
    let slots = std::iter::once(("Botas".to_string(), &build.boots)).chain(
        build
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| (format!("{}º item", i + 1), s)),
    );
    let mut out: Vec<DefItem> = Vec::new();
    for (label, list) in slots {
        for it in list
            .iter()
            .filter(|i| i.n >= min_games && gives(assets, i.id, need))
            .take(3)
        {
            if out.iter().any(|d| d.id == it.id) {
                continue;
            }
            out.push(DefItem {
                id: it.id,
                wr: it.wr,
                pr: it.pr,
                n: it.n,
                slot: label.clone(),
            });
        }
    }
    out
}

/// O que o cartão "Composição" mostra.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Composition {
    pub allies: TeamMix,
    pub enemies: TeamMix,
    /// Campeão → tipo de dano (para os ícones).
    pub types: HashMap<i64, DamageType>,
    pub need: Option<Defense>,
    pub items: Vec<DefItem>,
    /// Ex.: "Time inimigo: 4 físicos, 1 mágico → armadura · WR 55,2% (Botas Galvanizadas de Aço)".
    pub note: Option<String>,
}

pub fn summarize(
    allies: &[i64],
    enemies: &[i64],
    types: &HashMap<i64, DamageType>,
    build: Option<&Build>,
    assets: Option<&Assets>,
    min_games: u64,
) -> Composition {
    let mix = |ids: &[i64]| {
        team_mix(
            &ids.iter()
                .map(|id| types.get(id).copied())
                .collect::<Vec<_>>(),
        )
    };
    let enemies_mix = mix(enemies);
    let need = defense_need(&enemies_mix);
    let items = match (need, build, assets) {
        (Some(need), Some(b), Some(a)) => defensive_items(b, a, need, min_games),
        _ => Vec::new(),
    };
    let note = need.map(|n| {
        let what = match n {
            Defense::Armor => "armadura",
            Defense::MagicResist => "resistência mágica",
        };
        match (items.first(), assets) {
            (Some(d), Some(a)) => format!(
                "Dano inimigo concentrado: {what} ajuda. No seu build, {} tem WR {}.",
                a.item_name(d.id),
                pct(d.wr)
            ),
            _ => format!("Dano inimigo concentrado: {what} ajuda."),
        }
    });
    Composition {
        allies: mix(allies),
        enemies: enemies_mix,
        types: allies
            .iter()
            .chain(enemies)
            .filter_map(|id| types.get(id).map(|t| (*id, *t)))
            .collect(),
        need,
        items,
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::sample_assets;
    use crate::lola::model::{ItemStat, parse_build_full, tests::fixture};
    use DamageType::*;
    use serde_json::json;

    #[test]
    fn parses_damage_type() {
        let c = json!({"id": 31, "tacticalInfo": {"style": 7, "damageType": "kMagic"}});
        assert_eq!(parse_damage_type(&c), Some(Magic));
        assert_eq!(
            parse_damage_type(&json!({"tacticalInfo": {"damageType": "kMixed"}})),
            Some(Mixed)
        );
        assert_eq!(parse_damage_type(&json!({"tacticalInfo": {}})), None);
    }

    #[test]
    fn mix_counts_mixed_as_half() {
        let m = team_mix(&[
            Some(Physical),
            Some(Physical),
            Some(Mixed),
            Some(Magic),
            None,
        ]);
        assert_eq!((m.physical, m.magic, m.mixed, m.unknown), (2, 1, 1, 1));
        assert!((m.physical_share - 0.625).abs() < 1e-9);
        assert!((m.magic_share - 0.375).abs() < 1e-9);
    }

    #[test]
    fn defense_rule() {
        let phys = team_mix(&[
            Some(Physical),
            Some(Physical),
            Some(Physical),
            Some(Magic),
            Some(Magic),
        ]);
        assert_eq!(defense_need(&phys), Some(Defense::Armor)); // 60%
        let magic = team_mix(&[Some(Magic), Some(Magic), Some(Mixed), Some(Physical)]);
        assert_eq!(defense_need(&magic), Some(Defense::MagicResist)); // 62,5%
        let even = team_mix(&[
            Some(Magic),
            Some(Magic),
            Some(Physical),
            Some(Physical),
            Some(Mixed),
        ]);
        assert_eq!(defense_need(&even), None);
        // poucos campeões conhecidos: ainda não dá para dizer
        let few = team_mix(&[Some(Physical), Some(Physical), None]);
        assert_eq!(defense_need(&few), None);
    }

    #[test]
    fn highlights_defensive_items_of_the_build() {
        let mut b = parse_build_full(&fixture()).unwrap();
        b.boots.push(ItemStat {
            id: 3047,
            wr: 55.2,
            pr: 30.0,
            n: 3000,
        });
        b.slots[2] = vec![
            ItemStat {
                id: 3075,
                wr: 59.0,
                pr: 20.0,
                n: 900,
            },
            ItemStat {
                id: 3742,
                wr: 60.0,
                pr: 1.0,
                n: 50, // abaixo do mínimo
            },
        ];
        let a = sample_assets();
        let armor: Vec<(i64, String)> = defensive_items(&b, &a, Defense::Armor, 200)
            .into_iter()
            .map(|d| (d.id, d.slot))
            .collect();
        // 3009 (Botas da Rapidez) não dá armadura; 3742 aparece no 2º item (6900 partidas)
        assert_eq!(
            armor,
            vec![
                (3047, "Botas".to_string()),
                (3742, "2º item".to_string()),
                (3075, "3º item".to_string())
            ]
        );
        assert!(defensive_items(&b, &a, Defense::MagicResist, 200).is_empty());

        let types: HashMap<i64, DamageType> = [
            (1, Physical),
            (2, Physical),
            (3, Physical),
            (4, Magic),
            (5, Magic),
            (31, Magic),
        ]
        .into_iter()
        .collect();
        let c = summarize(&[31], &[1, 2, 3, 4, 5], &types, Some(&b), Some(&a), 200);
        assert_eq!(c.need, Some(Defense::Armor));
        assert_eq!(c.allies.magic, 1);
        assert_eq!(c.items.len(), 3);
        assert_eq!(c.types.len(), 6);
        assert!(
            c.note
                .unwrap()
                .contains("Botas Galvanizadas de Aço tem WR 55,2%")
        );
        // sem build (busca fora da seleção): só a contagem
        let c = summarize(&[], &[1, 2, 3], &types, None, None, 200);
        assert_eq!(c.need, Some(Defense::Armor));
        assert!(c.items.is_empty());
    }
}
