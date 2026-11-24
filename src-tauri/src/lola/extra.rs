//! Respostas `ep=list`, `ep=counter` e `ep=build-team` do lolalytics (conferidas em 2026-11-24, patch 16.23).
//!
//! - `list` (`lane=L`): `{avgWr, cid: {"<id>": {rank, tier, lane, defaultLane, pctLane, wr, avgWrDelta,
//!   pr, br, pbi, games, topRank, topWr, …}}}`. Todos os campeões (173), inclusive quem quase não
//!   joga na lane (`pctLane` = % das partidas do campeão nessa lane; `pr` soma ~200 porque há dois
//!   jogadores por lane). `avgWr` é a média ponderada da lane (~51,6, acima de 50 no lolalytics).
//! - `counter` (`c=X&lane=L&vslane=V`): `{stats: {cid, lane, vsLane, wr (texto), avgWr, lanes, …},
//!   counters: [{cid, vsWr, n, d1, d2, allWr, defaultLane}]}` = oponentes de X que jogaram na lane V.
//!   **`vsWr` é o WR de X contra `cid`** (bate com `enemy` do build geral de X); `allWr` = 100 − WR
//!   geral de `cid` (o que um campeão qualquer faz contra ele); `d1 = vsWr − allWr`;
//!   `d2 = d1 − (wr(X) − (100 − avgWr))` = efeito do confronto além da força geral dos dois.
//! - `build-team` (`c=X&lane=L`): `{team_h: ["id","wr","d1","d2","pr","n"], team: {lane: [[…]]}}`
//!   = WR de X quando o aliado da lane é `id`; `pr` = % das partidas de X com ele (cada lane soma
//!   100); `d1 = wr − (wr(X) + wr(id))/2`; `d2 = wr − (wr(X) + wr(id) − avgWr)` = sinergia além da
//!   força geral dos dois.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Número que às vezes vem como texto (`"54.53"`).
fn f(v: Option<&Value>) -> f64 {
    v.and_then(|x| {
        x.as_f64()
            .or_else(|| x.as_str().and_then(|s| s.trim().parse().ok()))
    })
    .unwrap_or(0.0)
}

fn u(v: Option<&Value>) -> u64 {
    v.and_then(|x| x.as_u64().or_else(|| x.as_f64().map(|f| f.max(0.0) as u64)))
        .unwrap_or(0)
}

fn s(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

fn status_error(v: &Value) -> Result<(), String> {
    match v.get("status").and_then(Value::as_i64) {
        Some(code) => Err(format!(
            "lolalytics sem dados para essa combinação (status {code})"
        )),
        None => Ok(()),
    }
}

/// Um campeão na tier list de uma lane.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TierEntry {
    pub id: i64,
    pub wr: f64,
    /// Pick rate na lane (a lista soma ~200).
    pub pr: f64,
    pub br: f64,
    pub games: u64,
    /// % das partidas do campeão que são nessa lane.
    pub pct_lane: f64,
    /// Lane mais jogada do campeão.
    pub default_lane: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TierList {
    pub lane: String,
    pub avg_wr: f64,
    pub entries: HashMap<i64, TierEntry>,
}

impl TierList {
    pub fn get(&self, id: i64) -> Option<&TierEntry> {
        self.entries.get(&id)
    }

    /// Soma dos pick rates (para virar a chance de o campeão estar numa vaga da lane).
    pub fn pr_total(&self) -> f64 {
        self.entries.values().map(|e| e.pr).sum()
    }
}

pub fn parse_list(v: &Value) -> Result<TierList, String> {
    status_error(v)?;
    let cid = v
        .get("cid")
        .and_then(Value::as_object)
        .ok_or("tier list do lolalytics sem 'cid'")?;
    let mut lane = String::new();
    let entries: HashMap<i64, TierEntry> = cid
        .iter()
        .filter_map(|(k, e)| {
            let id: i64 = k.parse().ok()?;
            if lane.is_empty() {
                lane = s(e.get("lane"));
            }
            Some((
                id,
                TierEntry {
                    id,
                    wr: f(e.get("wr")),
                    pr: f(e.get("pr")),
                    br: f(e.get("br")),
                    games: u(e.get("games")),
                    pct_lane: f(e.get("pctLane")),
                    default_lane: s(e.get("defaultLane")),
                },
            ))
        })
        .collect();
    let avg = f(v.get("avgWr"));
    Ok(TierList {
        lane,
        avg_wr: if avg > 0.0 { avg } else { 50.0 },
        entries,
    })
}

/// Um par de campeões (confronto ou dupla) com o WR do campeão consultado.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PairStat {
    pub id: i64,
    /// WR do campeão consultado nesse par.
    pub wr: f64,
    pub d1: f64,
    /// Efeito do par além da força geral dos dois (ver o topo do módulo).
    pub d2: f64,
    pub n: u64,
}

/// `ep=counter`: o campeão consultado contra os oponentes de uma lane.
#[derive(Debug, Clone, PartialEq)]
pub struct Counters {
    pub champion_id: i64,
    pub lane: String,
    pub vs_lane: String,
    pub wr: f64,
    /// Distribuição das partidas do campeão consultado por lane.
    pub lanes: Vec<(String, f64)>,
    pub rows: Vec<PairStat>,
}

impl Counters {
    pub fn row(&self, id: i64) -> Option<&PairStat> {
        self.rows.iter().find(|r| r.id == id)
    }
}

pub fn parse_counter(v: &Value) -> Result<Counters, String> {
    status_error(v)?;
    let stats = v.get("stats").unwrap_or(&Value::Null);
    let rows = v
        .get("counters")
        .and_then(Value::as_array)
        .ok_or("resposta do lolalytics sem 'counters'")?
        .iter()
        .filter_map(|r| {
            Some(PairStat {
                id: r.get("cid")?.as_i64()?,
                wr: f(r.get("vsWr")),
                d1: f(r.get("d1")),
                d2: f(r.get("d2")),
                n: u(r.get("n")),
            })
        })
        .collect();
    let mut lanes: Vec<(String, f64)> = stats
        .get("lanes")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .map(|(k, x)| (k.clone(), x.as_f64().unwrap_or(0.0)))
                .collect()
        })
        .unwrap_or_default();
    lanes.sort_by(|a, b| b.1.total_cmp(&a.1));
    Ok(Counters {
        champion_id: stats.get("cid").and_then(Value::as_i64).unwrap_or(0),
        lane: s(stats.get("lane")),
        vs_lane: s(stats.get("vsLane")),
        wr: f(stats.get("wr")),
        lanes,
        rows,
    })
}

/// `ep=build-team`: o campeão consultado com cada aliado, por lane do aliado.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Team {
    pub lanes: Vec<(String, Vec<PairStat>)>,
}

impl Team {
    /// O aliado `id` na `lane`.
    pub fn row(&self, lane: &str, id: i64) -> Option<&PairStat> {
        self.lanes
            .iter()
            .find(|(l, _)| l == lane)
            .and_then(|(_, rows)| rows.iter().find(|r| r.id == id))
    }
}

pub fn parse_build_team(v: &Value) -> Result<Team, String> {
    status_error(v)?;
    let team = v
        .get("team")
        .and_then(Value::as_object)
        .ok_or("resposta do lolalytics sem 'team'")?;
    // Posição das colunas pelo cabeçalho (padrão: id, wr, d1, d2, pr, n).
    let header: Vec<String> = v
        .get("team_h")
        .and_then(Value::as_array)
        .map(|h| h.iter().map(|x| s(Some(x))).collect())
        .unwrap_or_default();
    let col = |name: &str, default: usize| header.iter().position(|h| h == name).unwrap_or(default);
    let (ci, cw, c1, c2, cn) = (
        col("id", 0),
        col("wr", 1),
        col("d1", 2),
        col("d2", 3),
        col("n", 5),
    );
    let lanes = team
        .iter()
        .map(|(lane, rows)| {
            let list = rows
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|row| {
                    let r = row.as_array()?;
                    Some(PairStat {
                        id: r.get(ci)?.as_i64()?,
                        wr: f(r.get(cw)),
                        d1: f(r.get(c1)),
                        d2: f(r.get(c2)),
                        n: u(r.get(cn)),
                    })
                })
                .collect();
            (lane.clone(), list)
        })
        .collect();
    Ok(Team { lanes })
}

/// Maestria do jogador local (`GET /lol-champion-mastery/v1/local-player/champion-mastery`):
/// `[(campeão, pontos)]`, do maior para o menor. Só lê `championId` e `championPoints`.
pub fn parse_mastery(v: &Value) -> Vec<(i64, u64)> {
    let list = v
        .as_array()
        .or_else(|| v.get("masteries").and_then(Value::as_array));
    let mut out: Vec<(i64, u64)> = list
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let id = m.get("championId")?.as_i64()?;
            (id > 0).then(|| (id, u(m.get("championPoints"))))
        })
        .collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde_json::json;

    /// Tier list da selva: mesmo formato da resposta real, números inventados.
    pub fn list_fixture() -> Value {
        json!({
            "queue": 420, "view": "list", "avgWr": 51.0, "analysed": 1000000,
            "cid": {
                "31":  {"rank": 5, "tier": 3, "lane": "jungle", "defaultLane": "jungle", "pctLane": 60.0, "wr": 53.0, "avgWrDelta": 1.5, "pr": 5.0, "br": 7.0, "pbi": 9, "games": 60000, "topRank": 90, "topWr": "54.00"},
                "234": {"rank": 2, "tier": 2, "lane": "jungle", "defaultLane": "jungle", "pctLane": 95.0, "wr": 52.0, "avgWrDelta": 1.2, "pr": 9.0, "br": 6.0, "pbi": 3, "games": 100000, "topRank": 30, "topWr": "56.00"},
                "64":  {"rank": 9, "tier": 4, "lane": "jungle", "defaultLane": "jungle", "pctLane": 98.0, "wr": 50.0, "avgWrDelta": 1.0, "pr": 4.0, "br": 2.0, "pbi": 2, "games": 50000, "topRank": 50, "topWr": "52.00"},
                "14":  {"rank": 30, "tier": 6, "lane": "jungle", "defaultLane": "top", "pctLane": 15.0, "wr": 54.0, "avgWrDelta": 1.8, "pr": 0.9, "br": 0.8, "pbi": 3, "games": 12000, "topRank": 140, "topWr": "54.00"},
                "99":  {"rank": 0, "tier": 0, "lane": "jungle", "defaultLane": "middle", "pctLane": 0.1, "wr": 40.0, "avgWrDelta": 6.0, "pr": 0.0, "br": 4.0, "pbi": 0, "games": 30, "topRank": 90, "topWr": "55.00"},
                "122": {"rank": 7, "tier": 3, "lane": "jungle", "defaultLane": "top", "pctLane": 12.0, "wr": 51.0, "avgWrDelta": 2.0, "pr": 1.0, "br": 15.0, "pbi": 3, "games": 15000, "topRank": 40, "topWr": "56.00"},
                "876": {"rank": 8, "tier": 4, "lane": "jungle", "defaultLane": "jungle", "pctLane": 99.0, "wr": 55.0, "avgWrDelta": 2.0, "pr": 6.0, "br": 5.0, "pbi": 11, "games": 70000, "topRank": 100, "topWr": "56.00"}
            },
            "response": {"valid": true}
        })
    }

    /// `ep=counter` do Viego (selva) contra a selva, números inventados.
    pub fn counter_fixture() -> Value {
        json!({
            "stats": {"defaultLane": "jungle", "cid": 234, "lane": "jungle", "vsLane": "jungle", "avgWr": 51.0, "br": 6.0, "pr": "9.00", "wr": "52.00",
                      "lanes": {"top": 1.0, "jungle": 96.0, "middle": 2.0, "bottom": 0.5, "support": 0.5},
                      "counters": {"strong": [64], "weak": [31]}},
            "counters": [
                {"cid": 64, "vsWr": 56.0, "n": 900, "d1": 6.0, "d2": 3.0, "allWr": 50.0, "defaultLane": "jungle"},
                {"cid": 31, "vsWr": 45.0, "n": 300, "d1": -2.0, "d2": -5.0, "allWr": 47.0, "defaultLane": "jungle"},
                {"cid": 14, "vsWr": 50.0, "n": 20, "d1": 4.0, "d2": 1.0, "allWr": 46.0, "defaultLane": "top"}
            ],
            "response": {"valid": true}
        })
    }

    /// `ep=build-team` da Lux (meio), números inventados.
    pub fn team_fixture() -> Value {
        json!({
            "team_h": ["id", "wr", "d1", "d2", "pr", "n"],
            "team": {
                "top": [[122, 52.0, 0.5, 0.2, 5.0, 500]],
                "jungle": [[31, 55.0, 2.5, 1.5, 4.0, 600], [876, 50.0, -3.0, -4.0, 6.0, 900], [14, 60.0, 6.0, 5.0, 0.5, 30]],
                "bottom": [], "support": []
            },
            "response": {"valid": true}
        })
    }

    #[test]
    fn parses_list() {
        let l = parse_list(&list_fixture()).unwrap();
        assert_eq!(l.lane, "jungle");
        assert_eq!(l.avg_wr, 51.0);
        assert_eq!(l.entries.len(), 7);
        let sion = l.get(14).unwrap();
        assert_eq!(sion.default_lane, "top");
        assert_eq!((sion.pct_lane, sion.games), (15.0, 12000));
        assert!(parse_list(&json!({"status": 4044})).is_err());
    }

    #[test]
    fn parses_counter_with_text_numbers() {
        let c = parse_counter(&counter_fixture()).unwrap();
        assert_eq!(
            (c.champion_id, c.lane.as_str(), c.vs_lane.as_str()),
            (234, "jungle", "jungle")
        );
        assert_eq!(c.wr, 52.0);
        assert_eq!(c.lanes[0], ("jungle".to_string(), 96.0));
        let cho = c.row(31).unwrap();
        // WR do Viego contra o Cho'Gath
        assert_eq!((cho.wr, cho.d2, cho.n), (45.0, -5.0, 300));
        assert!(parse_counter(&json!({"status": 4042})).is_err());
    }

    #[test]
    fn parses_team_by_header() {
        let t = parse_build_team(&team_fixture()).unwrap();
        let cho = t.row("jungle", 31).unwrap();
        assert_eq!((cho.wr, cho.d1, cho.d2, cho.n), (55.0, 2.5, 1.5, 600));
        assert!(t.row("middle", 31).is_none());
        // colunas em outra ordem continuam certas
        let swapped = json!({"team_h": ["n", "id", "wr", "d2", "d1", "pr"], "team": {"top": [[40, 7, 51.0, 0.3, 0.9, 1.0]]}});
        let r = parse_build_team(&swapped).unwrap();
        let x = r.row("top", 7).unwrap();
        assert_eq!((x.wr, x.d1, x.d2, x.n), (51.0, 0.9, 0.3, 40));
    }

    #[test]
    fn mastery_sorted_and_without_identity() {
        let v = json!([
            {"championId": 31, "championPoints": 900, "championLevel": 7, "puuid": "não-usado"},
            {"championId": 64, "championPoints": 20000, "championLevel": 9},
            {"championId": -1, "championPoints": 5}
        ]);
        assert_eq!(parse_mastery(&v), vec![(64, 20000), (31, 900)]);
        assert_eq!(
            parse_mastery(&json!({"masteries": [{"championId": 1, "championPoints": 2}]})),
            vec![(1, 2)]
        );
        assert!(parse_mastery(&json!({})).is_empty());
    }
}
