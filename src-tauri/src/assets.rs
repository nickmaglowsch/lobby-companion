//! Dados estáticos lidos do próprio cliente (nomes em pt-BR, ícones), sem Data Dragon.

use crate::lcu::{Lcu, LcuError};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct Champion {
    pub id: i64,
    pub name: String,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Named {
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Style {
    pub id: i64,
    pub name: String,
    pub icon: String,
    /// Runas da árvore (keystones + fileiras normais), sem os fragmentos.
    pub perks: Vec<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Assets {
    pub champions: HashMap<i64, Champion>,
    pub items: HashMap<i64, Named>,
    pub perks: HashMap<i64, Named>,
    pub styles: Vec<Style>,
    pub spells: HashMap<i64, Named>,
}

fn s(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

fn named_map(list: &Value) -> HashMap<i64, Named> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            let id = e.get("id")?.as_i64()?;
            Some((
                id,
                Named {
                    name: s(e, "name"),
                    icon: s(e, "iconPath"),
                },
            ))
        })
        .collect()
}

/// Slug do campeão no lolalytics: alias em minúsculas só com letras/dígitos.
/// Conferido em 2026-11-19 contra os 173 campeões do cliente; a única exceção é o Wukong.
pub fn lolalytics_slug(alias: &str) -> String {
    let slug: String = alias
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    match slug.as_str() {
        "monkeyking" => "wukong".to_string(),
        _ => slug,
    }
}

impl Assets {
    pub fn from_values(
        champs: &Value,
        items: &Value,
        perks: &Value,
        styles: &Value,
        spells: &Value,
    ) -> Self {
        let champions = champs
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| {
                let id = e.get("id")?.as_i64()?;
                // ids < 1 = "nenhum"; > 9999 = variantes de modos especiais
                (1..=9999).contains(&id).then(|| {
                    (
                        id,
                        Champion {
                            id,
                            name: s(e, "name"),
                            alias: s(e, "alias"),
                        },
                    )
                })
            })
            .collect();
        let styles = styles
            .get("styles")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|st| {
                let id = st.get("id")?.as_i64()?;
                let perks = st
                    .get("slots")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|slot| slot.get("type").and_then(Value::as_str) != Some("kStatMod"))
                    .flat_map(|slot| {
                        slot.get("perks")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default()
                    })
                    .filter_map(|p| p.as_i64())
                    .collect();
                Some(Style {
                    id,
                    name: s(st, "name"),
                    icon: s(st, "iconPath"),
                    perks,
                })
            })
            .collect();
        Self {
            champions,
            items: named_map(items),
            perks: named_map(perks),
            styles,
            spells: named_map(spells),
        }
    }

    pub async fn load(lcu: &Lcu) -> Result<Self, LcuError> {
        let base = "/lol-game-data/assets/v1";
        let champs = lcu.get(&format!("{base}/champion-summary.json")).await?;
        let items = lcu.get(&format!("{base}/items.json")).await?;
        let perks = lcu.get(&format!("{base}/perks.json")).await?;
        let styles = lcu.get(&format!("{base}/perkstyles.json")).await?;
        let spells = lcu.get(&format!("{base}/summoner-spells.json")).await?;
        Ok(Self::from_values(&champs, &items, &perks, &styles, &spells))
    }

    /// Árvore de uma runa. Runas "splashable" podem aparecer em mais de uma árvore;
    /// `prefer` desempata (ex.: a primária ao procurar a secundária deve ser evitada).
    pub fn style_of_perk(&self, perk: i64, avoid: Option<i64>) -> Option<i64> {
        let mut found = self
            .styles
            .iter()
            .filter(|st| st.perks.contains(&perk))
            .map(|st| st.id);
        let first = found.next()?;
        if Some(first) != avoid {
            return Some(first);
        }
        found.next().or(Some(first))
    }

    /// Árvore secundária: a que contém as duas runas e não é a primária.
    pub fn secondary_style(&self, perks: &[i64], primary: i64) -> Option<i64> {
        self.styles
            .iter()
            .find(|st| st.id != primary && perks.iter().all(|p| st.perks.contains(p)))
            .map(|st| st.id)
            .or_else(|| {
                perks
                    .first()
                    .and_then(|p| self.style_of_perk(*p, Some(primary)))
            })
    }

    pub fn champion(&self, id: i64) -> Option<&Champion> {
        self.champions.get(&id)
    }

    pub fn item_name(&self, id: i64) -> String {
        self.items
            .get(&id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| format!("item {id}"))
    }
}

#[cfg(test)]
pub use tests::sample as sample_assets;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub fn sample() -> Assets {
        let styles = json!({"styles": [
            {"id": 8100, "name": "Dominação", "iconPath": "", "slots": [
                {"type": "kKeyStone", "perks": [9923, 8112]},
                {"type": "kMixedRegularSplashable", "perks": [8126, 8139]},
                {"type": "kMixedRegularSplashable", "perks": [8140, 8136]},
                {"type": "kMixedRegularSplashable", "perks": [8106, 8135]},
                {"type": "kStatMod", "perks": [5008, 5005]}]},
            {"id": 8200, "name": "Feitiçaria", "iconPath": "", "slots": [
                {"type": "kKeyStone", "perks": [8214]},
                {"type": "kMixedRegularSplashable", "perks": [8224, 8226]},
                {"type": "kMixedRegularSplashable", "perks": [8234, 8233]},
                {"type": "kMixedRegularSplashable", "perks": [8236, 8237]}]},
            {"id": 8400, "name": "Determinação", "iconPath": "", "slots": [
                {"type": "kKeyStone", "perks": [8437]},
                {"type": "kMixedRegularSplashable", "perks": [8242, 8446]}]},
        ]});
        let champs = json!([{"id": -1, "name": "Nenhum", "alias": "None"},
            {"id": 31, "name": "Cho'Gath", "alias": "Chogath"},
            {"id": 62, "name": "Wukong", "alias": "MonkeyKing"}]);
        let items = json!([{"id": 3084, "name": "Coração de Aço", "iconPath": "/x.png"},
            {"id": 1102, "name": "Filhote de Brasa", "iconPath": ""},
            {"id": 2003, "name": "Poção de Vida", "iconPath": ""},
            {"id": 2055, "name": "Sentinela de Controle", "iconPath": ""},
            {"id": 3009, "name": "Botas da Rapidez", "iconPath": ""},
            {"id": 3152, "name": "Cinto-foguete Hextec", "iconPath": ""}]);
        Assets::from_values(
            &champs,
            &items,
            &json!([]),
            &styles,
            &json!([{"id": 4, "name": "Flash"}]),
        )
    }

    #[test]
    fn slugs() {
        assert_eq!(lolalytics_slug("Chogath"), "chogath");
        assert_eq!(lolalytics_slug("MonkeyKing"), "wukong");
        assert_eq!(lolalytics_slug("KSante"), "ksante");
    }

    #[test]
    fn parses_and_finds_styles() {
        let a = sample();
        assert_eq!(a.champions.len(), 2, "ignora id -1");
        assert_eq!(a.style_of_perk(9923, None), Some(8100));
        assert_eq!(a.secondary_style(&[8224, 8234], 8100), Some(8200));
        assert!(
            !a.styles[0].perks.contains(&5008),
            "fragmentos fora da árvore"
        );
    }
}
