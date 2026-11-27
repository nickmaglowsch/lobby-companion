// Idioma da janela. O backend escolhe o locale (configuração "Idioma", senão o do cliente do
// League, senão o do sistema, senão inglês) e manda em `Snapshot.locale`; aqui só carregamos o
// dicionário e formatamos números. Mesmo mapeamento de `src-tauri/src/i18n.rs`.
// Os dicionários vêm de `locales/*.json` sem as chaves repetidas (`compactLocales` em `vite.config.js`).
import { keys, en as enValues, loaders } from "virtual:locales";

type Dict = Record<string, string>;
type Params = Record<string, string | number>;

/** Textos na ordem de `keys` (buracos ficam de fora: caem no inglês) + as chaves só do idioma. */
function dictOf(values: (string | undefined)[], extra: Dict = {}): Dict {
  const d = { ...extra };
  values.forEach((v, i) => (d[keys[i]] = v!));
  return d;
}
const en = dictOf(enValues);

/** Locales do cliente do League (ddragon `languages.json`), com o nome no próprio idioma. */
export const LOCALES: [string, string][] = [
  ["en_US", "English (US)"],
  ["en_GB", "English (UK)"],
  ["en_AU", "English (Australia)"],
  ["en_PH", "English (Philippines)"],
  ["en_SG", "English (Singapore)"],
  ["es_ES", "Español (España)"],
  ["es_MX", "Español (México)"],
  ["es_AR", "Español (Argentina)"],
  ["pt_BR", "Português (Brasil)"],
  ["fr_FR", "Français"],
  ["de_DE", "Deutsch"],
  ["it_IT", "Italiano"],
  ["pl_PL", "Polski"],
  ["ro_RO", "Română"],
  ["el_GR", "Ελληνικά"],
  ["hu_HU", "Magyar"],
  ["cs_CZ", "Čeština"],
  ["ru_RU", "Русский"],
  ["tr_TR", "Türkçe"],
  ["ja_JP", "日本語"],
  ["ko_KR", "한국어"],
  ["zh_CN", "简体中文"],
  ["zh_MY", "简体中文（马来西亚）"],
  ["zh_TW", "繁體中文"],
  ["th_TH", "ไทย"],
  ["vi_VN", "Tiếng Việt"],
  ["id_ID", "Bahasa Indonesia"],
  ["ar_AE", "العربية"],
];

/** Arquivo de tradução de cada locale (variantes iguais dividem o arquivo). */
export function fileFor(locale: string): string {
  if (locale.startsWith("en_")) return "en";
  if (locale === "es_MX" || locale === "es_AR") return "es_MX";
  if (locale === "zh_MY") return "zh_CN";
  if (["es_ES", "pt_BR", "zh_CN", "zh_TW"].includes(locale)) return locale;
  const lang = locale.split("_")[0];
  return LOCALES.some(([l]) => l === locale) ? lang : "en";
}

class I18n {
  locale = $state("en_US");
  /** Tag BCP 47 para `Intl` e `lang` ("pt-BR"). */
  tag = $derived(this.locale.replace("_", "-"));
  rtl = $derived(this.locale.startsWith("ar_"));
  /** Trocado inteiro (nunca mutado): `raw` evita um proxy com um sinal por chave lida. */
  dict = $state.raw<Dict>(en);
  plural = $derived(new Intl.PluralRules(this.tag));
  private number = $derived(new Intl.NumberFormat(this.tag));
  private percent = $derived(
    new Intl.NumberFormat(this.tag, { style: "percent", minimumFractionDigits: 1, maximumFractionDigits: 1 }),
  );
  private fixed = new Map<string, Intl.NumberFormat>();

  fixedFormat(digits: number): Intl.NumberFormat {
    const key = `${this.tag}:${digits}`;
    let f = this.fixed.get(key);
    if (!f) {
      f = new Intl.NumberFormat(this.tag, { minimumFractionDigits: digits, maximumFractionDigits: digits });
      this.fixed.set(key, f);
    }
    return f;
  }
  formatNumber = (x: number) => this.number.format(x);
  formatPercent = (x: number) => this.percent.format(x / 100);
}

export const i18n = new I18n();

/** Último idioma pedido: trocas rápidas (configuração e snapshot) não chegam fora de ordem. */
let wanted: string | null = null;
let pending: Promise<void> = Promise.resolve();

/** Troca o idioma (carrega o arquivo antes, para a tela não piscar com chaves). */
export function setLocale(locale: string): Promise<void> {
  if (locale === wanted) return pending;
  wanted = locale;
  return (pending = apply(locale));
}

async function apply(locale: string) {
  const load = loaders[fileFor(locale)];
  const dict = load ? await load().then((m) => dictOf(...m.default), () => en) : en;
  if (locale !== wanted) return;
  i18n.dict = dict;
  i18n.locale = LOCALES.some(([l]) => l === locale) ? locale : "en_US";
  if (typeof document !== "undefined") {
    document.documentElement.lang = i18n.tag;
    document.documentElement.dir = i18n.rtl ? "rtl" : "ltr";
  }
}

function lookup(key: string): string | undefined {
  return i18n.dict[key] ?? en[key];
}

/**
 * Texto traduzido. `{nome}` no texto vira `params.nome`. Com `params.count` numérico, usa a forma
 * de plural do idioma (`chave_one`, `chave_few`, `chave_other`…) quando existe.
 */
export function t(key: string, params?: Params): string {
  let s: string | undefined;
  if (params && typeof params.count === "number") {
    const cat = i18n.plural.select(params.count);
    s = lookup(`${key}_${cat}`) ?? lookup(`${key}_other`);
  }
  s ??= lookup(key) ?? key;
  if (!params) return s;
  return s.replace(/\{(\w+)\}/g, (m, name: string) => {
    const v = params[name];
    if (v === undefined) return m;
    return typeof v === "number" ? i18n.formatNumber(v) : v;
  });
}

/** "51,8%" / "51.8%" / "%51,8" (o número já vem de 0 a 100). */
export const pct = (x: number) => i18n.formatPercent(x);
/** Pontos com sinal: "+1,20" / "−0,35". Em RTL, isolado como LTR (senão o sinal vai para o fim). */
export const pts = (x: number, digits = 2) => {
  const s = `${x >= 0 ? "+" : "−"}${i18n.fixedFormat(digits).format(Math.abs(x))}`;
  return i18n.rtl ? `\u2066${s}\u2069` : s;
};
/** Inteiro com separador de milhar do idioma. */
export const num = (x: number) => i18n.formatNumber(x);
/** Número com casas fixas ("1,5"). */
export const fixed = (x: number, digits = 1) => i18n.fixedFormat(digits).format(x);

/** Nome da lane do lolalytics ("jungle" → "Selva"); desconhecida, o próprio texto. */
export const laneName = (lane: string | null | undefined) => (lane ? (lookup(`lane.${lane}`) ?? lane) : "");
/** Fase do cliente (`gameflow-phase`). */
export const phaseName = (phase: string) => lookup(`phase.${phase}`) ?? phase;
/** Comparação de nomes na ordem do idioma. */
export const compare = (a: string, b: string) => a.localeCompare(b, i18n.tag);
export const lower = (s: string) => s.toLocaleLowerCase(i18n.tag);
