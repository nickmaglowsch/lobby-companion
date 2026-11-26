// Métrica das sugestões de ban e pick (`metric.rs`): textos e unidade de cada uma.
export type Metric = "wpa" | "wr";

export const METRICS: [Metric, string, string][] = [
  ["wpa", "WPA", "Probabilidade de vitória adicionada: quanto a escolha muda a chance de vitória do time em relação ao que aconteceria sem ela (o campeão típico da lane, ou a vaga sem o banido), no draft atual."],
  ["wr", "WR", "Pontos de win rate acima da média (a conta original): força na lane e cada confronto/dupla além da força geral, × a frequência nos bans."],
];

/** Unidade curta dos pontos: "pp" = pontos percentuais de chance de vitória; "pts" = de win rate. */
export const unit = (m: Metric | undefined) => (m === "wr" ? "pts" : "pp");

/** Rodapé dos bans. */
export function banExplain(m: Metric | undefined, byTier: boolean): string {
  if (m === "wr") {
    return byTier
      ? "Pts = win rate acima da média da lane × chance de ele cair na sua lane (pick rate); amostra pequena conta menos."
      : "Pts = win rate recuperado, em média, sem esse campeão no jogo: frequência dele × queda do WR (amostra pequena conta menos).";
  }
  return byTier
    ? "pp (WPA) = quanto a chance de vitória sobe se a vaga do oponente na sua lane ficar com os outros campeões (pelo pick rate de cada um); amostra pequena conta menos."
    : "pp (WPA) = quanto a chance de vitória do time sobe sem ele: a vaga dele fica com os outros campeões daquela lane, na proporção do pick rate (amostra pequena conta menos).";
}

/** Rodapé das sugestões de pick. */
export function pickExplain(m: Metric | undefined): string {
  return m === "wr"
    ? "Pts = win rate a mais (ou a menos) que a média: força na lane + cada confronto e dupla além da força geral dos dois (amostra pequena conta menos)."
    : "pp (WPA) = chance de vitória com ele menos a chance com o campeão típico da lane, neste draft; cada confronto e dupla mostra quanto a chance cai sem ele (amostra pequena conta menos).";
}
