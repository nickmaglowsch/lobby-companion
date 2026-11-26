// Métrica das sugestões de ban e pick e dos itens (`metric.rs`): textos, unidade e a conta dos itens.
import type { Combo, ItemStat } from "./api";
export type Metric = "wpa" | "wr";

export const METRICS: [Metric, string, string][] = [
  ["wpa", "WPA", "Probabilidade de vitória adicionada: quanto a escolha muda a chance de vitória do time em relação ao que aconteceria sem ela (o campeão típico da lane, ou a vaga sem o banido), no draft atual. Nos itens: o item contra o item típico daquela posição da build."],
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

// ---- Itens (mesma conta de `metric::list_wpa` e `itemset.rs`) ----

const ITEM_SHRINK = 300;
const logit = (pct: number) => {
  const p = Math.min(0.99, Math.max(0.01, pct / 100));
  return Math.log(p / (1 - p));
};
const prob = (x: number) => 100 / (1 + Math.exp(-x));
const shrunkProb = (base: number, wr: number, n: number) => {
  const b = logit(base);
  return prob(b + (logit(wr) - b) * (n / (n + ITEM_SHRINK)));
};

/**
 * WPA (pp) de cada linha de uma lista de itens: a chance com ela (encolhida para o WR do campeão)
 * menos o contrafactual. `onePerGame` (posição, botas, início, núcleo): o item típico da própria
 * lista pelo pick rate (quem falta para 100% entra com o WR do campeão); senão o WR do campeão.
 */
export function listWpa(rows: { wr: number; pr: number; n: number }[], champWr: number, onePerGame: boolean): number[] {
  const p = rows.map((r) => shrunkProb(champWr, r.wr, r.n));
  let base = champWr;
  if (onePerGame) {
    const listed = rows.reduce((s, r) => s + Math.max(0, r.pr), 0);
    const s = Math.max(listed, 100);
    base = (rows.reduce((acc, r, i) => acc + Math.max(0, r.pr) * p[i], 0) + (s - listed) * champWr) / s;
  }
  return p.map((x) => x - base);
}

export function itemWpa(list: ItemStat[], champWr: number, onePerGame: boolean): Map<number, number> {
  const w = listWpa(list, champWr, onePerGame);
  return new Map(list.map((it, i) => [it.id, w[i]]));
}

/** WPA de cada combinação (inícios, núcleos), na ordem da lista. */
export const comboWpa = (list: Combo[], champWr: number) => listWpa(list, champWr, true);

/** "+0,8" / "−0,4" (uma casa: cabe embaixo do ícone). */
export const pp = (x: number) => {
  const r = Math.round(x * 10) / 10;
  return `${r < 0 ? "−" : "+"}${Math.abs(r).toFixed(1).replace(".", ",")}`;
};

/** Número de um item ou combinação na métrica escolhida: "53,5%" ou "+0,8 pp". */
export const scoreText = (m: Metric, wr: number, wpa: number | undefined) =>
  m === "wpa" && wpa !== undefined ? `${pp(wpa)} pp` : `${wr.toFixed(1).replace(".", ",")}%`;

/** Legenda das cores dos itens. */
export const itemLegend = (m: Metric, minGames: number) =>
  m === "wpa"
    ? `WPA = chance de vitória com o item menos a com o item típico daquela posição (1ª volta e situacionais: menos o WR do campeão); verde/vermelho = ≥0,5 pp. Não desconta quem compra porque já está ganhando. Apagado = menos de ${minGames} partidas.`
    : `Verde/vermelho = ≥1 ponto acima/abaixo do WR do campeão. Apagado = menos de ${minGames} partidas.`;
