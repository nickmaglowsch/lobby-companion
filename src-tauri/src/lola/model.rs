//! Resposta `ep=build-full` do lolalytics → [`Build`].
//!
//! Formatos observados em 2026-11-19 (patch 16.23):
//! - listas de itens/botas: `[itemId, wr, pr, partidas, tempo]`;
//! - `startSet`/`spells`/`skillOrder`: `["1102_2003", wr, pr, partidas]`;
//! - `summary.pick|win.runes.set`: `{pri: [4], sec: [2], mod: [3]}` (fragmentos na ordem das fileiras);
//! - `summary.pick.skillorder.id`: dígitos 1–4 = Q/W/E/R por nível.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ItemStat {
    pub id: i64,
    pub wr: f64,
    pub pr: f64,
    pub n: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuneSet {
    pub primary: Vec<i64>,
    pub secondary: Vec<i64>,
    pub shards: Vec<i64>,
    pub wr: f64,
    pub n: u64,
}

impl RuneSet {
    /// Ordem do cliente: keystone + 3 primárias, 2 secundárias, 3 fragmentos.
    pub fn selected_perk_ids(&self) -> Vec<i64> {
        self.primary
            .iter()
            .chain(&self.secondary)
            .chain(&self.shards)
            .copied()
            .collect()
    }

    pub fn is_complete(&self) -> bool {
        self.primary.len() == 4 && self.secondary.len() == 2 && self.shards.len() == 3
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Combo {
    /// Itens/feitiços/letras da combinação, na ordem do lolalytics.
    pub ids: Vec<String>,
    pub wr: f64,
    pub pr: f64,
    pub n: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Build {
    pub slug: String,
    pub lane: String,
    pub patch: String,
    pub wr: f64,
    pub pr: f64,
    pub br: f64,
    pub n: u64,
    pub rank: Option<u64>,
    pub rank_total: Option<u64>,
    pub runes_pick: Option<RuneSet>,
    pub runes_win: Option<RuneSet>,
    pub spells: Vec<Combo>,
    pub skill_priority: Vec<Combo>,
    /// Ordem de 15 níveis mais jogada, ex. "EQWEERQEQERQQWW".
    pub skill_order: Option<String>,
    pub start_sets: Vec<Combo>,
    pub boots: Vec<ItemStat>,
    /// `item1..item5`: itens por posição na build, com WR.
    pub slots: Vec<Vec<ItemStat>>,
    /// Lanes do campeão com % de jogos (`nav.lanes`).
    pub lanes: Vec<(String, f64)>,
}

fn f(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(0.0)
}

fn u(v: Option<&Value>) -> u64 {
    v.and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f.max(0.0) as u64)))
        .unwrap_or(0)
}

fn ids(v: Option<&Value>) -> Vec<i64> {
    v.and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .collect()
}

fn item_list(v: Option<&Value>) -> Vec<ItemStat> {
    let mut out: Vec<ItemStat> = v
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let r = row.as_array()?;
            Some(ItemStat {
                id: r.first()?.as_i64()?,
                wr: f(r.get(1)),
                pr: f(r.get(2)),
                n: u(r.get(3)),
            })
        })
        .collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.n));
    out
}

fn combo_list(v: Option<&Value>) -> Vec<Combo> {
    let mut out: Vec<Combo> = v
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let r = row.as_array()?;
            let key = match r.first()? {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            let ids = if key.contains('_') {
                key.split('_').map(String::from).collect()
            } else if key.chars().all(|c| c.is_ascii_alphabetic()) {
                key.chars().map(String::from).collect()
            } else {
                vec![key]
            };
            Some(Combo {
                ids,
                wr: f(r.get(1)),
                pr: f(r.get(2)),
                n: u(r.get(3)),
            })
        })
        .collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.n));
    out
}

fn rune_set(v: Option<&Value>) -> Option<RuneSet> {
    let r = v?;
    let set = r.get("set")?;
    Some(RuneSet {
        primary: ids(set.get("pri")),
        secondary: ids(set.get("sec")),
        shards: ids(set.get("mod")),
        wr: f(r.get("wr")),
        n: u(r.get("n")),
    })
}

/// `312334313141122` → `"EQWEEREQEQRQQWW"`.
pub fn skill_order_letters(id: &Value) -> Option<String> {
    let digits = match id {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => return None,
    };
    let letters: Option<String> = digits
        .chars()
        .map(|c| match c {
            '1' => Some('Q'),
            '2' => Some('W'),
            '3' => Some('E'),
            '4' => Some('R'),
            _ => None,
        })
        .collect();
    letters.filter(|s| !s.is_empty())
}

/// Erros do lolalytics vêm como `{"status": 4042}` (campeão inválido) / `4044` (lane inválida).
pub fn parse_build_full(v: &Value) -> Result<Build, String> {
    if let Some(code) = v.get("status").and_then(Value::as_i64) {
        return Err(format!(
            "lolalytics sem dados para essa combinação (status {code})"
        ));
    }
    let header = v
        .get("header")
        .ok_or("resposta do lolalytics sem 'header'")?;
    let pick = v.pointer("/summary/pick");
    let win = v.pointer("/summary/win");
    let lanes = v
        .pointer("/nav/lanes")
        .and_then(Value::as_object)
        .map(|m| {
            let mut l: Vec<(String, f64)> = m
                .iter()
                .map(|(k, x)| (k.clone(), x.as_f64().unwrap_or(0.0)))
                .collect();
            l.sort_by(|a, b| b.1.total_cmp(&a.1));
            l
        })
        .unwrap_or_default();
    Ok(Build {
        slug: String::new(),
        lane: header
            .get("lane")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        patch: header
            .get("patch")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        wr: f(header.get("wr")),
        pr: f(header.get("pr")),
        br: f(header.get("br")),
        n: u(header.get("n")),
        rank: header.get("rank").and_then(Value::as_u64),
        rank_total: header.get("rankTotal").and_then(Value::as_u64),
        runes_pick: rune_set(pick.and_then(|p| p.get("runes"))),
        runes_win: rune_set(win.and_then(|p| p.get("runes"))),
        spells: combo_list(v.get("spells")),
        skill_priority: combo_list(v.get("skillOrder")),
        skill_order: pick
            .and_then(|p| p.pointer("/skillorder/id"))
            .and_then(skill_order_letters),
        start_sets: combo_list(v.get("startSet")),
        boots: item_list(v.get("boots")),
        slots: (1..=5)
            .map(|i| item_list(v.get(format!("item{i}").as_str())))
            .collect(),
        lanes,
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    /// Mesmo formato da resposta real, com números inventados (não redistribuímos dados do lolalytics).
    pub fn fixture() -> Value {
        json!({
            "header": {"patch": "16.23", "lane": "jungle", "wr": 53.4, "pr": 5.1, "br": 7.0, "n": 13000, "rank": 7, "rankTotal": 79},
            "nav": {"lanes": {"top": 32.0, "jungle": 60.0, "middle": 5.5}},
            "summary": {
                "pick": {
                    "skillorder": {"id": 312334313141122u64, "n": 3800, "wr": 60.0},
                    "runes": {"wr": 53.5, "n": 12800, "set": {"pri": [9923, 8126, 8140, 8106], "sec": [8224, 8234], "mod": [5008, 5010, 5001]}}
                },
                "win": {"runes": {"wr": 54.0, "n": 900, "set": {"pri": [9923, 8126, 8140, 8106], "sec": [8446, 8242], "mod": [5005, 5008, 5013]}}}
            },
            "spells": [["4_11", 53.4, 98.8, 13100], ["6_11", 53.3, 1.1, 150]],
            "skillOrder": [["EQW", 54.1, 78.0, 10000], ["EWQ", 55.7, 19.3, 2500]],
            "startSet": [["1102_2003", 51.8, 32.7, 4300], ["1102", 53.6, 27.2, 3600], ["2003_2003", 0, 0.02, 2]],
            "boots": [[3009, 55.4, 58.6, 7800, 11], [3111, 51.7, 19.3, 2500, 15], [9999, 31.6, 0.9, 117, 0]],
            "item1": [[3152, 53.2, 27.4, 3600, 10], [3084, 53.5, 69.0, 9100, 11], [6664, 56.0, 0.2, 25, 11]],
            "item2": [[3742, 55.7, 51.9, 6900, 19]],
            "item3": [], "item4": [], "item5": []
        })
    }

    #[test]
    fn parses_fixture() {
        let b = parse_build_full(&fixture()).unwrap();
        assert_eq!(b.patch, "16.23");
        assert_eq!(b.lane, "jungle");
        let r = b.runes_pick.as_ref().unwrap();
        assert!(r.is_complete());
        assert_eq!(
            r.selected_perk_ids(),
            vec![9923, 8126, 8140, 8106, 8224, 8234, 5008, 5010, 5001]
        );
        assert_eq!(b.spells[0].ids, vec!["4", "11"]);
        assert_eq!(b.skill_priority[0].ids, vec!["E", "Q", "W"]);
        assert_eq!(b.skill_order.as_deref(), Some("EQWEEREQEQRQQWW"));
        assert_eq!(b.start_sets[0].ids, vec!["1102", "2003"]);
        // ordenado por partidas, não pela ordem da resposta
        assert_eq!(b.slots[0][0].id, 3084);
        assert_eq!(b.slots.len(), 5);
        assert_eq!(b.lanes[0].0, "jungle");
    }

    #[test]
    fn error_status() {
        assert!(parse_build_full(&json!({"status": 4042})).is_err());
    }
}
