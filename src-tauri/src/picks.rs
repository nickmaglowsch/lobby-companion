//! Sugestão de pick para a sua posição (counter + sinergia + força na lane), bans pela tier list
//! quando o time ainda não declarou nada e a sinergia do seu campeão com os aliados.
//!
//! Tudo em pontos de win rate, na mesma escala de `bans.rs`, e puxado para zero com pouca
//! amostra (`× n/(n + 300)`):
//! - **base**: WR do candidato na sua lane − média da lane (`avgWr` da tier list);
//! - **counter**: para cada inimigo visível, `−d2` da linha do candidato no `ep=counter` do
//!   inimigo (na lane mais jogada dele, contra a sua lane): o quanto o candidato ganha ou perde
//!   nesse confronto além da força geral dos dois;
//! - **sinergia**: para cada aliado com lane, `d2` da linha do candidato no `ep=build-team` do
//!   aliado (lane do aliado, linha da sua lane): a dupla além da força geral dos dois.
//!
//! Como o `d2` já desconta a força geral de cada campeão, as partes somam sem contar a força do
//! candidato duas vezes. É tendência de estatística, não leitura do outro time.
//!
//! Com a métrica WPA (`metric.rs`) as mesmas partes viram log-odds e somam lá; o total é a chance
//! de vitória com o candidato **menos** a chance com o campeão típico da lane (média ponderada
//! pelo pick rate da tier list) no mesmo draft. O counter pesa pela chance de o inimigo estar
//! na lane usada (`lanes` do `ep=counter`). Cada parte mostra quanto a chance cai sem ela.

use crate::bans::BanPick;
use crate::champselect::Ally;
use crate::lola::extra::{Counters, PairStat, Team, TierEntry, TierList};
use crate::metric::{self, Metric};
use serde::Serialize;
use std::collections::HashMap;

/// Mesmo encolhimento de `bans.rs`.
pub const SHRINK: f64 = 300.0;
/// Candidatos considerados (do seu pool ou da tier list).
pub const POOL_SIZE: usize = 15;
/// Sugestões mostradas.
pub const TOP: usize = 5;
/// Do seu pool, só quem joga essa lane em pelo menos 5% das partidas dele.
pub const MIN_PCT_LANE: f64 = 5.0;
/// Na tier list, só quem aparece na lane (pick rate ≥ 1%).
pub const MIN_PR_TIER: f64 = 1.0;

fn shrunk(x: f64, n: u64) -> f64 {
    x * n as f64 / (n as f64 + SHRINK)
}

/// Força do campeão na lane: WR − média, com o encolhimento pela amostra.
pub fn base_delta(list: &TierList, e: &TierEntry) -> f64 {
    shrunk(e.wr - list.avg_wr, e.games)
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PartKind {
    /// Contra um inimigo visível.
    Counter,
    /// Com um aliado escolhido ou declarado.
    Synergy,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PickPart {
    pub kind: PartKind,
    /// Inimigo ou aliado.
    pub champion_id: i64,
    /// Lane dele usada na conta.
    pub lane: String,
    /// Pontos que entram no total (já encolhidos pela amostra).
    pub delta: f64,
    /// WR do candidato nesse confronto / dessa dupla.
    pub wr: f64,
    pub n: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PickSuggestion {
    pub champion_id: i64,
    pub total: f64,
    /// Parte da força geral na lane.
    pub base: f64,
    /// WR do candidato na lane e partidas (tier list).
    pub wr: f64,
    pub games: u64,
    /// Pontos de maestria (quando o candidato veio do seu pool).
    pub mastery: Option<u64>,
    /// Counters e sinergias, do maior efeito (em módulo) para o menor.
    pub parts: Vec<PickPart>,
}

/// Um inimigo visível, a lane que usamos para ele e o `ep=counter` dele contra a sua lane.
pub struct Enemy<'a> {
    pub champion_id: i64,
    pub lane: String,
    pub counters: &'a Counters,
}

/// Um aliado com lane e o `ep=build-team` dele.
pub struct Mate<'a> {
    pub champion_id: i64,
    pub lane: String,
    pub team: &'a Team,
}

/// A lane de um inimigo (o cliente não mostra): a mais jogada dele, pela tier list da sua lane.
/// Sem dados, supõe a sua lane.
pub fn enemy_lane(list: &TierList, id: i64, my_lane: &str) -> String {
    list.get(id)
        .map(|e| e.default_lane.clone())
        .filter(|l| !l.is_empty())
        .unwrap_or_else(|| my_lane.to_string())
}

/// Até `max` campeões do seu pool (mais pontos de maestria primeiro) que jogam a lane.
pub fn pool_from_mastery(
    mastery: &[(i64, u64)],
    list: &TierList,
    exclude: &[i64],
    max: usize,
) -> Vec<i64> {
    mastery
        .iter()
        .filter(|(id, _)| !exclude.contains(id))
        .filter(|(id, _)| list.get(*id).is_some_and(|e| e.pct_lane >= MIN_PCT_LANE))
        .map(|(id, _)| *id)
        .take(max)
        .collect()
}

/// Até `max` campeões da tier list da lane: populares (pick rate ≥ 1%) e mais fortes primeiro.
pub fn pool_from_tier(list: &TierList, exclude: &[i64], max: usize) -> Vec<i64> {
    let mut c: Vec<&TierEntry> = list
        .entries
        .values()
        .filter(|e| e.pr >= MIN_PR_TIER && e.pct_lane >= MIN_PCT_LANE && !exclude.contains(&e.id))
        .collect();
    c.sort_by(|a, b| {
        base_delta(list, b)
            .total_cmp(&base_delta(list, a))
            .then(a.id.cmp(&b.id))
    });
    c.into_iter().take(max).map(|e| e.id).collect()
}

/// Efeito de um par (confronto ou dupla) em log-odds: `wr` contra o esperado `wr − d2`,
/// encolhido pela amostra.
fn pair_logit(r: &PairStat) -> f64 {
    (metric::logit(r.wr) - metric::logit(r.wr - r.d2)) * metric::shrink_factor(r.n, SHRINK)
}

/// Chance (0–1) de o inimigo estar na lane usada na conta (`lanes` do `ep=counter`, em %).
/// Sem a distribuição, supõe que está.
fn lane_share(c: &Counters, lane: &str) -> f64 {
    c.lanes
        .iter()
        .find(|(l, _)| l == lane)
        .map(|(_, v)| (v / 100.0).clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

/// Counters e sinergias do candidato `id`. O `delta` sai pronto na métrica WR; no WPA fica 0 e
/// o segundo valor é o efeito em log-odds (o counter já pesado pela chance da lane do inimigo).
fn raw_parts(
    my_lane: &str,
    id: i64,
    enemies: &[Enemy],
    mates: &[Mate],
    metric: Metric,
) -> Vec<(PickPart, f64)> {
    let wr_mode = metric == Metric::Wr;
    let mut parts = Vec::new();
    for e in enemies {
        if let Some(r) = e.counters.row(id).filter(|r| r.n > 0) {
            // A linha é do ponto de vista do inimigo: o efeito do candidato é o oposto.
            let share = if wr_mode {
                1.0
            } else {
                lane_share(e.counters, &e.lane)
            };
            parts.push((
                PickPart {
                    kind: PartKind::Counter,
                    champion_id: e.champion_id,
                    lane: e.lane.clone(),
                    delta: if wr_mode { shrunk(-r.d2, r.n) } else { 0.0 },
                    wr: 100.0 - r.wr,
                    n: r.n,
                },
                -pair_logit(r) * share,
            ));
        }
    }
    for m in mates {
        if let Some(r) = m.team.row(my_lane, id).filter(|r| r.n > 0) {
            parts.push((
                PickPart {
                    kind: PartKind::Synergy,
                    champion_id: m.champion_id,
                    lane: m.lane.clone(),
                    delta: if wr_mode { shrunk(r.d2, r.n) } else { 0.0 },
                    wr: r.wr,
                    n: r.n,
                },
                pair_logit(r),
            ));
        }
    }
    parts
}

/// Log-odds da chance de vitória com `e` na sua lane, nesse draft (WPA).
fn wpa_logit(list: &TierList, e: &TierEntry, parts: &[(PickPart, f64)]) -> f64 {
    let base = metric::logit(list.avg_wr);
    base + (metric::logit(e.wr) - base) * metric::shrink_factor(e.games, SHRINK)
        + parts.iter().map(|p| p.1).sum::<f64>()
}

/// Chance de vitória (%) com o campeão típico da lane nesse draft: média das chances de cada
/// campeão da tier list, ponderada pelo pick rate (o contrafactual do WPA).
fn wpa_baseline(my_lane: &str, list: &TierList, enemies: &[Enemy], mates: &[Mate]) -> f64 {
    let (mut sum, mut w) = (0.0, 0.0);
    for e in list.entries.values().filter(|e| e.pr > 0.0) {
        let parts = raw_parts(my_lane, e.id, enemies, mates, Metric::Wpa);
        sum += e.pr * metric::prob(wpa_logit(list, e, &parts));
        w += e.pr;
    }
    if w > 0.0 { sum / w } else { list.avg_wr }
}

/// Pontua os candidatos e devolve os `top` melhores.
#[allow(clippy::too_many_arguments)]
pub fn suggest(
    my_lane: &str,
    list: &TierList,
    candidates: &[i64],
    enemies: &[Enemy],
    mates: &[Mate],
    mastery: &[(i64, u64)],
    top: usize,
    metric: Metric,
) -> Vec<PickSuggestion> {
    let baseline = match metric {
        Metric::Wpa => wpa_baseline(my_lane, list, enemies, mates),
        Metric::Wr => 0.0,
    };
    let mut out: Vec<PickSuggestion> = candidates
        .iter()
        .filter_map(|&id| {
            let entry = list.get(id)?;
            let raw = raw_parts(my_lane, id, enemies, mates, metric);
            let (base, mut parts): (f64, Vec<PickPart>) = match metric {
                Metric::Wr => (
                    base_delta(list, entry),
                    raw.into_iter().map(|p| p.0).collect(),
                ),
                Metric::Wpa => {
                    let full = wpa_logit(list, entry, &raw);
                    let p = metric::prob(full);
                    let parts: Vec<PickPart> = raw
                        .into_iter()
                        .map(|(part, x)| PickPart {
                            // quanto a chance cai sem essa parte
                            delta: p - metric::prob(full - x),
                            ..part
                        })
                        .collect();
                    // a força na lane fica com o resto: total = chance − contrafactual
                    let sum: f64 = parts.iter().map(|x| x.delta).sum();
                    (p - baseline - sum, parts)
                }
            };
            parts.sort_by(|a, b| b.delta.abs().total_cmp(&a.delta.abs()));
            let total = base + parts.iter().map(|p| p.delta).sum::<f64>();
            Some(PickSuggestion {
                champion_id: id,
                total,
                base,
                wr: entry.wr,
                games: entry.games,
                mastery: mastery.iter().find(|(m, _)| *m == id).map(|(_, p)| *p),
                parts,
            })
        })
        .collect();
    out.sort_by(|a, b| {
        b.total
            .total_cmp(&a.total)
            .then(a.champion_id.cmp(&b.champion_id))
    });
    out.truncate(top);
    out
}

/// Bans sem campeões declarados no seu time: pela força geral na sua lane.
/// - WR: `ganho = chance de ele estar na vaga (pr / soma dos pr) × (WR − média) × n/(n + 300)`,
///   na mesma escala de `bans::suggest` (pontos de WR que você recupera, em média, sem ele).
/// - WPA: a vaga do oponente é a tier list (sem os indisponíveis); o ganho é quanto a chance do
///   oponente cai quando ela fica com os outros (`metric::ban_gains` do lado dele, invertido).
pub fn tier_bans(list: &TierList, unavailable: &[i64], top: usize, metric: Metric) -> Vec<BanPick> {
    let total = list.pr_total();
    if total <= 0.0 {
        return Vec::new();
    }
    let open: Vec<&TierEntry> = list
        .entries
        .values()
        .filter(|e| !unavailable.contains(&e.id))
        .collect();
    let gains: HashMap<i64, f64> = match metric {
        Metric::Wr => open
            .iter()
            .map(|e| (e.id, e.pr / total * base_delta(list, e)))
            .collect(),
        Metric::Wpa => {
            let slot: Vec<(i64, f64, f64)> = open
                .iter()
                .map(|e| {
                    let p = metric::shrunk_prob(list.avg_wr, e.wr, e.games, SHRINK);
                    (e.id, e.pr, p)
                })
                .collect();
            // A vaga é a lane inteira (os pick rates somam ~200: dois por lane), menos os
            // indisponíveis; quem faltar na lista entra com a média da lane.
            let gone: f64 = list
                .entries
                .values()
                .filter(|e| unavailable.contains(&e.id))
                .map(|e| e.pr)
                .sum();
            metric::ban_gains(&slot, total.max(200.0) - gone, list.avg_wr)
                .into_iter()
                .map(|(id, g)| (id, -g))
                .collect()
        }
    };
    let mut out: Vec<BanPick> = open
        .into_iter()
        .map(|e| BanPick {
            champion_id: e.id,
            gain: gains.get(&e.id).copied().unwrap_or(0.0),
            reasons: Vec::new(),
            tier: Some(e.clone()),
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

/// "você + Lux (meio): 55,0% · geral 53,4% · 300 partidas".
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SynergyLine {
    pub ally_champion_id: i64,
    pub lane: String,
    /// WR do seu campeão com esse aliado nessa lane.
    pub wr: f64,
    /// Efeito da dupla além da força geral dos dois (`d2`).
    pub delta: f64,
    pub n: u64,
}

/// Sinergia do seu campeão (pelo `ep=build-team` dele) com cada aliado que tem lane.
pub fn synergy_lines(team: &Team, allies: &[Ally]) -> Vec<SynergyLine> {
    allies
        .iter()
        .filter(|a| !a.me)
        .filter_map(|a| {
            let lane = a.lane.as_deref()?;
            let r = team.row(lane, a.champion_id)?;
            Some(SynergyLine {
                ally_champion_id: a.champion_id,
                lane: lane.to_string(),
                wr: r.wr,
                delta: r.d2,
                n: r.n,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lola::extra::tests::{counter_fixture, list_fixture, team_fixture};
    use crate::lola::extra::{parse_build_team, parse_counter, parse_list};

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn pools_respect_lane_and_exclusions() {
        let l = parse_list(&list_fixture()).unwrap();
        // maestria: Lux (99) quase não joga selva; Cho (31) indisponível
        let mastery = vec![
            (99, 90000),
            (31, 80000),
            (14, 50000),
            (64, 10000),
            (5, 9000),
        ];
        assert_eq!(pool_from_mastery(&mastery, &l, &[31], 15), vec![14, 64]);
        assert_eq!(pool_from_mastery(&mastery, &l, &[], 2), vec![31, 14]);
        // tier list: pick rate ≥ 1% e mais fortes primeiro (Sion fica de fora pelo pick rate)
        assert_eq!(pool_from_tier(&l, &[876], 15), vec![31, 234, 122, 64]);
    }

    #[test]
    fn base_pulls_small_samples_to_zero() {
        let l = parse_list(&list_fixture()).unwrap();
        let lux = l.get(99).unwrap();
        // 40% em 30 partidas: −11 × 30/330
        assert!(close(base_delta(&l, lux), -11.0 * 30.0 / 330.0));
        let lillia = l.get(876).unwrap();
        assert!(close(base_delta(&l, lillia), 4.0 * 70000.0 / 70300.0));
    }

    #[test]
    fn scores_counter_and_synergy_with_breakdown() {
        let l = parse_list(&list_fixture()).unwrap();
        let viego = parse_counter(&counter_fixture()).unwrap();
        let lux = parse_build_team(&team_fixture()).unwrap();
        let enemies = [Enemy {
            champion_id: 234,
            lane: "jungle".into(),
            counters: &viego,
        }];
        let mates = [Mate {
            champion_id: 99,
            lane: "middle".into(),
            team: &lux,
        }];
        let picks = suggest(
            "jungle",
            &l,
            &[31, 64, 14, 876, 4242],
            &enemies,
            &mates,
            &[(31, 1000)],
            5,
            Metric::Wr,
        );
        // 4242 não está na tier list: fica de fora
        assert_eq!(picks.len(), 4);
        let cho = picks.iter().find(|p| p.champion_id == 31).unwrap();
        let base = 2.0 * 60000.0 / 60300.0;
        // Viego tem d2 −5 contra o Cho (300 partidas): +5 × 300/600 para o Cho
        let counter = 5.0 * 300.0 / 600.0;
        // Lux + Cho na selva: d2 1,5 em 600 partidas
        let synergy = 1.5 * 600.0 / 900.0;
        assert!(close(cho.base, base));
        assert!(close(cho.total, base + counter + synergy));
        assert_eq!(cho.mastery, Some(1000));
        assert_eq!(cho.parts.len(), 2);
        assert_eq!(cho.parts[0].kind, PartKind::Counter);
        assert_eq!(cho.parts[0].champion_id, 234);
        // WR do Cho contra o Viego = 100 − 45
        assert!(close(cho.parts[0].wr, 55.0));
        assert_eq!(cho.parts[1].kind, PartKind::Synergy);
        assert!(close(cho.parts[1].wr, 55.0));
        // Nunu (64): Viego ganha dele (d2 +3) → negativo
        let nunu = picks.iter().find(|p| p.champion_id == 64).unwrap();
        assert!(close(nunu.parts[0].delta, -3.0 * 900.0 / 1200.0));
        // Sion: amostra minúscula (20 e 30 partidas) pesa pouco
        let sion = picks.iter().find(|p| p.champion_id == 14).unwrap();
        assert!(sion.parts.iter().all(|p| p.delta.abs() < 0.5));
        // ordem pelo total: a Lillia é a mais forte na lane, mas a dupla com a Lux (d2 −4 em
        // 900 partidas) a derruba para baixo do Sion
        let order: Vec<i64> = picks.iter().map(|p| p.champion_id).collect();
        assert_eq!(order, vec![31, 14, 876, 64]);
        // top corta
        assert_eq!(
            suggest("jungle", &l, &[31, 64], &[], &[], &[], 1, Metric::Wr).len(),
            1
        );
    }

    #[test]
    fn wpa_compares_with_the_typical_pick_in_the_same_draft() {
        let l = parse_list(&list_fixture()).unwrap();
        let viego = parse_counter(&counter_fixture()).unwrap();
        let lux = parse_build_team(&team_fixture()).unwrap();
        let enemies = [Enemy {
            champion_id: 234,
            lane: "jungle".into(),
            counters: &viego,
        }];
        let mates = [Mate {
            champion_id: 99,
            lane: "middle".into(),
            team: &lux,
        }];
        let cands = [31, 64, 14, 876];
        let run = |enemies: &[Enemy], m: Metric| {
            suggest("jungle", &l, &cands, enemies, &mates, &[], 5, m)
        };
        let wpa = run(&enemies, Metric::Wpa);
        let wr = run(&enemies, Metric::Wr);
        let cho = wpa.iter().find(|p| p.champion_id == 31).unwrap();
        let cho_wr = wr.iter().find(|p| p.champion_id == 31).unwrap();
        // total = chance com o Cho − chance com o típico da lane, no mesmo draft
        let raw = raw_parts("jungle", 31, &enemies, &mates, Metric::Wpa);
        let chance = metric::prob(wpa_logit(&l, l.get(31).unwrap(), &raw));
        let typical = wpa_baseline("jungle", &l, &enemies, &mates);
        assert!(close(cho.total, chance - typical));
        assert!(close(
            cho.total,
            cho.base + cho.parts.iter().map(|p| p.delta).sum::<f64>()
        ));
        // counter a favor do Cho, um pouco menor que na conta por WR (Viego joga 96% na selva)
        let counter = |s: &PickSuggestion| {
            s.parts
                .iter()
                .find(|p| p.kind == PartKind::Counter)
                .unwrap()
                .delta
        };
        assert!(counter(cho) > 0.0 && counter(cho) < counter(cho_wr));
        // um Viego que só joga metade das partidas na selva pesa metade
        let mut half = viego.clone();
        half.lanes = vec![("jungle".into(), 50.0), ("top".into(), 50.0)];
        let half_enemy = [Enemy {
            champion_id: 234,
            lane: "jungle".into(),
            counters: &half,
        }];
        let ratio = counter(
            run(&half_enemy, Metric::Wpa)
                .iter()
                .find(|p| p.champion_id == 31)
                .unwrap(),
        ) / counter(cho);
        assert!((ratio - 50.0 / 96.0).abs() < 0.01, "{ratio}");
        // a Lillia perde com a Lux (d2 −4 em 900 partidas)
        let lillia = wpa.iter().find(|p| p.champion_id == 876).unwrap();
        assert!(
            lillia
                .parts
                .iter()
                .any(|p| p.kind == PartKind::Synergy && p.delta < 0.0)
        );
        assert_eq!(wpa[0].champion_id, 31);
    }

    #[test]
    fn wpa_tier_bans_renormalize_the_lane() {
        let l = parse_list(&list_fixture()).unwrap();
        let wpa = tier_bans(&l, &[], 5, Metric::Wpa);
        let wr = tier_bans(&l, &[], 5, Metric::Wr);
        let ids = |v: &[BanPick]| v.iter().map(|b| b.champion_id).collect::<Vec<_>>();
        assert_eq!(ids(&wpa), ids(&wr));
        // tirar alguém da vaga muda a média de quem sobra
        assert_eq!(tier_bans(&l, &[876], 1, Metric::Wpa)[0].champion_id, 31);
        assert!(wpa.iter().all(|b| b.gain > 0.0));
    }

    #[test]
    fn enemy_lane_from_tier_list() {
        let l = parse_list(&list_fixture()).unwrap();
        assert_eq!(enemy_lane(&l, 14, "jungle"), "top");
        assert_eq!(enemy_lane(&l, 234, "jungle"), "jungle");
        assert_eq!(enemy_lane(&l, 4242, "jungle"), "jungle");
    }

    #[test]
    fn tier_bans_weight_win_rate_by_pick_rate() {
        let l = parse_list(&list_fixture()).unwrap();
        let bans = tier_bans(&l, &[], 5, Metric::Wr);
        let total = l.pr_total();
        // Lillia: 55% (+4) e 6% das vagas; Cho: +2 e 5%; Viego: +1 e 9%; Sion: +3 mas 0,9%
        let ids: Vec<i64> = bans.iter().map(|b| b.champion_id).collect();
        assert_eq!(ids, vec![876, 31, 234, 14]);
        assert!(close(bans[0].gain, 6.0 / total * 4.0 * 70000.0 / 70300.0));
        assert_eq!(bans[0].tier.as_ref().unwrap().br, 5.0);
        // abaixo da média (Nunu, Darius, Lux) não entra; indisponível também não
        assert!(!ids.contains(&64) && !ids.contains(&99));
        assert_eq!(tier_bans(&l, &[876], 1, Metric::Wr)[0].champion_id, 31);
    }

    #[test]
    fn synergy_of_my_champion_with_allies() {
        let lux_team = parse_build_team(&team_fixture()).unwrap();
        let allies = vec![
            Ally {
                champion_id: 31,
                lane: Some("jungle".into()),
                me: true,
            },
            Ally {
                champion_id: 31,
                lane: Some("jungle".into()),
                me: false,
            },
            Ally {
                champion_id: 122,
                lane: Some("top".into()),
                me: false,
            },
            Ally {
                champion_id: 7,
                lane: None,
                me: false,
            },
        ];
        let lines = synergy_lines(&lux_team, &allies);
        // o próprio jogador e quem não tem lane ficam de fora
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].ally_champion_id, 122);
        assert_eq!((lines[1].wr, lines[1].n), (52.0, 500));
        assert!(close(lines[0].delta, 1.5));
    }
}
