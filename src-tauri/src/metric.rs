//! Métrica das sugestões de ban e pick (configuração `metric`).
//!
//! - **`wr`** (a conta original): pontos de win rate acima da média, somados. Ban = `pr × (WR do
//!   seu campeão − WR dele contra o inimigo)`; pick = `(WR − média da lane) + Σ d2`.
//! - **`wpa`** (*win probability added*): quanto a decisão muda a **probabilidade de vitória** do
//!   seu time em relação ao que aconteceria sem ela, no estado atual da seleção:
//!   - as chances viram log-odds (como num modelo de probabilidade de vitória), os efeitos somam
//!     em log-odds e a amostra pequena encolhe o efeito em log-odds;
//!   - a comparação é com o contrafactual: no pick, o campeão típico da lane (média ponderada
//!     pelo pick rate) **no mesmo draft**; no ban, a distribuição de quem o inimigo jogaria nessa
//!     vaga sem o banido (sem quem já está indisponível, renormalizada);
//!   - o confronto com um inimigo visível pesa pela chance de ele estar na lane usada na conta.
//!
//! As duas dão pontos percentuais; o WPA é mais conservador nos extremos e não premia um WR alto
//! que vem de amostra pequena ou de situação que não é a sua.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Metric {
    /// Pontos de win rate (a conta original).
    Wr,
    /// Probabilidade de vitória adicionada.
    #[default]
    Wpa,
}

impl Metric {
    /// `"wr"` ou `"wpa"` (configuração); qualquer outra coisa vira o padrão (WPA).
    pub fn from_setting(s: &str) -> Self {
        match s {
            "wr" => Metric::Wr,
            _ => Metric::Wpa,
        }
    }
}

/// Encolhimento pela amostra (o mesmo nas duas métricas): `n/(n + 300)`.
pub fn shrink_factor(n: u64, shrink: f64) -> f64 {
    n as f64 / (n as f64 + shrink)
}

/// Win rate em % → log-odds (limitado a 1–99% para não explodir).
pub fn logit(pct: f64) -> f64 {
    let p = (pct / 100.0).clamp(0.01, 0.99);
    (p / (1.0 - p)).ln()
}

/// Log-odds → probabilidade em %.
pub fn prob(x: f64) -> f64 {
    100.0 / (1.0 + (-x).exp())
}

/// Chance (em %) de `wr` com o efeito em log-odds encolhido pela amostra, a partir de `base`.
pub fn shrunk_prob(base: f64, wr: f64, n: u64, shrink: f64) -> f64 {
    let b = logit(base);
    prob(b + (logit(wr) - b) * shrink_factor(n, shrink))
}

/// Ganho de banir cada campeão de uma vaga do inimigo, em pontos percentuais da **sua** chance.
/// `rows` = `(id, pr, p)` com `p` = a sua chance (%) se ele estiver na vaga. Sem ele, a vaga
/// fica com os outros na mesma proporção: `ganho = pr·(P − p)/(S − pr)`, com `P = Σ pr·p / Σ pr`
/// (a sua chance hoje) e `S` = o tamanho da vaga: `Σ pr`, ou `slot` se for maior (lista sem
/// todos os campeões: quem falta entra em `P` com a chance `rest`).
pub fn ban_gains(rows: &[(i64, f64, f64)], slot: f64, rest: f64) -> Vec<(i64, f64)> {
    let listed: f64 = rows.iter().map(|r| r.1.max(0.0)).sum();
    if listed <= 0.0 {
        return Vec::new();
    }
    let s = listed.max(slot);
    let now = (rows.iter().map(|r| r.1.max(0.0) * r.2).sum::<f64>() + (s - listed) * rest) / s;
    rows.iter()
        .map(|&(id, pr, p)| {
            let rest = s - pr.max(0.0);
            let gain = if rest > 0.0 {
                pr.max(0.0) * (now - p) / rest
            } else {
                0.0
            };
            (id, gain)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn logit_and_prob_round_trip() {
        assert!(close(logit(50.0), 0.0));
        assert!(close(prob(logit(53.4)), 53.4));
        // 0% e 100% não explodem
        assert!(logit(0.0).is_finite() && logit(100.0).is_finite());
        assert_eq!(Metric::from_setting("wr"), Metric::Wr);
        assert_eq!(Metric::from_setting("wpa"), Metric::Wpa);
        assert_eq!(Metric::from_setting(""), Metric::Wpa);
    }

    #[test]
    fn shrinks_in_log_odds() {
        // sem amostra fica na base; com muita, perto do WR
        assert!(close(shrunk_prob(52.0, 40.0, 0, 300.0), 52.0));
        let big = shrunk_prob(52.0, 40.0, 1_000_000, 300.0);
        assert!((big - 40.0).abs() < 0.01);
    }

    #[test]
    fn ban_gain_renormalizes_the_slot() {
        // vaga: A (50% das partidas, você ganha 40%) e B (50%, você ganha 60%) → hoje 50%.
        // Banir A: a vaga vira só B (60%) → +10. Banir B: só A → −10.
        let g = ban_gains(&[(1, 50.0, 40.0), (2, 50.0, 60.0)], 0.0, 0.0);
        assert!(close(g[0].1, 10.0) && close(g[1].1, -10.0));
        // a conta por WR daria pr × diferença = 0,5 × 10 = 5: o WPA conta a vaga que sobra
        assert!(ban_gains(&[], 100.0, 50.0).is_empty());
        assert!(close(ban_gains(&[(1, 100.0, 40.0)], 0.0, 0.0)[0].1, 0.0));
        // lista incompleta (só 10 de 100): o resto da vaga entra com 50% → hoje 49%;
        // sem ele, a vaga toda fica com 50%: +1
        let g = ban_gains(&[(1, 10.0, 40.0)], 100.0, 50.0);
        assert!(close(g[0].1, 1.0));
    }
}
