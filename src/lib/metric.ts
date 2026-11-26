// Métrica das sugestões de ban e pick e dos itens (`metric.rs`): textos, unidade e a conta dos itens.
// No modo WR os textos são os mesmos de antes (chaves já existentes); os do WPA ficam em `metric.*`.
import type { Combo, ItemStat } from "./api";
import { pct, pts, t } from "./i18n/index.svelte";

export type Metric = "wpa" | "wr";

/** Valor, rótulo do botão e chave da explicação (tooltip). */
export const METRICS: [Metric, string, string][] = [
  ["wpa", "WPA", "metric.wpaHelp"],
  ["wr", "WR", "metric.wrHelp"],
];

/** Pontos com sinal e a unidade da métrica: "+0,21 pp" / "+0,21 pts". */
export const gainText = (m: Metric | undefined, value: string) =>
  m === "wr" ? t("lib.points", { value }) : t("metric.pointsPp", { value });

/** Rodapé dos bans (o do WR é o texto de antes). */
export const banExplain = (m: Metric | undefined, byTier: boolean) =>
  m === "wr"
    ? t(byTier ? "vClassic.ptsTierHint" : "vClassic.ptsTeamHint")
    : t(byTier ? "metric.banTierWpa" : "metric.banTeamWpa");

/** Rodapé curto dos bans (layout Abas). */
export const banShort = (m: Metric | undefined) => t(m === "wr" ? "vTabs.ptsHint" : "metric.banShortWpa");

/** Rodapé das sugestões de pick. */
export const pickExplain = (m: Metric | undefined) => t(m === "wr" ? "pickSuggest.footnote" : "metric.pickWpa");

// ---- Itens (mesma conta de `metric::list_wpa` e `itemset.rs`) ----

const ITEM_SHRINK = 300;
const logit = (p100: number) => {
  const p = Math.min(0.99, Math.max(0.01, p100 / 100));
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

/** "+0,8" / "−0,4" no formato do idioma (uma casa: cabe embaixo do ícone). */
export const pp = (x: number) => pts(Math.round(x * 10) / 10 || 0, 1);

/** Número de um item ou combinação na métrica escolhida: "53,5%" ou "+0,8 pp". */
export const scoreText = (m: Metric, wr: number, wpa: number | undefined) =>
  m === "wpa" && wpa !== undefined ? t("metric.pointsPp", { value: pp(wpa) }) : pct(wr);

/** Legenda das cores dos itens (layout Clássico). */
export const itemLegend = (m: Metric, minGames: number) =>
  m === "wpa" ? t("metric.itemLegendWpa", { min: minGames }) : t("vClassic.itemLegend", { count: minGames });

/** Subtítulo da seção de itens (layout Clássico). */
export const itemsSub = (m: Metric) => t(m === "wpa" ? "metric.itemsSubWpa" : "vClassic.itemsSub");
