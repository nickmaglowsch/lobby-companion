//! Dados estáticos lidos do próprio cliente (nomes em pt-BR, ícones), sem Data Dragon.

use crate::lcu::{Lcu, LcuError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Champion {
    pub id: i64,
    pub name: String,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Named {
    pub name: String,
    pub icon: String,
    /// Só itens: `categories` do `items.json` do cliente (ex. "Armor", "SpellBlock", "Boots",
    /// "Consumable", "Trinket"). Vazio nas runas e feitiços.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
    /// Só itens: preço total (`priceTotal`), para o overlay (ouro em itens, próximo item).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub price: u32,
    /// Só itens: componentes (`from`), para descontar o que você já tem.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub from: Vec<i64>,
}

fn is_zero(x: &u32) -> bool {
    *x == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub id: i64,
    pub name: String,
    pub icon: String,
    /// Runas da árvore (keystones + fileiras normais), sem os fragmentos.
    pub perks: Vec<i64>,
    /// As mesmas runas por fileira (`slots[0]` = keystones), para achar as alternativas de cada escolha.
    pub slots: Vec<Vec<i64>>,
}

/// Cópia em disco (`assets.json` na pasta de cache) da última vez que o cliente respondeu: com ela
/// a janela mostra campeões e runas, busca builds e monta a sua página de runas sem o cliente aberto.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Assets {
    pub champions: HashMap<i64, Champion>,
    pub items: HashMap<i64, Named>,
    pub perks: HashMap<i64, Named>,
    pub styles: Vec<Style>,
    pub spells: HashMap<i64, Named>,
    /// Fileiras dos fragmentos (ataque, flexível, defesa), iguais em todas as árvores.
    #[serde(rename = "shardSlots")]
    pub shard_slots: Vec<Vec<i64>>,
}

/// Grava num temporário ao lado e troca pelo nome final: quem lê (outra tarefa, ou o app na
/// próxima abertura) nunca vê o arquivo pela metade, e duas gravações do mesmo arquivo ao mesmo
/// tempo não se misturam (cada uma tem o seu temporário; vale a última troca).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(
        ".{}-{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = PathBuf::from(tmp);
    let r = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    r
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
                    categories: e
                        .get("categories")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|c| c.as_str().map(String::from))
                        .collect(),
                    price: e
                        .get("priceTotal")
                        .and_then(Value::as_u64)
                        .and_then(|p| u32::try_from(p).ok())
                        .unwrap_or(0),
                    from: e
                        .get("from")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|v| {
                            v.as_i64()
                                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                        })
                        .collect(),
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
        let rows = |st: &Value, stat_mods: bool| -> Vec<Vec<i64>> {
            st.get("slots")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|slot| {
                    (slot.get("type").and_then(Value::as_str) == Some("kStatMod")) == stat_mods
                })
                .map(|slot| {
                    slot.get("perks")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_i64)
                        .collect()
                })
                .collect()
        };
        let style_list = styles
            .get("styles")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        let shard_slots = style_list
            .iter()
            .map(|st| rows(st, true))
            .find(|r| !r.is_empty())
            .unwrap_or_default();
        let styles = style_list
            .iter()
            .filter_map(|st| {
                let id = st.get("id")?.as_i64()?;
                let slots = rows(st, false);
                Some(Style {
                    id,
                    name: s(st, "name"),
                    icon: s(st, "iconPath"),
                    perks: slots.iter().flatten().copied().collect(),
                    slots,
                })
            })
            .collect();
        Self {
            champions,
            items: named_map(items),
            perks: named_map(perks),
            styles,
            spells: named_map(spells),
            shard_slots,
        }
    }

    /// Os cinco arquivos ao mesmo tempo (só GET local).
    pub async fn load(lcu: &Lcu) -> Result<Self, LcuError> {
        let (champs, items, perks, styles, spells) = tokio::try_join!(
            lcu.get("/lol-game-data/assets/v1/champion-summary.json"),
            lcu.get("/lol-game-data/assets/v1/items.json"),
            lcu.get("/lol-game-data/assets/v1/perks.json"),
            lcu.get("/lol-game-data/assets/v1/perkstyles.json"),
            lcu.get("/lol-game-data/assets/v1/summoner-spells.json"),
        )?;
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

    /// Cópia salva da última conexão (`None` se nunca houve, ou se o arquivo não serve).
    pub fn load_cached(path: &Path) -> Option<Self> {
        let a: Self = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        (!a.champions.is_empty() && !a.styles.is_empty()).then_some(a)
    }

    pub fn save_cached(&self, path: &Path) -> std::io::Result<()> {
        write_atomic(
            path,
            &serde_json::to_vec(self).map_err(std::io::Error::other)?,
        )
    }

    /// Ícones das runas e das árvores (os que o editor de página precisa sem o cliente).
    pub fn rune_icons(&self) -> Vec<String> {
        self.perks
            .values()
            .map(|p| &p.icon)
            .chain(self.styles.iter().map(|s| &s.icon))
            .filter(|i| !i.is_empty())
            .cloned()
            .collect()
    }

    pub fn champion(&self, id: i64) -> Option<&Champion> {
        self.champions.get(&id)
    }

    /// O item tem essa categoria no cliente (ex. "Armor")?
    pub fn item_has(&self, id: i64, category: &str) -> bool {
        self.items
            .get(&id)
            .is_some_and(|i| i.categories.iter().any(|c| c == category))
    }

    pub fn item_name(&self, id: i64) -> String {
        self.items
            .get(&id)
            .map(|n| n.name.clone())
            .unwrap_or_else(|| crate::t!("item.unknown", id = id))
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
                {"type": "kStatMod", "perks": [5008, 5005, 5007]},
                {"type": "kStatMod", "perks": [5008, 5010, 5001]},
                {"type": "kStatMod", "perks": [5011, 5013, 5001]}]},
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
        // Nomes e categorias no formato do cliente; 3111 fica de fora de propósito (testes de botas).
        let items = json!([{"id": 3084, "name": "Coração de Aço", "iconPath": "/x.png", "categories": ["Health", "HealthRegen"]},
            {"id": 1102, "name": "Filhote de Brasa", "iconPath": "", "categories": ["Jungle"]},
            {"id": 2003, "name": "Poção de Vida", "iconPath": "", "categories": ["HealthRegen", "Consumable", "Lane", "Jungle"]},
            {"id": 2055, "name": "Sentinela de Controle", "iconPath": "", "categories": ["Consumable", "Vision"]},
            {"id": 3009, "name": "Botas da Rapidez", "iconPath": "", "categories": ["Boots"]},
            {"id": 3152, "name": "Cinto-foguete Hextec", "iconPath": "", "categories": ["SpellDamage"]},
            {"id": 3068, "name": "Égide de Fogo Solar", "iconPath": "", "categories": ["Health", "Armor", "Aura"]},
            {"id": 3047, "name": "Botas Galvanizadas de Aço", "iconPath": "", "categories": ["Armor", "Boots"]},
            {"id": 2502, "name": "Desespero Eterno", "iconPath": "", "categories": ["Health", "Armor"]},
            {"id": 3742, "name": "Couraça do Defunto", "iconPath": "", "categories": ["Health", "Armor"]},
            {"id": 3075, "name": "Armadura de Espinhos", "iconPath": "", "categories": ["Health", "Armor"]},
            {"id": 3065, "name": "Semblante Espiritual", "iconPath": "", "categories": ["Health", "SpellBlock"]},
            {"id": 2525, "name": "Couraça Protoplasmática", "iconPath": "", "categories": ["Health", "Tenacity"]},
            {"id": 1028, "name": "Cristal de Rubi", "iconPath": "", "categories": ["Health"]},
            {"id": 1001, "name": "Botas", "iconPath": "", "categories": ["Boots"]},
            {"id": 3340, "name": "Sentinela Invisível", "iconPath": "", "categories": ["Trinket", "Vision"]}]);
        Assets::from_values(
            &champs,
            &items,
            &json!([]),
            &styles,
            &json!([{"id": 4, "name": "Flash"}]),
        )
    }

    #[test]
    fn cached_copy_roundtrip() {
        let dir = std::env::temp_dir().join(format!("lc-assets-{}", std::process::id()));
        let path = dir.join("assets.json");
        assert!(Assets::load_cached(&path).is_none());
        let a = sample();
        a.save_cached(&path).unwrap();
        let b = Assets::load_cached(&path).unwrap();
        assert_eq!(b.champions.len(), a.champions.len());
        assert_eq!(b.styles[0].slots, a.styles[0].slots);
        assert_eq!(b.shard_slots, a.shard_slots);
        assert!(b.item_has(3047, "Armor"));
        std::fs::write(&path, b"{}").unwrap();
        assert!(
            Assets::load_cached(&path).is_none(),
            "cópia vazia não serve"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_writes_never_mix_and_leave_no_temp_files() {
        let dir = std::env::temp_dir().join(format!("lc-atomic-{}", std::process::id()));
        let path = dir.join("icons").join("x.png");
        // Várias gravações do mesmo arquivo ao mesmo tempo: no fim, uma delas inteira.
        let threads: Vec<_> = (0..8u8)
            .map(|i| {
                let path = path.clone();
                std::thread::spawn(move || write_atomic(&path, &[i; 4096]))
            })
            .collect();
        for t in threads {
            // No Windows, trocar enquanto outra troca acontece pode falhar: o arquivo continua
            // inteiro (de outra gravação) e o temporário sai.
            let _ = t.join().unwrap();
        }
        let got = std::fs::read(&path).unwrap();
        assert_eq!(got.len(), 4096);
        assert!(got.iter().all(|b| *b == got[0]));
        let left = std::fs::read_dir(path.parent().unwrap()).unwrap().count();
        assert_eq!(left, 1, "sobrou arquivo temporário");
        let _ = std::fs::remove_dir_all(&dir);
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
        assert_eq!(a.styles[0].slots[1], vec![8126, 8139]);
        assert_eq!(a.shard_slots.len(), 3);
        assert_eq!(a.shard_slots[1], vec![5008, 5010, 5001]);
        assert!(a.item_has(3047, "Armor"));
        assert!(!a.item_has(3009, "Armor"));
        assert!(a.perks.values().all(|p| p.categories.is_empty()));
    }
}
