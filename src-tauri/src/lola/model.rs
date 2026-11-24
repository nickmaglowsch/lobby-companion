//! Resposta `ep=build-full` do lolalytics → [`Build`].
//!
//! Formatos observados em 2026-11-19 (patch 16.23):
//! - listas de itens/botas: `[itemId, wr, pr, partidas, tempo]`;
//! - `startSet`/`spells`/`skillOrder`: `["1102_2003", wr, pr, partidas]`;
//! - `summary.pick|win.runes.set`: `{pri: [4], sec: [2], mod: [3]}` (fragmentos na ordem das fileiras);
//! - `summary.pick.skillorder.id`: dígitos 1–4 = Q/W/E/R por nível.
//!
//! Campos extras (2026-11-24, patch 16.23; todos opcionais: o build de confronto não traz
//! `graph` nem `header.damage`, e `builtBootSet3` pode vir vazio):
//! - `builtBootSet3`: `[["3068_3047_2502", wr, pr, partidas], ...]` = os 3 primeiros itens
//!   (botas incluídas) na ordem de compra;
//! - `winningItem`/`popularItem`: `[[itemId, wr, pr, partidas, tempo], ...]` (o primeiro vem
//!   ordenado por WR, muitas vezes com amostra pequena);
//! - `earlyItem`: `[[itemId, wr, pr, partidas], ...]` (o que se compra cedo, primeira volta);
//! - `skill6`/`skill10`: `[[211314, wr, pr, partidas], ...]` (número; dígitos 1–4 = Q/W/E/R);
//! - `runes.stats`: ver [`rune_stats`];
//! - `graph`: `{dates, wr: {all, <elo>…}, wrs: {…}, pr, n, br}` (~30 dias, por dia);
//! - `header.damage`: `{physical, magic, true}` (dano médio por partida).

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

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

/// Seu win rate contra um campeão do outro time (lista `enemy` do build geral).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Matchup {
    /// Id do campeão inimigo.
    pub id: i64,
    /// Seu win rate contra ele.
    pub wr: f64,
    /// % das suas partidas em que ele estava nessa lane do outro lado.
    pub pr: f64,
    pub n: u64,
}

/// Estatística de uma runa: `pr` = % das partidas com ela, `wr` = win rate com ela.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PerkStat {
    pub pr: f64,
    pub wr: f64,
    pub n: u64,
}

/// WR/PR de cada runa (`runes.stats`), separado pelo lugar em que ela aparece na página.
#[derive(Debug, Clone, Serialize, Default)]
pub struct RuneStats {
    /// Runa na árvore primária (keystones e fileiras). Os fragmentos das fileiras de ataque
    /// e de defesa também ficam aqui (chave sem sufixo).
    pub primary: HashMap<i64, PerkStat>,
    /// Runa na árvore secundária.
    pub secondary: HashMap<i64, PerkStat>,
    /// Fragmentos da fileira do meio (chave com sufixo `f`, ex. `"5008f"`).
    pub flex: HashMap<i64, PerkStat>,
}

/// Win rate dos últimos ~30 dias (`graph`, linha `all` = todos os elos).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trend {
    pub dates: Vec<String>,
    /// WR do dia.
    pub wr: Vec<f64>,
    /// WR suavizado pelo lolalytics (média móvel).
    pub wrs: Vec<f64>,
    pub n: Vec<u64>,
    /// WR dos últimos 7 dias (média ponderada pelas partidas).
    pub wr_last7: f64,
    /// WR do período inteiro (média ponderada pelas partidas).
    pub wr_period: f64,
}

/// Dano médio por partida (`header.damage`).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Damage {
    pub physical: f64,
    pub magic: f64,
    #[serde(rename = "true")]
    pub true_damage: f64,
}

/// Cabeçalho do build de confronto (`/vs/`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VsInfo {
    pub champion_id: i64,
    pub lane: String,
    /// Seu win rate geral (sem filtrar o oponente), para comparar.
    pub usual_wr: f64,
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
    /// `Some` quando é o build contra um campeão específico.
    pub vs: Option<VsInfo>,
    /// Lane do inimigo → seu WR contra cada campeão (só no build geral; usado nos bans).
    #[serde(skip)]
    pub matchups: Vec<(String, Vec<Matchup>)>,
    /// Núcleo: os 3 primeiros itens (com botas) na ordem de compra (`builtBootSet3`), por partidas.
    pub core: Vec<Combo>,
    /// Itens de maior WR (`winningItem`), do maior para o menor WR.
    pub winning_items: Vec<ItemStat>,
    /// Itens comprados cedo, primeira volta (`earlyItem`), por partidas.
    pub early_items: Vec<ItemStat>,
    /// Habilidades até o nível 6 e até o 10 (letras), por partidas.
    pub skill6: Vec<Combo>,
    pub skill10: Vec<Combo>,
    pub rune_stats: Option<RuneStats>,
    pub trend: Option<Trend>,
    pub damage: Option<Damage>,
}

impl Build {
    /// Amostra mínima para um item contar. Num confronto há bem menos partidas,
    /// então o corte acompanha o tamanho da amostra (mesma regra na janela, `minGamesFor`).
    pub fn min_games(&self, configured: u64) -> u64 {
        if self.vs.is_some() {
            configured.min((self.n / 20).max(10))
        } else {
            configured
        }
    }
}

/// Quantas linhas de cada lista longa guardamos (o snapshot vai inteiro para a janela).
const KEEP_CORE: usize = 10;
const KEEP_ITEMS: usize = 20;
const KEEP_SKILLS: usize = 8;

/// `runes.stats: {"<perkId>": [[pr, wr, n], ...]}`.
///
/// Conferido na resposta real (Sion selva, 16.23): runas de árvore têm até 2 linhas,
/// a 1ª = na árvore primária e a 2ª = na secundária. As keystones só têm a 1ª, e a soma
/// das PRs da 1ª linha de cada fileira bate com a soma das keystones da mesma árvore
/// (ex.: Determinação 66,7 + 1,9 + 0,3 nas keystones ≈ 43,2 + 25,1 + 0,7 na 1ª fileira);
/// a runa secundária mais jogada tem PR alto só na 2ª linha (70% contra 0,5%).
/// Fragmentos têm 1 linha; a fileira do meio vem com sufixo `f` (`"5008f"`), as de
/// ataque e defesa sem sufixo (os ids não se repetem entre essas duas).
fn rune_stats(v: Option<&Value>) -> Option<RuneStats> {
    let m = v?.as_object()?;
    let stat = |row: &Value| -> Option<PerkStat> {
        let r = row.as_array()?;
        Some(PerkStat {
            pr: f(r.first()),
            wr: f(r.get(1)),
            n: u(r.get(2)),
        })
    };
    let mut out = RuneStats::default();
    for (key, rows) in m {
        let rows = rows.as_array().map(Vec::as_slice).unwrap_or_default();
        if let Some(id) = key.strip_suffix('f').and_then(|k| k.parse::<i64>().ok()) {
            if let Some(s) = rows.first().and_then(stat) {
                out.flex.insert(id, s);
            }
            continue;
        }
        let Ok(id) = key.parse::<i64>() else {
            continue;
        };
        if let Some(s) = rows.first().and_then(stat) {
            out.primary.insert(id, s);
        }
        if let Some(s) = rows.get(1).and_then(stat) {
            out.secondary.insert(id, s);
        }
    }
    (!out.primary.is_empty() || !out.flex.is_empty()).then_some(out)
}

/// Média ponderada pelas partidas.
fn weighted(wr: &[f64], n: &[u64]) -> Option<f64> {
    let total: u64 = n.iter().take(wr.len()).sum();
    (total > 0).then(|| wr.iter().zip(n).map(|(w, k)| w * *k as f64).sum::<f64>() / total as f64)
}

/// `graph: {dates: [...], wr: {all: [...]}, wrs: {all: [...]}, n: {all: [...]}}`.
/// Usa a linha `all` (todos os elos): o gráfico não tem o elo filtrado (`emerald_plus`).
fn trend(v: Option<&Value>) -> Option<Trend> {
    let g = v?;
    let floats = |k: &str| -> Vec<f64> {
        g.pointer(&format!("/{k}/all"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|x| x.as_f64().unwrap_or(0.0))
            .collect()
    };
    let dates: Vec<String> = g
        .get("dates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|d| d.as_str().unwrap_or("").to_string())
        .collect();
    let wr = floats("wr");
    let n: Vec<u64> = g
        .pointer("/n/all")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|x| u(Some(x)))
        .collect();
    let len = wr.len().min(n.len());
    if len < 2 {
        return None;
    }
    let (wr, n) = (wr[..len].to_vec(), n[..len].to_vec());
    let mut wrs = floats("wrs");
    if wrs.len() != len {
        wrs = wr.clone();
    }
    let recent = len.saturating_sub(7);
    Some(Trend {
        wr_last7: weighted(&wr[recent..], &n[recent..])?,
        wr_period: weighted(&wr, &n)?,
        dates: dates.into_iter().take(len).collect(),
        wr,
        wrs,
        n,
    })
}

fn damage(v: Option<&Value>) -> Option<Damage> {
    let d = v?;
    let out = Damage {
        physical: f(d.get("physical")),
        magic: f(d.get("magic")),
        true_damage: f(d.get("true")),
    };
    (out.physical + out.magic + out.true_damage > 0.0).then_some(out)
}

/// `skill6`/`skill10`: `[[211314, wr, pr, n], ...]` → letras (`["W", "Q", "Q", ...]`).
fn skill_list(v: Option<&Value>) -> Vec<Combo> {
    let mut out: Vec<Combo> = v
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let r = row.as_array()?;
            let letters = skill_order_letters(r.first()?)?;
            Some(Combo {
                ids: letters.chars().map(String::from).collect(),
                wr: f(r.get(1)),
                pr: f(r.get(2)),
                n: u(r.get(3)),
            })
        })
        .collect();
    out.sort_by_key(|a| std::cmp::Reverse(a.n));
    out.truncate(KEEP_SKILLS);
    out
}

/// `enemy: {lane: [[id, wr, d1, d2, pr, n], ...]}`.
fn matchups(v: Option<&Value>) -> Vec<(String, Vec<Matchup>)> {
    let Some(m) = v.and_then(Value::as_object) else {
        return Vec::new();
    };
    m.iter()
        .map(|(lane, rows)| {
            let list = rows
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    let r = row.as_array()?;
                    Some(Matchup {
                        id: r.first()?.as_i64()?,
                        wr: f(r.get(1)),
                        pr: f(r.get(4)),
                        n: u(r.get(5)),
                    })
                })
                .collect();
            (lane.clone(), list)
        })
        .collect()
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
        vs: header
            .get("vs")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .map(|id| VsInfo {
                champion_id: id,
                lane: header
                    .get("vsLane")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                usual_wr: f(header.get("usualWinRate")),
            }),
        matchups: matchups(v.get("enemy")),
        core: {
            let mut c = combo_list(v.get("builtBootSet3"));
            c.truncate(KEEP_CORE);
            c
        },
        winning_items: {
            let mut w = item_list(v.get("winningItem"));
            w.sort_by(|a, b| b.wr.total_cmp(&a.wr));
            w
        },
        early_items: {
            let mut e = item_list(v.get("earlyItem"));
            e.truncate(KEEP_ITEMS);
            e
        },
        skill6: skill_list(v.get("skill6")),
        skill10: skill_list(v.get("skill10")),
        rune_stats: rune_stats(v.pointer("/runes/stats")),
        trend: trend(v.get("graph")),
        damage: damage(header.get("damage")),
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    /// Mesmo formato da resposta real, com números inventados (não redistribuímos dados do lolalytics).
    pub fn fixture() -> Value {
        json!({
            "header": {"patch": "16.23", "lane": "jungle", "wr": 53.4, "pr": 5.1, "br": 7.0, "n": 13000, "rank": 7, "rankTotal": 79,
                "damage": {"physical": 6000.0, "magic": 3000.0, "true": 1000.0}},
            "builtBootSet3": [["3068_3047_2502", 57.1, 10.0, 1300], ["3084_3009_3742", 55.0, 4.0, 500], ["3068_3111_2502", 56.0, 5.0, 150]],
            "winningItem": [[2525, 69.0, 0.5, 50, 26], [3075, 61.0, 2.0, 300, 27], [3009, 60.0, 5.0, 900, 10],
                [3065, 58.0, 8.0, 1000, 26], [3742, 57.0, 52.0, 6900, 19], [3152, 52.0, 3.0, 400, 20], [2003, 70.0, 9.0, 999, 1]],
            "earlyItem": [[1028, 54.0, 69.0, 8000], [3340, 50.0, 90.0, 9000], [2055, 55.0, 31.0, 3700],
                [1001, 54.5, 55.0, 6600], [1102, 53.0, 40.0, 5000], [3068, 58.0, 15.0, 1800], [9999, 50.0, 1.0, 120]],
            "skill6": [[123114, 56.0, 27.0, 3200], [211314, 54.7, 32.3, 3800], [213114, 60.0, 1.0, 20]],
            "skill10": [[2113141212, 56.0, 26.0, 3000], [1231141212, 54.0, 23.0, 2700]],
            "runes": {"stats": {
                "9923": [[60.0, 53.0, 7000]], "8112": [[20.0, 56.0, 2400]],
                "8126": [[40.0, 53.0, 4800], [1.0, 50.0, 100]], "8139": [[20.0, 55.0, 2400], [0.5, 40.0, 60]],
                "8224": [[2.0, 50.0, 240], [30.0, 52.0, 3600]], "8226": [[1.0, 51.0, 120], [20.0, 55.5, 2400]],
                "5008": [[50.0, 53.0, 6000]], "5005": [[45.0, 54.0, 5400]],
                "5008f": [[80.0, 53.0, 9600]], "5010f": [[10.0, 56.0, 1200]], "5001": [[90.0, 54.0, 10800]],
                "x": [[1, 2, 3]]
            }},
            "graph": {
                "dates": ["2026-11-01", "2026-11-02", "2026-11-03", "2026-11-04", "2026-11-05",
                    "2026-11-06", "2026-11-07", "2026-11-08", "2026-11-09", "2026-11-10"],
                "wr": {"all": [50.0, 50.0, 50.0, 52.0, 52.0, 52.0, 52.0, 52.0, 52.0, 52.0], "gold": [1, 2, 3]},
                "wrs": {"all": [50.0, 50.0, 50.0, 50.5, 51.0, 51.5, 52.0, 52.0, 52.0, 52.0]},
                "n": {"all": [100, 100, 100, 100, 100, 100, 100, 100, 100, 100]}
            },
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
            "item3": [], "item4": [], "item5": [],
            "enemy_h": ["id", "wr", "d1", "d2", "pr", "n"],
            "enemy": {
                "jungle": [[234, 47.0, -2.0, -3.0, 9.0, 1100], [64, 55.0, 2.0, 1.0, 8.0, 1000], [28, 40.0, -9.0, -9.5, 0.1, 12]],
                "top": [[122, 50.0, 0.0, -1.0, 6.0, 700]]
            }
        })
    }

    /// Build de confronto: mesmo formato, sem `enemy`, com `vs`/`vsLane`/`usualWinRate` no header.
    pub fn fixture_vs() -> Value {
        let mut v = fixture();
        let h = v["header"].as_object_mut().unwrap();
        h.insert("vs".into(), json!(234));
        h.insert("vsLane".into(), json!("jungle"));
        h.insert("usualWinRate".into(), json!(53.4));
        h.insert("n".into(), json!(600));
        h.insert("wr".into(), json!(48.0));
        v.as_object_mut().unwrap().remove("enemy");
        v
    }

    #[test]
    fn parses_matchup_build() {
        let b = parse_build_full(&fixture_vs()).unwrap();
        let vs = b.vs.as_ref().unwrap();
        assert_eq!((vs.champion_id, vs.lane.as_str()), (234, "jungle"));
        assert!(b.matchups.is_empty());
        // 600 partidas: o corte cai de 200 para 30
        assert_eq!(b.min_games(200), 30);
        let geral = parse_build_full(&fixture()).unwrap();
        assert!(geral.vs.is_none());
        assert_eq!(geral.min_games(200), 200);
        let jg = &geral
            .matchups
            .iter()
            .find(|(l, _)| l == "jungle")
            .unwrap()
            .1;
        assert_eq!(
            jg[0],
            Matchup {
                id: 234,
                wr: 47.0,
                pr: 9.0,
                n: 1100
            }
        );
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

    #[test]
    fn parses_core_items_and_skills() {
        let b = parse_build_full(&fixture()).unwrap();
        assert_eq!(b.core[0].ids, vec!["3068", "3047", "2502"]);
        assert_eq!(b.core[0].n, 1300);
        assert_eq!(b.core.len(), 3);
        // winningItem: do maior para o menor WR
        assert_eq!(b.winning_items[0].id, 2003);
        assert_eq!(b.winning_items[1].id, 2525);
        // earlyItem: por partidas
        assert_eq!(b.early_items[0].id, 3340);
        assert_eq!(b.early_items[1].id, 1028);
        // skill6/skill10: número → letras, por partidas
        assert_eq!(b.skill6[0].ids.concat(), "WQQEQR");
        assert_eq!(b.skill6[0].n, 3800);
        assert_eq!(b.skill6[2].ids.concat(), "WQEQQR");
        assert_eq!(b.skill10[0].ids.concat(), "WQQEQRQWQW");
    }

    #[test]
    fn parses_rune_stats() {
        let b = parse_build_full(&fixture()).unwrap();
        let r = b.rune_stats.as_ref().unwrap();
        assert_eq!(
            r.primary[&8126],
            PerkStat {
                pr: 40.0,
                wr: 53.0,
                n: 4800
            }
        );
        assert_eq!(r.secondary[&8226].wr, 55.5);
        assert!(!r.secondary.contains_key(&9923), "keystone só na primária");
        assert_eq!(r.flex[&5010].n, 1200);
        assert_eq!(r.primary[&5008].pr, 50.0, "sem sufixo = ataque/defesa");
        assert_eq!(r.flex[&5008].pr, 80.0);
        let empty = parse_build_full(&json!({"header": {}, "runes": {"stats": {}}})).unwrap();
        assert!(empty.rune_stats.is_none());
    }

    #[test]
    fn parses_trend_and_damage() {
        let b = parse_build_full(&fixture()).unwrap();
        let t = b.trend.as_ref().unwrap();
        assert_eq!(t.dates.len(), 10);
        assert_eq!(t.wrs[3], 50.5);
        // últimos 7 dias: 52; período: (3×50 + 7×52) / 10 = 51,4
        assert!((t.wr_last7 - 52.0).abs() < 1e-9);
        assert!((t.wr_period - 51.4).abs() < 1e-9);
        let d = b.damage.as_ref().unwrap();
        assert_eq!(
            (d.physical, d.magic, d.true_damage),
            (6000.0, 3000.0, 1000.0)
        );
        assert_eq!(
            serde_json::to_value(d).unwrap(),
            json!({"physical": 6000.0, "magic": 3000.0, "true": 1000.0})
        );
    }

    #[test]
    fn extras_are_optional() {
        // Confronto: sem graph/damage; resposta mínima: nada quebra.
        let mut v = fixture_vs();
        v.as_object_mut().unwrap().remove("graph");
        v["header"].as_object_mut().unwrap().remove("damage");
        v["builtBootSet3"] = json!([]);
        let b = parse_build_full(&v).unwrap();
        assert!(b.trend.is_none() && b.damage.is_none() && b.core.is_empty());
        let min = parse_build_full(&json!({"header": {"wr": 50.0}})).unwrap();
        assert!(min.winning_items.is_empty() && min.skill6.is_empty() && min.rune_stats.is_none());
    }
}
