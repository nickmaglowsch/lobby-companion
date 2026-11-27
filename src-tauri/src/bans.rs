//! Sugestão de bans pelo seu time: quem mais derruba o win rate dos campeões escolhidos
//! (ou declarados) do seu lado, segundo a lista `enemy` do build geral de cada um no lolalytics.
//!
//! Para cada campeão do time e cada inimigo `e` (em qualquer lane do outro lado):
//! `ganho = pr(e)/100 × (WR do campeão − WR dele contra e) × n/(n + 300)`,
//! ou seja, quantos pontos de win rate somem em média por causa de `e`; o fator `n/(n+300)`
//! puxa para zero os confrontos com pouca amostra. O ganho do time é a média ponderada
//! (o seu campeão pesa 2). É tendência de estatística, não leitura do outro time.
//!
//! Com a métrica WPA (`metric.rs`), cada lane do inimigo vira uma vaga: a sua chance contra cada
//! um (encolhida em log-odds) e o ganho de banir = quanto a sua chance média sobe quando a vaga
//! fica com os outros (sem os indisponíveis, renormalizada).

use crate::champselect::Ally;
use crate::lola::extra::TierEntry;
use crate::lola::model::{Build, Matchup};
use crate::metric::{self, Metric};
use serde::Serialize;
use std::collections::HashMap;

const ME_WEIGHT: f64 = 2.0;
const REASONS: usize = 2;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reason {
    /// Campeão do seu time prejudicado.
    pub ally_champion_id: i64,
    /// Lane do inimigo nesse confronto.
    pub lane: String,
    /// WR do seu campeão contra ele.
    pub wr: f64,
    /// WR geral do seu campeão (para comparar).
    pub base_wr: f64,
    pub n: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BanPick {
    pub champion_id: i64,
    /// Pontos de win rate que o time ganha, em média, se ele sair do jogo.
    pub gain: f64,
    pub reasons: Vec<Reason>,
    /// Só nos bans pela tier list da lane (`picks::tier_bans`): WR, pick e ban rate dele.
    pub tier: Option<TierEntry>,
}

/// Ganho de banir cada inimigo de uma lane, para um campeão do seu time.
fn lane_gains<'a>(
    build: &Build,
    list: &'a [Matchup],
    unavailable: &[i64],
    metric: Metric,
) -> Vec<(&'a Matchup, f64)> {
    let rows: Vec<&Matchup> = list
        .iter()
        .filter(|m| m.n > 0 && !unavailable.contains(&m.id))
        .collect();
    match metric {
        Metric::Wr => rows
            .into_iter()
            .map(|m| {
                let harm = (build.wr - m.wr) * metric::shrink_factor(m.n);
                (m, m.pr / 100.0 * harm)
            })
            .collect(),
        Metric::Wpa => {
            // A vaga é a lane inteira (100% das suas partidas), menos quem já está indisponível.
            let listed: f64 = list.iter().filter(|m| m.n > 0).map(|m| m.pr).sum();
            let gone: f64 = list
                .iter()
                .filter(|m| m.n > 0 && unavailable.contains(&m.id))
                .map(|m| m.pr)
                .sum();
            let slot: Vec<(i64, f64, f64)> = rows
                .iter()
                .map(|m| (m.id, m.pr, metric::shrunk_prob(build.wr, m.wr, m.n)))
                .collect();
            rows.into_iter()
                .zip(metric::ban_gains(&slot, listed.max(100.0) - gone, build.wr))
                .map(|(m, (_, g))| (m, g))
                .collect()
        }
    }
}

pub fn suggest(
    inputs: &[(Ally, &Build)],
    unavailable: &[i64],
    top: usize,
    metric: Metric,
) -> Vec<BanPick> {
    rank(
        inputs.iter().map(|(a, b)| (a, *b)),
        None,
        unavailable,
        top,
        metric,
    )
}

/// Um motivo antes de virar [`Reason`] (só os `REASONS` maiores de cada inimigo viram).
struct Hit<'a> {
    contrib: f64,
    ally_champion_id: i64,
    lane: &'a str,
    matchup: &'a Matchup,
    base_wr: f64,
}

/// A conta de [`suggest`], só com as lanes do inimigo iguais a `only_lane` (se houver).
fn rank<'a, I>(
    inputs: I,
    only_lane: Option<&str>,
    unavailable: &[i64],
    top: usize,
    metric: Metric,
) -> Vec<BanPick>
where
    I: Iterator<Item = (&'a Ally, &'a Build)> + Clone,
{
    let weight = |a: &Ally| if a.me { ME_WEIGHT } else { 1.0 };
    let total: f64 = inputs.clone().map(|(a, _)| weight(a)).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    let mut acc: HashMap<i64, (f64, Vec<Hit<'a>>)> = HashMap::new();
    for (ally, build) in inputs {
        let w = weight(ally);
        let lanes = build
            .matchups
            .iter()
            .filter(|(l, _)| only_lane.is_none_or(|o| o == l));
        for (lane, list) in lanes {
            for (m, contrib) in lane_gains(build, list, unavailable, metric) {
                let e = acc.entry(m.id).or_default();
                e.0 += w * contrib;
                if contrib > 0.0 {
                    e.1.push(Hit {
                        contrib,
                        ally_champion_id: ally.champion_id,
                        lane,
                        matchup: m,
                        base_wr: build.wr,
                    });
                }
            }
        }
    }
    let mut out: Vec<BanPick> = acc
        .into_iter()
        .filter(|(_, (score, _))| score / total > 0.0)
        .map(|(id, (score, mut hits))| {
            hits.sort_by(|a, b| b.contrib.total_cmp(&a.contrib));
            BanPick {
                champion_id: id,
                gain: score / total,
                reasons: hits
                    .into_iter()
                    .take(REASONS)
                    .map(|h| Reason {
                        ally_champion_id: h.ally_champion_id,
                        lane: h.lane.to_string(),
                        wr: h.matchup.wr,
                        base_wr: h.base_wr,
                        n: h.matchup.n,
                    })
                    .collect(),
                tier: None,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.gain
            .total_cmp(&a.gain)
            .then(a.champion_id.cmp(&b.champion_id))
    });
    out.truncate(top);
    out
}

/// Bans contra o **seu** pick: só os oponentes da sua lane, com a mesma conta de [`suggest`]
/// (`pr × queda de WR × encolhimento`), ou seja, confronto ruim **e** provável de aparecer.
/// `exclude` tira quem já está na lista do time, para as duas listas não se repetirem (só da
/// lista: na conta do WPA eles continuam na vaga).
pub fn lane_suggest(
    me: &Ally,
    build: &Build,
    unavailable: &[i64],
    exclude: &[i64],
    top: usize,
    metric: Metric,
) -> Vec<BanPick> {
    let lane = if build.lane.is_empty() {
        me.lane.as_deref().unwrap_or_default()
    } else {
        &build.lane
    };
    let mut out = rank(
        std::iter::once((me, build)),
        Some(lane),
        unavailable,
        usize::MAX,
        metric,
    );
    out.retain(|p| !exclude.contains(&p.champion_id));
    out.truncate(top);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lola::model::{parse_build_full, tests::fixture};

    fn ally(id: i64, me: bool) -> Ally {
        Ally {
            champion_id: id,
            lane: Some("jungle".into()),
            me,
        }
    }

    #[test]
    fn ranks_by_lost_win_rate_and_skips_unavailable() {
        // Fixture: WR geral 53,4. Inimigos da selva: 234 (47% em 1.100, 9% das partidas),
        // 64 (55%, favorável), 28 (40% mas 12 partidas e 0,1%); topo: 122 (50%, 6%).
        let b = parse_build_full(&fixture()).unwrap();
        let picks = suggest(&[(ally(31, true), &b)], &[], 5, Metric::Wr);
        let ids: Vec<i64> = picks.iter().map(|p| p.champion_id).collect();
        assert_eq!(ids, vec![234, 122, 28]);
        let first = &picks[0];
        // 0,09 × 6,4 × 1100/1400
        assert!((first.gain - 0.09 * 6.4 * 1100.0 / 1400.0).abs() < 1e-9);
        assert_eq!(first.reasons[0].ally_champion_id, 31);
        assert_eq!(first.reasons[0].lane, "jungle");
        // confronto favorável não vira sugestão
        assert!(!ids.contains(&64));
        // banido/escolhido some da lista
        let without = suggest(&[(ally(31, true), &b)], &[234], 5, Metric::Wr);
        assert_eq!(without[0].champion_id, 122);
    }

    #[test]
    fn team_average_weights_me_double() {
        let b = parse_build_full(&fixture()).unwrap();
        let mut other = b.clone();
        other.matchups.clear();
        let solo = suggest(&[(ally(31, true), &b)], &[], 1, Metric::Wr)[0].gain;
        // o meu campeão pesa 2, o aliado sem dados pesa 1: 2/3 do ganho
        let team = suggest(
            &[(ally(31, true), &b), (ally(99, false), &other)],
            &[],
            1,
            Metric::Wr,
        )[0]
        .gain;
        assert!((team - solo * 2.0 / 3.0).abs() < 1e-9);
        assert!(suggest(&[], &[], 5, Metric::Wr).is_empty());
    }

    #[test]
    fn wpa_renormalizes_each_enemy_lane() {
        let b = parse_build_full(&fixture()).unwrap();
        let picks = suggest(&[(ally(31, true), &b)], &[], 5, Metric::Wpa);
        let ids: Vec<i64> = picks.iter().map(|p| p.champion_id).collect();
        assert_eq!(ids, vec![234, 122, 28]);
        // selva: 234 (9%), 64 (8%), 28 (0,1%); o resto dos 100% entra com o WR geral (53,4)
        let p = |wr: f64, n: u64| metric::shrunk_prob(53.4, wr, n);
        let now =
            (9.0 * p(47.0, 1100) + 8.0 * p(55.0, 1000) + 0.1 * p(40.0, 12) + 82.9 * 53.4) / 100.0;
        assert!((picks[0].gain - 9.0 * (now - p(47.0, 1100)) / 91.0).abs() < 1e-9);
        // e vale mais do que a conta por WR (a vaga que sobra fica com os outros)
        let wr = suggest(&[(ally(31, true), &b)], &[], 1, Metric::Wr)[0].gain;
        assert!(picks[0].gain > wr);
        // indisponível sai da vaga e da lista
        let without = suggest(&[(ally(31, true), &b)], &[234], 5, Metric::Wpa);
        assert_eq!(without[0].champion_id, 122);
        // a lista "contra você" tira os do time só da exibição: a conta da vaga não muda
        let me = ally(31, true);
        let lane = lane_suggest(&me, &b, &[], &[234], 5, Metric::Wpa);
        let full = lane_suggest(&me, &b, &[], &[], 5, Metric::Wpa);
        assert_eq!(lane[0].champion_id, 28);
        assert!((lane[0].gain - full[1].gain).abs() < 1e-12);
    }

    #[test]
    fn lane_list_only_has_my_lane_and_skips_the_team_list() {
        // Fixture: selva = 234 (ruim, 9%), 64 (favorável), 28 (ruim, raro); topo = 122.
        let b = parse_build_full(&fixture()).unwrap();
        let me = ally(31, true);
        let ids = |v: Vec<BanPick>| v.iter().map(|p| p.champion_id).collect::<Vec<_>>();
        // o topo (122) fica de fora: só oponentes da sua lane
        assert_eq!(
            ids(lane_suggest(&me, &b, &[], &[], 5, Metric::Wr)),
            vec![234, 28]
        );
        // mesma conta da lista do time para o campeão em comum
        let solo = suggest(&[(me.clone(), &b)], &[], 1, Metric::Wr)[0].gain;
        assert!((lane_suggest(&me, &b, &[], &[], 1, Metric::Wr)[0].gain - solo).abs() < 1e-9);
        // sem repetir o que já está na lista do time, nem indisponíveis
        assert_eq!(
            ids(lane_suggest(&me, &b, &[], &[234], 5, Metric::Wr)),
            vec![28]
        );
        assert!(lane_suggest(&me, &b, &[28], &[234], 5, Metric::Wr).is_empty());
    }
}
