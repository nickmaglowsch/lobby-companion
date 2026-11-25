//! Sugestão de bans pelo seu time: quem mais derruba o win rate dos campeões escolhidos
//! (ou declarados) do seu lado, segundo a lista `enemy` do build geral de cada um no lolalytics.
//!
//! Para cada campeão do time e cada inimigo `e` (em qualquer lane do outro lado):
//! `ganho = pr(e)/100 × (WR do campeão − WR dele contra e) × n/(n + 300)`,
//! ou seja, quantos pontos de win rate somem em média por causa de `e`; o fator `n/(n+300)`
//! puxa para zero os confrontos com pouca amostra. O ganho do time é a média ponderada
//! (o seu campeão pesa 2). É tendência de estatística, não leitura do outro time.

use crate::champselect::Ally;
use crate::lola::extra::TierEntry;
use crate::lola::model::Build;
use serde::Serialize;
use std::collections::HashMap;

const SHRINK: f64 = 300.0;
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

pub fn suggest(inputs: &[(Ally, &Build)], unavailable: &[i64], top: usize) -> Vec<BanPick> {
    let weight = |a: &Ally| if a.me { ME_WEIGHT } else { 1.0 };
    let total: f64 = inputs.iter().map(|(a, _)| weight(a)).sum();
    if total <= 0.0 {
        return Vec::new();
    }
    let mut acc: HashMap<i64, (f64, Vec<(f64, Reason)>)> = HashMap::new();
    for (ally, build) in inputs {
        let w = weight(ally);
        for (lane, list) in &build.matchups {
            for m in list {
                if m.n == 0 || unavailable.contains(&m.id) {
                    continue;
                }
                let harm = (build.wr - m.wr) * m.n as f64 / (m.n as f64 + SHRINK);
                let contrib = m.pr / 100.0 * harm;
                let e = acc.entry(m.id).or_default();
                e.0 += w * contrib;
                if contrib > 0.0 {
                    e.1.push((
                        contrib,
                        Reason {
                            ally_champion_id: ally.champion_id,
                            lane: lane.clone(),
                            wr: m.wr,
                            base_wr: build.wr,
                            n: m.n,
                        },
                    ));
                }
            }
        }
    }
    let mut out: Vec<BanPick> = acc
        .into_iter()
        .map(|(id, (score, mut reasons))| {
            reasons.sort_by(|a, b| b.0.total_cmp(&a.0));
            BanPick {
                champion_id: id,
                gain: score / total,
                reasons: reasons.into_iter().take(REASONS).map(|r| r.1).collect(),
                tier: None,
            }
        })
        .filter(|p| p.gain > 0.0)
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
/// `exclude` tira quem já está na lista do time, para as duas listas não se repetirem.
pub fn lane_suggest(
    me: &Ally,
    build: &Build,
    unavailable: &[i64],
    exclude: &[i64],
    top: usize,
) -> Vec<BanPick> {
    let lane = if build.lane.is_empty() {
        me.lane.clone().unwrap_or_default()
    } else {
        build.lane.clone()
    };
    let mut mine = build.clone();
    mine.matchups.retain(|(l, _)| *l == lane);
    let skip: Vec<i64> = unavailable.iter().chain(exclude).copied().collect();
    suggest(&[(me.clone(), &mine)], &skip, top)
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
        let picks = suggest(&[(ally(31, true), &b)], &[], 5);
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
        let without = suggest(&[(ally(31, true), &b)], &[234], 5);
        assert_eq!(without[0].champion_id, 122);
    }

    #[test]
    fn team_average_weights_me_double() {
        let b = parse_build_full(&fixture()).unwrap();
        let mut other = b.clone();
        other.matchups.clear();
        let solo = suggest(&[(ally(31, true), &b)], &[], 1)[0].gain;
        // o meu campeão pesa 2, o aliado sem dados pesa 1: 2/3 do ganho
        let team = suggest(&[(ally(31, true), &b), (ally(99, false), &other)], &[], 1)[0].gain;
        assert!((team - solo * 2.0 / 3.0).abs() < 1e-9);
        assert!(suggest(&[], &[], 5).is_empty());
    }

    #[test]
    fn lane_list_only_has_my_lane_and_skips_the_team_list() {
        // Fixture: selva = 234 (ruim, 9%), 64 (favorável), 28 (ruim, raro); topo = 122.
        let b = parse_build_full(&fixture()).unwrap();
        let me = ally(31, true);
        let ids = |v: Vec<BanPick>| v.iter().map(|p| p.champion_id).collect::<Vec<_>>();
        // o topo (122) fica de fora: só oponentes da sua lane
        assert_eq!(ids(lane_suggest(&me, &b, &[], &[], 5)), vec![234, 28]);
        // mesma conta da lista do time para o campeão em comum
        let solo = suggest(&[(me.clone(), &b)], &[], 1)[0].gain;
        assert!((lane_suggest(&me, &b, &[], &[], 1)[0].gain - solo).abs() < 1e-9);
        // sem repetir o que já está na lista do time, nem indisponíveis
        assert_eq!(ids(lane_suggest(&me, &b, &[], &[234], 5)), vec![28]);
        assert!(lane_suggest(&me, &b, &[28], &[234], 5).is_empty());
    }
}
