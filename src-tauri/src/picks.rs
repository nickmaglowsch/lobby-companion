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

use crate::bans::BanPick;
use crate::champselect::Ally;
use crate::lola::extra::{Counters, Team, TierEntry, TierList};
use serde::Serialize;

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

/// Pontua os candidatos e devolve os `top` melhores.
pub fn suggest(
    my_lane: &str,
    list: &TierList,
    candidates: &[i64],
    enemies: &[Enemy],
    mates: &[Mate],
    mastery: &[(i64, u64)],
    top: usize,
) -> Vec<PickSuggestion> {
    let mut out: Vec<PickSuggestion> = candidates
        .iter()
        .filter_map(|&id| {
            let entry = list.get(id)?;
            let base = base_delta(list, entry);
            let mut parts: Vec<PickPart> = Vec::new();
            for e in enemies {
                if let Some(r) = e.counters.row(id).filter(|r| r.n > 0) {
                    // A linha é do ponto de vista do inimigo: o efeito do candidato é o oposto.
                    parts.push(PickPart {
                        kind: PartKind::Counter,
                        champion_id: e.champion_id,
                        lane: e.lane.clone(),
                        delta: shrunk(-r.d2, r.n),
                        wr: 100.0 - r.wr,
                        n: r.n,
                    });
                }
            }
            for m in mates {
                if let Some(r) = m.team.row(my_lane, id).filter(|r| r.n > 0) {
                    parts.push(PickPart {
                        kind: PartKind::Synergy,
                        champion_id: m.champion_id,
                        lane: m.lane.clone(),
                        delta: shrunk(r.d2, r.n),
                        wr: r.wr,
                        n: r.n,
                    });
                }
            }
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
/// `ganho = chance de ele estar na vaga (pr / soma dos pr) × (WR − média) × n/(n + 300)`,
/// na mesma escala de `bans::suggest` (pontos de WR que você recupera, em média, sem ele).
pub fn tier_bans(list: &TierList, unavailable: &[i64], top: usize) -> Vec<BanPick> {
    let total = list.pr_total();
    if total <= 0.0 {
        return Vec::new();
    }
    let mut out: Vec<BanPick> = list
        .entries
        .values()
        .filter(|e| !unavailable.contains(&e.id))
        .map(|e| BanPick {
            champion_id: e.id,
            gain: e.pr / total * base_delta(list, e),
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
        assert_eq!(suggest("jungle", &l, &[31, 64], &[], &[], &[], 1).len(), 1);
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
        let bans = tier_bans(&l, &[], 5);
        let total = l.pr_total();
        // Lillia: 55% (+4) e 6% das vagas; Cho: +2 e 5%; Viego: +1 e 9%; Sion: +3 mas 0,9%
        let ids: Vec<i64> = bans.iter().map(|b| b.champion_id).collect();
        assert_eq!(ids, vec![876, 31, 234, 14]);
        assert!(close(bans[0].gain, 6.0 / total * 4.0 * 70000.0 / 70300.0));
        assert_eq!(bans[0].tier.as_ref().unwrap().br, 5.0);
        // abaixo da média (Nunu, Darius, Lux) não entra; indisponível também não
        assert!(!ids.contains(&64) && !ids.contains(&99));
        assert_eq!(tier_bans(&l, &[876], 1)[0].champion_id, 31);
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
