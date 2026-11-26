//! Páginas de runas próprias (para OTPs): **várias por campeão, cada uma com um nome**, guardadas
//! em `rune-pages.json`, ao lado de `settings.json`. Só ids de runas e árvores, nada de outros jogadores.
//!
//! Cada página aparece como uma opção (pelo nome) ao lado de "Mais jogada" e "Maior WR". A escolhida
//! do campeão (`active`) é a que vai para o cliente na importação dele, em qualquer lane; sem
//! escolhida, o campeão segue a escolha geral (`runeChoice`).
//!
//! Formato (versão 2): `{"version": 2, "champions": {"<championId>": {"pages": [{id, name,
//! primaryStyle, subStyle, perks[9], savedAt}], "active": id|null}}}`. O formato da 0.6.0/0.6.1
//! (uma página por campeão e lane: `{"pages": {"<id>": {"<lane>": {...}}}}`) é convertido ao ler:
//! cada lane vira uma página com o nome da lane.

use crate::assets::Assets;
use crate::champselect;
use crate::lola::model::RuneSet;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Nome curto: o cliente corta o nome da página (`LC: <Campeão> <nome>`) em 25 caracteres.
pub const MAX_NAME: usize = 20;

/// Uma página: árvores e as 9 runas na ordem do cliente (keystone + 3, 2 secundárias, 3 fragmentos).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnPage {
    /// Único dentro do campeão.
    pub id: u32,
    pub name: String,
    pub primary_style: i64,
    pub sub_style: i64,
    pub perks: Vec<i64>,
    /// Segundos desde 1970 (só para mostrar quando foi salva).
    pub saved_at: u64,
}

/// Nome que a pessoa deu, limpo: sem espaços sobrando, sem o nosso prefixo `LC:`, até `MAX_NAME`.
/// Vazio = `None` (quem chama escolhe um padrão).
pub fn clean_name(name: &str) -> Option<String> {
    let name = name.trim();
    let name = name.strip_prefix("LC:").unwrap_or(name);
    let joined = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let short: String = joined.chars().take(MAX_NAME).collect();
    let short = short.trim().to_string();
    (!short.is_empty()).then_some(short)
}

impl OwnPage {
    fn new(primary_style: i64, sub_style: i64, perks: Vec<i64>, saved_at: u64) -> Self {
        Self {
            id: 0,
            name: String::new(),
            primary_style,
            sub_style,
            perks,
            saved_at,
        }
    }

    /// Página montada na janela (sem o cliente): só confere o formato; as runas, `check`.
    pub fn from_editor(
        primary_style: i64,
        sub_style: i64,
        perks: Vec<i64>,
        saved_at: u64,
    ) -> Result<Self, String> {
        if perks.len() != 9 || perks.contains(&0) || primary_style <= 0 || sub_style <= 0 {
            return Err(crate::t!("own.pickNine"));
        }
        Ok(Self::new(primary_style, sub_style, perks, saved_at))
    }

    /// Lê a página atual do cliente (o nome dela vira o nome sugerido). Recusa página incompleta.
    pub fn from_client(page: &Value, saved_at: u64) -> Result<Self, String> {
        if page.get("isValid").and_then(Value::as_bool) == Some(false) {
            return Err(crate::t!("own.clientIncomplete"));
        }
        let style = |k: &str| page.get(k).and_then(Value::as_i64).unwrap_or(0);
        let perks: Vec<i64> = page
            .get("selectedPerkIds")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_i64).collect())
            .unwrap_or_default();
        let (primary_style, sub_style) = (style("primaryStyleId"), style("subStyleId"));
        if perks.len() != 9 || perks.contains(&0) || primary_style <= 0 || sub_style <= 0 {
            return Err(crate::t!("own.clientIncomplete"));
        }
        let mut p = Self::new(primary_style, sub_style, perks, saved_at);
        p.name = page
            .get("name")
            .and_then(Value::as_str)
            .and_then(clean_name)
            .unwrap_or_default();
        Ok(p)
    }

    /// Confere a página contra as runas do cliente (um patch pode trocar runas).
    pub fn check(&self, assets: &Assets) -> Result<(), String> {
        let stale = || crate::t!("own.stale");
        let style = |id: i64| assets.styles.iter().find(|s| s.id == id);
        let (Some(pri), Some(sub)) = (style(self.primary_style), style(self.sub_style)) else {
            return Err(stale());
        };
        if self.perks.len() != 9 || pri.id == sub.id {
            return Err(stale());
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
            return Err(stale());
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

/// As páginas de um campeão e a escolhida (`None` = segue Mais jogada/Maior WR).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default)]
pub struct ChampionPages {
    pub pages: Vec<OwnPage>,
    pub active: Option<u32>,
}

/// Uma linha da lista que a janela recebe (`get_own_pages`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnEntry {
    pub champion_id: i64,
    /// É a escolhida do campeão (vai para o cliente na importação).
    pub active: bool,
    #[serde(flatten)]
    pub page: OwnPage,
}

/// O arquivo inteiro (versão 2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct OwnPages {
    pub version: u32,
    pub champions: BTreeMap<i64, ChampionPages>,
}

impl Default for OwnPages {
    fn default() -> Self {
        Self {
            version: 2,
            champions: BTreeMap::new(),
        }
    }
}

/// Página do formato antigo (uma por campeão e lane).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct V1Page {
    primary_style: i64,
    sub_style: i64,
    perks: Vec<i64>,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    saved_at: u64,
}

impl OwnPages {
    /// Arquivo ausente ou ilegível = nenhuma página (não apaga nada: só regrava ao salvar).
    /// O formato antigo (por lane) é convertido aqui e gravado no novo no próximo salvar.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|v| Self::from_value(&v))
            .unwrap_or_default()
    }

    pub fn from_value(v: &Value) -> Self {
        if v.get("champions").is_some() {
            return serde_json::from_value(v.clone()).unwrap_or_default();
        }
        let Ok(old) = serde_json::from_value::<BTreeMap<i64, BTreeMap<String, V1Page>>>(
            v.get("pages").cloned().unwrap_or(Value::Null),
        ) else {
            return Self::default();
        };
        let mut out = Self::default();
        for (champion, lanes) in old {
            let mut c = ChampionPages::default();
            for (i, (lane, p)) in lanes.into_iter().enumerate() {
                let id = i as u32 + 1;
                let label = champselect::lane_label(crate::i18n::Lang::current(), &lane);
                c.pages.push(OwnPage {
                    id,
                    name: if label.is_empty() {
                        crate::t!("own.defaultName", n = id)
                    } else {
                        label
                    },
                    primary_style: p.primary_style,
                    sub_style: p.sub_style,
                    perks: p.perks,
                    saved_at: p.saved_at,
                });
                if p.active && c.active.is_none() {
                    c.active = Some(id);
                }
            }
            if !c.pages.is_empty() {
                out.champions.insert(champion, c);
            }
        }
        out
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

    pub fn get(&self, champion_id: i64, id: u32) -> Option<&OwnPage> {
        self.champions
            .get(&champion_id)?
            .pages
            .iter()
            .find(|p| p.id == id)
    }

    /// A página escolhida do campeão (a que vai para o cliente), se houver.
    pub fn active(&self, champion_id: i64) -> Option<&OwnPage> {
        let c = self.champions.get(&champion_id)?;
        c.pages.iter().find(|p| Some(p.id) == c.active)
    }

    /// Grava a página (`id` existente = troca; senão cria) e a deixa escolhida. Nome vazio vira
    /// "Página N". Devolve o id.
    pub fn upsert(&mut self, champion_id: i64, id: Option<u32>, mut page: OwnPage) -> u32 {
        let c = self.champions.entry(champion_id).or_default();
        let existing = id.and_then(|id| c.pages.iter().position(|p| p.id == id));
        let id = match existing {
            Some(i) => c.pages[i].id,
            None => c.pages.iter().map(|p| p.id).max().unwrap_or(0) + 1,
        };
        page.id = id;
        if page.name.is_empty() {
            page.name = match existing {
                Some(i) => c.pages[i].name.clone(),
                None => crate::t!("own.defaultName", n = c.pages.len() + 1),
            };
        }
        match existing {
            Some(i) => c.pages[i] = page,
            None => c.pages.push(page),
        }
        c.active = Some(id);
        id
    }

    /// Apaga; devolve a página e se era a escolhida.
    pub fn remove(&mut self, champion_id: i64, id: u32) -> Option<(OwnPage, bool)> {
        let c = self.champions.get_mut(&champion_id)?;
        let i = c.pages.iter().position(|p| p.id == id)?;
        let page = c.pages.remove(i);
        let was_active = c.active == Some(id);
        if was_active {
            c.active = None;
        }
        if c.pages.is_empty() {
            self.champions.remove(&champion_id);
        }
        Some((page, was_active))
    }

    /// Escolhe a página do campeão (`None` = nenhuma: segue Mais jogada/Maior WR).
    /// Devolve se mudou algo; id que não existe não muda nada.
    pub fn set_active(&mut self, champion_id: i64, id: Option<u32>) -> bool {
        let Some(c) = self.champions.get_mut(&champion_id) else {
            return false;
        };
        if id.is_some_and(|id| !c.pages.iter().any(|p| p.id == id)) || c.active == id {
            return false;
        }
        c.active = id;
        true
    }

    pub fn entries(&self) -> Vec<OwnEntry> {
        self.champions
            .iter()
            .flat_map(|(champion, c)| {
                c.pages.iter().map(|p| OwnEntry {
                    champion_id: *champion,
                    active: c.active == Some(p.id),
                    page: p.clone(),
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

/// A página escolhida do campeão vence; senão Maior WR (com a mais jogada de reserva)
/// ou Mais jogada, como antes.
pub fn choose<'a>(
    rune_choice: &str,
    runes_pick: Option<&'a RuneSet>,
    runes_win: Option<&'a RuneSet>,
    active_own: Option<&'a OwnPage>,
) -> Chosen<'a> {
    if let Some(p) = active_own {
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
        json!({"id": 55, "name": "  Ahri   agressiva ", "current": true, "isValid": true, "isDeletable": true,
            "primaryStyleId": 8100, "subStyleId": 8200,
            "selectedPerkIds": [9923, 8126, 8140, 8106, 8224, 8234, 5008, 5010, 5011]})
    }

    fn page() -> OwnPage {
        OwnPage::from_client(&client_page(), 1_700_000_000).unwrap()
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
        let p = page();
        assert_eq!((p.primary_style, p.sub_style), (8100, 8200));
        assert_eq!(p.name, "Ahri agressiva", "nome do cliente limpo");
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
    fn names_are_cleaned() {
        assert_eq!(clean_name("  Full   AP "), Some("Full AP".into()));
        assert_eq!(
            clean_name("LC: Cho'Gath Selva"),
            Some("Cho'Gath Selva".into())
        );
        assert_eq!(clean_name("   "), None);
        assert_eq!(
            clean_name(&"x".repeat(40)).unwrap().chars().count(),
            MAX_NAME
        );
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
        let mut p = page();
        p.perks[1] = 8224; // runa da Feitiçaria na árvore primária
        assert!(p.check(&assets).is_err());
        let mut p = page();
        p.sub_style = 9999;
        assert!(p.check(&assets).is_err());
        let mut p = page();
        p.perks[8] = 5005; // fragmento de ataque na fileira de defesa
        assert!(p.check(&assets).is_err());
    }

    #[test]
    fn many_named_pages_per_champion() {
        let mut pages = OwnPages::default();
        let a = pages.upsert(103, None, page());
        let mut b_page = page();
        b_page.name = String::new();
        let b = pages.upsert(103, None, b_page);
        assert_eq!((a, b), (1, 2));
        assert_eq!(
            pages.get(103, 2).unwrap().name,
            crate::t!("own.defaultName", n = 2),
            "sem nome: padrão"
        );
        assert_eq!(
            pages.active(103).unwrap().id,
            2,
            "a última salva fica escolhida"
        );

        // trocar a 1 mantém o id e o nome quando o novo vem vazio
        let mut edit = page();
        edit.name = String::new();
        edit.perks[1] = 8139;
        assert_eq!(pages.upsert(103, Some(1), edit), 1);
        assert_eq!(pages.get(103, 1).unwrap().name, "Ahri agressiva");
        assert_eq!(pages.get(103, 1).unwrap().perks[1], 8139);
        assert_eq!(pages.champions[&103].pages.len(), 2);
        assert_eq!(pages.active(103).unwrap().id, 1);

        assert!(pages.set_active(103, None));
        assert!(pages.active(103).is_none());
        assert!(!pages.set_active(103, Some(9)), "id que não existe");
        assert!(pages.set_active(103, Some(2)));
        assert!(!pages.set_active(103, Some(2)));
        assert!(!pages.set_active(7, Some(1)));

        let e = pages.entries();
        assert_eq!(e.len(), 2);
        assert_eq!(e.iter().filter(|x| x.active).count(), 1);

        assert_eq!(pages.remove(103, 2).map(|(_, was)| was), Some(true));
        assert!(
            pages.active(103).is_none(),
            "apagou a escolhida: volta ao lolalytics"
        );
        assert!(pages.remove(103, 2).is_none());
        // id novo nunca reaproveita um em uso
        assert_eq!(pages.upsert(103, Some(42), page()), 2);
        pages.remove(103, 1);
        pages.remove(103, 2);
        assert!(
            !pages.champions.contains_key(&103),
            "campeão sem páginas sai do arquivo"
        );
    }

    #[test]
    fn store_roundtrip_and_json_shape() {
        let dir = std::env::temp_dir().join(format!("lc-own-runes-{}", std::process::id()));
        let path = dir.join("rune-pages.json");
        let _ = std::fs::remove_file(&path);
        assert_eq!(OwnPages::load(&path), OwnPages::default());

        let mut pages = OwnPages::default();
        pages.upsert(103, None, page());
        pages.upsert(31, None, page());
        pages.save(&path).unwrap();

        let raw: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(raw["version"], 2);
        assert_eq!(
            raw["champions"]["103"]["pages"][0]["name"],
            "Ahri agressiva"
        );
        assert_eq!(raw["champions"]["103"]["pages"][0]["primaryStyle"], 8100);
        assert_eq!(raw["champions"]["103"]["active"], 1);
        assert_eq!(OwnPages::load(&path), pages);

        let v = serde_json::to_value(&pages.entries()[0]).unwrap();
        assert_eq!(v["championId"], 31);
        assert_eq!(v["id"], 1);
        assert_eq!(v["active"], true);
        assert_eq!(v["savedAt"], 1_700_000_000u64);

        std::fs::write(&path, b"lixo").unwrap();
        assert_eq!(OwnPages::load(&path), OwnPages::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn converts_the_old_per_lane_file() {
        let p = json!({"primaryStyle": 8100, "subStyle": 8200,
            "perks": [9923, 8126, 8140, 8106, 8224, 8234, 5008, 5010, 5011], "savedAt": 5});
        let mut top = p.clone();
        top["active"] = json!(false);
        let mut mid = p.clone();
        mid["active"] = json!(true);
        let mut jg = p;
        jg["active"] = json!(false);
        let v1 = json!({"pages": {"103": {"top": top, "middle": mid}, "31": {"jungle": jg}}});
        let got = OwnPages::from_value(&v1);
        // As páginas antigas ganham o nome da lane no idioma do app.
        let label = |l: &str| champselect::lane_label(crate::i18n::Lang::current(), l);
        assert_eq!(got.version, 2);
        let ahri = &got.champions[&103];
        // lanes em ordem alfabética: middle (1), top (2)
        assert_eq!(
            ahri.pages
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            vec![label("middle"), label("top")]
        );
        assert_eq!(ahri.active, Some(1), "a ativa continua escolhida");
        assert_eq!(got.champions[&31].active, None);
        assert_eq!(got.champions[&31].pages[0].name, label("jungle"));
        assert_eq!(got.champions[&31].pages[0].saved_at, 5);
        assert_eq!(
            OwnPages::from_value(&json!({"pages": 3})),
            OwnPages::default()
        );
    }

    #[test]
    fn choice_prefers_the_chosen_own_page() {
        let (pick, win) = (rs([9923, 8126, 8140, 8106]), rs([8112, 8126, 8140, 8106]));
        let own = page();
        assert_eq!(
            choose("pick", Some(&pick), Some(&win), Some(&own)),
            Chosen::Own(&own)
        );
        assert_eq!(choose("win", None, None, Some(&own)), Chosen::Own(&own));
        let got = choose("win", Some(&pick), Some(&win), None);
        assert!(matches!(got, Chosen::Lola(s) if s.primary[0] == 8112));
        let got = choose("pick", Some(&pick), Some(&win), None);
        assert!(matches!(got, Chosen::Lola(s) if s.primary[0] == 9923));
        let got = choose("win", Some(&pick), None, None);
        assert!(matches!(got, Chosen::Lola(s) if s.primary[0] == 9923));
        assert_eq!(choose("pick", None, Some(&win), None), Chosen::None);
    }
}
