//! Páginas de runas próprias (para OTPs): uma por campeão e lane, guardadas em
//! `rune-pages.json`, ao lado de `settings.json`. Só ids de runas e árvores, nada de outros jogadores.
//!
//! A página vem da página atual do cliente (`GET /lol-perks/v1/currentpage`, só leitura) e aparece
//! como a opção "Minha" ao lado de "Mais jogada" e "Maior WR". Com `active`, é ela que vai para o
//! cliente na importação desse campeão nessa lane; os outros campeões seguem a escolha geral.

use crate::assets::Assets;
use crate::lola::model::RuneSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Uma página: árvores e as 9 runas na ordem do cliente (keystone + 3, 2 secundárias, 3 fragmentos).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnPage {
    pub primary_style: i64,
    pub sub_style: i64,
    pub perks: Vec<i64>,
    /// Usar esta página na importação (escolhida "Minha"); falso = segue Mais jogada/Maior WR.
    pub active: bool,
    /// Segundos desde 1970 (só para mostrar quando foi salva).
    pub saved_at: u64,
}

impl OwnPage {
    /// Página montada na janela (sem o cliente): só confere o formato; as runas, `check`.
    pub fn from_editor(
        primary_style: i64,
        sub_style: i64,
        perks: Vec<i64>,
        saved_at: u64,
    ) -> Result<Self, String> {
        if perks.len() != 9 || perks.contains(&0) || primary_style <= 0 || sub_style <= 0 {
            return Err("escolha as 9 runas da página".into());
        }
        Ok(Self {
            primary_style,
            sub_style,
            perks,
            active: true,
            saved_at,
        })
    }

    /// Lê a página atual do cliente. Recusa página incompleta.
    pub fn from_client(page: &Value, saved_at: u64) -> Result<Self, String> {
        if page.get("isValid").and_then(Value::as_bool) == Some(false) {
            return Err("a página atual do cliente está incompleta".into());
        }
        let style = |k: &str| page.get(k).and_then(Value::as_i64).unwrap_or(0);
        let perks: Vec<i64> = page
            .get("selectedPerkIds")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        let (primary_style, sub_style) = (style("primaryStyleId"), style("subStyleId"));
        if perks.len() != 9 || perks.contains(&0) || primary_style <= 0 || sub_style <= 0 {
            return Err("a página atual do cliente está incompleta".into());
        }
        Ok(Self {
            primary_style,
            sub_style,
            perks,
            active: true,
            saved_at,
        })
    }

    /// Confere a página contra as runas do cliente (um patch pode trocar runas).
    pub fn check(&self, assets: &Assets) -> Result<(), String> {
        const STALE: &str = "a sua página tem runas que não existem mais: salve de novo";
        let style = |id: i64| assets.styles.iter().find(|s| s.id == id);
        let (Some(pri), Some(sub)) = (style(self.primary_style), style(self.sub_style)) else {
            return Err(STALE.into());
        };
        if self.perks.len() != 9 || pri.id == sub.id {
            return Err(STALE.into());
        }
        // Primária: keystone e uma runa de cada fileira, na ordem.
        let primary_ok =
            (0..4).all(|i| pri.slots.get(i).is_some_and(|r| r.contains(&self.perks[i])));
        // Secundária: duas runas de fileiras diferentes, sem a dos keystones.
        let row = |p: i64| sub.slots.iter().position(|r| r.contains(&p));
        let secondary_ok = matches!(
            (row(self.perks[4]), row(self.perks[5])),
            (Some(a), Some(b)) if a > 0 && b > 0 && a != b
        );
        let shards_ok = self.perks[6..]
            .iter()
            .zip(&assets.shard_slots)
            .all(|(p, row)| row.contains(p))
            && assets.shard_slots.len() >= 3;
        if !primary_ok || !secondary_ok || !shards_ok {
            return Err(STALE.into());
        }
        Ok(())
    }

    /// Mesmo formato das páginas do lolalytics (para a janela e as comparações), sem WR/amostra.
    pub fn rune_set(&self) -> RuneSet {
        let p = |r: std::ops::Range<usize>| self.perks.get(r).unwrap_or_default().to_vec();
        RuneSet {
            primary: p(0..4),
            secondary: p(4..6),
            shards: p(6..9),
            wr: 0.0,
            n: 0,
        }
    }
}

/// Uma linha da lista que a janela recebe (`get_own_pages`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnEntry {
    pub champion_id: i64,
    /// Lane do lolalytics (top, jungle, middle, bottom, support).
    pub lane: String,
    #[serde(flatten)]
    pub page: OwnPage,
}

/// O arquivo inteiro: `{"pages": {"<championId>": {"<lane>": página}}}`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct OwnPages {
    pub pages: BTreeMap<i64, BTreeMap<String, OwnPage>>,
}

impl OwnPages {
    /// Arquivo ausente ou ilegível = nenhuma página (não apaga nada: só regrava ao salvar).
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        // Grava ao lado e troca: um fechamento no meio não deixa o arquivo pela metade.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }

    pub fn get(&self, champion_id: i64, lane: &str) -> Option<&OwnPage> {
        self.pages.get(&champion_id)?.get(lane)
    }

    pub fn set(&mut self, champion_id: i64, lane: &str, page: OwnPage) {
        self.pages
            .entry(champion_id)
            .or_default()
            .insert(lane.to_string(), page);
    }

    /// Apaga; devolve a página que existia.
    pub fn remove(&mut self, champion_id: i64, lane: &str) -> Option<OwnPage> {
        let lanes = self.pages.get_mut(&champion_id)?;
        let old = lanes.remove(lane);
        if lanes.is_empty() {
            self.pages.remove(&champion_id);
        }
        old
    }

    /// Liga/desliga "Minha" para o campeão e lane; devolve se mudou algo.
    pub fn set_active(&mut self, champion_id: i64, lane: &str, active: bool) -> bool {
        match self
            .pages
            .get_mut(&champion_id)
            .and_then(|l| l.get_mut(lane))
        {
            Some(p) if p.active != active => {
                p.active = active;
                true
            }
            _ => false,
        }
    }

    pub fn entries(&self) -> Vec<OwnEntry> {
        self.pages
            .iter()
            .flat_map(|(id, lanes)| {
                lanes.iter().map(|(lane, page)| OwnEntry {
                    champion_id: *id,
                    lane: lane.clone(),
                    page: page.clone(),
                })
            })
            .collect()
    }
}

/// Que página de runas importar.
#[derive(Debug, PartialEq)]
pub enum Chosen<'a> {
    Own(&'a OwnPage),
    Lola(&'a RuneSet),
    None,
}

/// A sua página (se salva e escolhida) vence; senão Maior WR (com a mais jogada de reserva)
/// ou Mais jogada, como antes.
pub fn choose<'a>(
    rune_choice: &str,
    runes_pick: Option<&'a RuneSet>,
    runes_win: Option<&'a RuneSet>,
    own: Option<&'a OwnPage>,
) -> Chosen<'a> {
    if let Some(p) = own.filter(|p| p.active) {
        return Chosen::Own(p);
    }
    let set = if rune_choice == "win" {
        runes_win.or(runes_pick)
    } else {
        runes_pick
    };
    set.map_or(Chosen::None, Chosen::Lola)
}

pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::sample_assets;
    use serde_json::json;

    /// Página no formato do cliente com as runas da `sample_assets` (Dominação + Feitiçaria).
    fn client_page() -> Value {
        json!({"id": 55, "name": "Minha Ahri", "current": true, "isValid": true, "isDeletable": true,
            "primaryStyleId": 8100, "subStyleId": 8200,
            "selectedPerkIds": [9923, 8126, 8140, 8106, 8224, 8234, 5008, 5010, 5011]})
    }

    fn rs(primary: [i64; 4]) -> RuneSet {
        RuneSet {
            primary: primary.to_vec(),
            secondary: vec![8224, 8234],
            shards: vec![5008, 5008, 5001],
            wr: 52.0,
            n: 1000,
        }
    }

    #[test]
    fn parses_client_page() {
        let p = OwnPage::from_client(&client_page(), 1_700_000_000).unwrap();
        assert_eq!((p.primary_style, p.sub_style), (8100, 8200));
        assert_eq!(p.perks.len(), 9);
        assert!(p.active);
        let set = p.rune_set();
        assert_eq!(set.primary, vec![9923, 8126, 8140, 8106]);
        assert_eq!(set.secondary, vec![8224, 8234]);
        assert_eq!(set.shards, vec![5008, 5010, 5011]);
        assert!(set.is_complete());
        assert!(p.check(&sample_assets()).is_ok());

        let mut short = client_page();
        short["selectedPerkIds"] = json!([9923, 8126]);
        assert!(OwnPage::from_client(&short, 0).is_err());
        let mut invalid = client_page();
        invalid["isValid"] = json!(false);
        assert!(OwnPage::from_client(&invalid, 0).is_err());
        assert!(OwnPage::from_client(&Value::Null, 0).is_err());
    }

    #[test]
    fn editor_page_follows_the_rows() {
        let assets = sample_assets();
        let ok = |perks: Vec<i64>| {
            OwnPage::from_editor(8100, 8200, perks, 0).and_then(|p| p.check(&assets))
        };
        assert!(ok(vec![9923, 8126, 8140, 8106, 8224, 8234, 5008, 5010, 5011]).is_ok());
        assert!(
            ok(vec![9923, 8126, 8140, 8106, 8234, 8224, 5008, 5010, 5011]).is_ok(),
            "secundárias em qualquer ordem"
        );
        assert!(
            ok(vec![9923, 8126, 8140, 8106, 8224, 8226, 5008, 5010, 5011]).is_err(),
            "duas da mesma fileira"
        );
        assert!(
            ok(vec![9923, 8126, 8140, 8106, 8214, 8234, 5008, 5010, 5011]).is_err(),
            "keystone na secundária"
        );
        assert!(
            ok(vec![9923, 8140, 8126, 8106, 8224, 8234, 5008, 5010, 5011]).is_err(),
            "fileiras trocadas"
        );
        assert!(ok(vec![9923, 8126, 8140]).is_err());
        assert!(OwnPage::from_editor(0, 8200, vec![1; 9], 0).is_err());
    }

    #[test]
    fn check_catches_stale_runes() {
        let assets = sample_assets();
        let mut p = OwnPage::from_client(&client_page(), 0).unwrap();
        p.perks[1] = 8224; // runa da Feitiçaria na árvore primária
        assert!(p.check(&assets).is_err());
        let mut p = OwnPage::from_client(&client_page(), 0).unwrap();
        p.sub_style = 9999;
        assert!(p.check(&assets).is_err());
        let mut p = OwnPage::from_client(&client_page(), 0).unwrap();
        p.perks[8] = 5005; // fragmento de ataque na fileira de defesa
        assert!(p.check(&assets).is_err());
    }

    #[test]
    fn store_roundtrip_by_champion_and_lane() {
        let dir = std::env::temp_dir().join(format!("lc-own-runes-{}", std::process::id()));
        let path = dir.join("rune-pages.json");
        let _ = std::fs::remove_file(&path);
        assert_eq!(OwnPages::load(&path), OwnPages::default());

        let page = OwnPage::from_client(&client_page(), 1_700_000_000).unwrap();
        let mut pages = OwnPages::default();
        pages.set(103, "middle", page.clone());
        pages.set(103, "top", page.clone());
        pages.set(31, "jungle", page.clone());
        pages.save(&path).unwrap();

        let raw: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw["pages"]["103"]["middle"]["primaryStyle"], 8100);
        assert_eq!(raw["pages"]["31"]["jungle"]["perks"][0], 9923);

        let mut back = OwnPages::load(&path);
        assert_eq!(back, pages);
        assert_eq!(back.get(103, "middle"), Some(&page));
        assert_eq!(back.get(103, "jungle"), None);
        assert_eq!(back.entries().len(), 3);
        assert_eq!(back.entries()[0].champion_id, 31);

        assert!(back.set_active(103, "middle", false));
        assert!(!back.set_active(103, "middle", false));
        assert!(!back.set_active(7, "middle", true));
        assert!(!back.get(103, "middle").unwrap().active);

        assert!(back.remove(103, "middle").is_some());
        assert!(back.remove(103, "middle").is_none());
        assert!(back.remove(103, "top").is_some());
        assert!(
            !back.pages.contains_key(&103),
            "campeão sem páginas sai do arquivo"
        );

        std::fs::write(&path, b"lixo").unwrap();
        assert_eq!(OwnPages::load(&path), OwnPages::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entry_json_is_flat() {
        let e = OwnEntry {
            champion_id: 103,
            lane: "middle".into(),
            page: OwnPage::from_client(&client_page(), 5).unwrap(),
        };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["championId"], 103);
        assert_eq!(v["savedAt"], 5);
        assert_eq!(v["active"], true);
    }

    #[test]
    fn choice_prefers_active_own_page() {
        let (pick, win) = (rs([9923, 8126, 8140, 8106]), rs([8112, 8126, 8140, 8106]));
        let mut own = OwnPage::from_client(&client_page(), 0).unwrap();
        assert_eq!(
            choose("pick", Some(&pick), Some(&win), Some(&own)),
            Chosen::Own(&own)
        );
        assert_eq!(choose("win", None, None, Some(&own)), Chosen::Own(&own));
        own.active = false;
        let got = choose("win", Some(&pick), Some(&win), Some(&own));
        assert!(matches!(got, Chosen::Lola(s) if s.primary[0] == 8112));
        let got = choose("pick", Some(&pick), Some(&win), None);
        assert!(matches!(got, Chosen::Lola(s) if s.primary[0] == 9923));
        let got = choose("win", Some(&pick), None, None);
        assert!(matches!(got, Chosen::Lola(s) if s.primary[0] == 9923));
        assert_eq!(choose("pick", None, Some(&win), Some(&own)), Chosen::None);
    }
}
