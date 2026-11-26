// Tipos espelhando o serde do backend (src-tauri/src/app.rs, lola/model.rs, assets.rs, settings.rs).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { cdragonUrl } from "./mockIcons";
import { itemWpa, type Metric } from "./metric";
import { t } from "./i18n/index.svelte";

export interface ItemStat { id: number; wr: number; pr: number; n: number }
export interface Combo { ids: string[]; wr: number; pr: number; n: number }
export interface RuneSet { primary: number[]; secondary: number[]; shards: number[]; wr: number; n: number }
/** Página de runas própria (`own_runes.rs`): várias por campeão, cada uma com nome.
 *  `active` = a escolhida do campeão (vai para o cliente na importação). */
export interface OwnPage {
  championId: number; id: number; name: string; primaryStyle: number; subStyle: number;
  /** 9 runas na ordem do cliente: keystone + 3, 2 secundárias, 3 fragmentos. */
  perks: number[];
  active: boolean;
  /** Segundos desde 1970. */
  savedAt: number;
}
/** Aba de runas: "pick" (mais jogada), "win" (maior WR) ou "own:<id>" (uma página sua). */
export type RuneChoice = "pick" | "win" | `own:${number}`;

export interface Build {
  slug: string;
  lane: string;
  patch: string;
  wr: number;
  pr: number;
  br: number;
  n: number;
  rank: number | null;
  rankTotal: number | null;
  runesPick: RuneSet | null;
  runesWin: RuneSet | null;
  spells: Combo[];
  skillPriority: Combo[];
  skillOrder: string | null;
  startSets: Combo[];
  boots: ItemStat[];
  slots: ItemStat[][];
  lanes: [string, number][];
  /** Preenchido quando é o build contra um campeão específico. */
  vs: VsInfo | null;
  /** Núcleo: 3 primeiros itens (com botas) na ordem de compra, por partidas. */
  core: Combo[];
  /** Itens de maior WR (do maior para o menor). */
  winningItems: ItemStat[];
  /** Itens comprados cedo (primeira volta), por partidas. */
  earlyItems: ItemStat[];
  /** Habilidades até o 6 e até o 10 (letras em `ids`), por partidas. */
  skill6: Combo[];
  skill10: Combo[];
  runeStats: RuneStats | null;
  trend: Trend | null;
  damage: Damage | null;
}

/** `pr` = % das partidas com a runa; `wr` = win rate com ela. */
export interface PerkStat { pr: number; wr: number; n: number }
/** Por lugar na página: árvore primária (e fragmentos de ataque/defesa), secundária e fragmentos do meio. */
export interface RuneStats {
  primary: Record<string, PerkStat>;
  secondary: Record<string, PerkStat>;
  flex: Record<string, PerkStat>;
}
/** WR por dia dos últimos ~30 dias (todos os elos). */
export interface Trend { dates: string[]; wr: number[]; wrs: number[]; n: number[]; wrLast7: number; wrPeriod: number }
/** Dano médio por partida. */
export interface Damage { physical: number; magic: number; true: number }

export interface VsInfo { championId: number; lane: string; usualWr: number }

/** Mesma regra de `Build::min_games` (Rust): no confronto o corte acompanha a amostra. */
export function minGamesFor(build: Build, configured: number): number {
  return build.vs ? Math.min(configured, Math.max(10, Math.floor(build.n / 20))) : configured;
}

export interface Ally { championId: number; lane: string | null; me: boolean }
export interface BanReason { allyChampionId: number; lane: string; wr: number; baseWr: number; n: number }
/** Um campeão na tier list de uma lane (lolalytics `ep=list`). */
export interface TierEntry { id: number; wr: number; pr: number; br: number; games: number; pctLane: number; defaultLane: string }
/** `tier` só nos bans pela tier list (ninguém do time declarou ainda). */
export interface BanPick { championId: number; gain: number; reasons: BanReason[]; tier: TierEntry | null }
export interface BanAdvice {
  loading: boolean; basedOn: Ally[]; picks: BanPick[]; note: string | null;
  /** Bans pela força geral na sua lane (tier list), sem campeões declarados no time. */
  byTier: boolean; lane: string | null; pageUrl: string | null;
  /** Contra o seu pick: oponentes da sua lane com confronto ruim e pick provável (sem repetir `picks`). */
  lanePicks: BanPick[]; laneFor: Ally | null; laneNote: string | null;
  /** Métrica dos ganhos (`metric.rs`): "wpa" = pontos percentuais de chance; "wr" = pontos de WR. */
  metric: Metric;
}

/** Parte da nota de um pick: contra um inimigo ou com um aliado (pontos já encolhidos pela amostra). */
export interface PickPart { kind: "counter" | "synergy"; championId: number; lane: string; delta: number; wr: number; n: number }
export interface PickSuggestion {
  championId: number; total: number; base: number; wr: number; games: number;
  mastery: number | null; parts: PickPart[];
}
export interface PickAdvice {
  loading: boolean; lane: string | null;
  /** "mastery" = seu pool; "tier" = tier list da lane. */
  pool: string;
  suggestions: PickSuggestion[]; enemies: number[]; allies: Ally[]; note: string | null; pageUrl: string | null;
  /** Métrica de `total`/`base`/`delta`. */
  metric: Metric;
}
export interface SynergyLine { allyChampionId: number; lane: string; wr: number; delta: number; n: number }
export interface SynergyAdvice { championId: number; lane: string; baseWr: number | null; lines: SynergyLine[]; pageUrl: string | null }

export interface Outcome { ok: boolean; msg: string }
export interface ImportReport { runes: Outcome | null; items: Outcome | null; spells: Outcome | null }

export interface Snapshot {
  connected: boolean;
  phase: string;
  inChampSelect: boolean;
  /** Na seleção, ainda há ban por fazer. */
  banning: boolean;
  locked: boolean;
  /** É a sua vez de banir: clicar numa sugestão de ban bane no cliente. */
  canBan: boolean;
  championId: number;
  lane: string | null;
  loading: boolean;
  build: Build | null;
  pageUrl: string | null;
  warning: string | null;
  error: string | null;
  import: ImportReport | null;
  vsChampionId: number;
  vsLane: string | null;
  enemyPicks: number[];
  /** Campeões escolhidos ou declarados do seu time (você incluso). */
  allyPicks: number[];
  bans: BanAdvice | null;
  picks: PickAdvice | null;
  synergy: SynergyAdvice | null;
  /** Idioma da janela (locale do cliente, ex. "pt_BR"): configuração, senão o do cliente, senão o do sistema. */
  locale: string;
}

export type DamageType = "physical" | "magic" | "mixed";
export type Defense = "armor" | "magicResist";
export interface TeamMix { physical: number; magic: number; mixed: number; unknown: number; physicalShare: number; magicShare: number }
export interface DefItem { id: number; wr: number; pr: number; n: number; slot: string }
export interface Composition {
  allies: TeamMix;
  enemies: TeamMix;
  types: Record<string, DamageType>;
  need: Defense | null;
  items: DefItem[];
  note: string | null;
}

/** `categories` só nos itens (ex. "Armor", "SpellBlock", "Boots", "Consumable"). */
export interface Named { name: string; icon: string; categories?: string[] }
export interface Champion { id: number; name: string; alias: string }
/** `slots` = runas por fileira (`slots[0]` = keystones). */
export interface Style { id: number; name: string; icon: string; perks: number[]; slots: number[][] }
export interface Assets {
  champions: Record<string, Champion>;
  items: Record<string, Named>;
  perks: Record<string, Named>;
  styles: Style[];
  spells: Record<string, Named>;
  /** Fileiras dos fragmentos: ataque, flexível, defesa. */
  shardSlots: number[][];
}

export interface Settings {
  tier: string;
  region: string;
  patch: string;
  autoImport: boolean;
  importRunes: boolean;
  importItems: boolean;
  importSpells: boolean;
  runeChoice: "pick" | "win";
  minGames: number;
  replaceCurrentPage: boolean;
  flashSlot: number;
  closeInGame: boolean;
  /** Overlay na partida (próximo item, CS/min, inibidores), só pela API oficial do jogo. */
  overlay: boolean;
  suggestBans: boolean;
  suggestPicks: boolean;
  pickPool: "mastery" | "tier";
  /** Métrica das sugestões de ban e pick. */
  metric: Metric;
  checkUpdates: boolean;
  /** "phase" | "grid" | "tabs" | "classic" */
  layout: string;
  /** "" = automático (do cliente do League); senão um locale do cliente ("pt_BR"). */
  language: string;
  /** Último locale lido do cliente (`/riotclient/region-locale`), para quando ele está fechado. */
  clientLocale: string;
}

/** Overlay da partida (`overlay.rs`): só dados do próprio jogador, da Live Client Data. */
export interface OverlayState {
  visible: boolean;
  locale: string;
  gameTime: number;
  cs: number;
  csPerMin: number;
  gold: number;
  next: { id: number; name: string; icon: string; price: number; cost: number; missing: number } | null;
  /** `side`: "ally" | "enemy" | "blue" | "red"; `lane`: "top" | "mid" | "bot". */
  inhibs: { side: string; lane: string; respawnIn: number }[];
}

/** Versão nova no GitHub Releases (`update.rs`). */
export interface UpdateInfo { version: string; current: string; notes: string | null; url: string }
export interface UpdateProgress { downloaded: number; total: number | null }

/** Fora do Tauri (navegador com `npm run dev`) a página usa dados sintéticos de `mock.ts`. */
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let mockAssetsWithIcons: Promise<Assets> | undefined;
let mockBanTurnSet = false;
let mockLang: Promise<void> | undefined;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  const m = await import("./mock");
  // Para ver cada layout e etapa no navegador: ?layout=phase|grid|tabs|classic&fase=bans|picks|build
  const q = new URLSearchParams(location.search);
  // ?lang=xx_YY: idioma (como se o cliente estivesse nele); sem, pt_BR.
  if (!mockLang) {
    m.mockSettings.clientLocale = q.get("lang") ?? "pt_BR";
    mockLang = m.mockLanguage(m.mockSettings.clientLocale);
  }
  await mockLang;
  if (cmd === "save_settings") {
    // Como o backend: idioma escolhido, senão o do "cliente".
    const s = (args as { settings: Settings }).settings;
    Object.assign(m.mockSettings, s, { clientLocale: m.mockSettings.clientLocale });
    await m.mockLanguage(s.language || m.mockSettings.clientLocale);
    return undefined as T;
  }
  if (q.get("layout")) m.mockSettings.layout = q.get("layout")!;
  const fase = q.get("fase");
  if (fase) Object.assign(m.mockSnapshot, { banning: fase === "bans", locked: fase === "build" });
  // Primeira consulta com ?fase=bans: é a sua vez de banir (depois o mock segue o clique).
  if (fase === "bans" && !mockBanTurnSet) {
    mockBanTurnSet = true;
    m.mockSnapshot.canBan = true;
  }
  if (cmd === "ban_champion") {
    m.mockSnapshot.canBan = false;
    return undefined as T;
  }
  // ?offline=1: só a janela aberta, sem o cliente do League (montar a página no app).
  if (q.get("offline") === "1") Object.assign(m.mockSnapshot, { connected: false, phase: "None", inChampSelect: false, canBan: false });
  if (["get_own_pages", "save_own_runes", "save_own_runes_page", "delete_own_runes", "choose_runes"].includes(cmd)) {
    return m.mockOwnCommand(cmd, args ?? {}) as T;
  }
  if (cmd === "get_overlay") {
    // Nome e ícone do item no idioma do "cliente" (CommunityDragon), como no app.
    const a = await (mockAssetsWithIcons ??= import("./mockIcons").then((x) => x.withIcons(m.mockAssets)));
    const o: OverlayState = structuredClone(m.mockOverlay);
    const it = o.next ? a.items[String(o.next.id)] : undefined;
    if (o.next && it) Object.assign(o.next, { name: it.name, icon: it.icon });
    o.locale = m.mockSnapshot.locale;
    return o as T;
  }
  if (cmd === "get_assets") {
    return (await (mockAssetsWithIcons ??= import("./mockIcons").then((x) => x.withIcons(m.mockAssets)))) as T;
  }
  const fake: Record<string, unknown> = {
    get_snapshot: m.mockSnapshot,
    get_settings: m.mockSettings,
    reimport: m.mockSnapshot.import,
    get_composition: m.mockComposition,
    // Aviso de versão nova só com ?update=1 (os prints da landing page saem sem ele).
    get_update: q.get("update") === "1" ? m.mockUpdate : null,
    check_update: m.mockUpdate,
  };
  return fake[cmd] as T;
}

export const api = {
  snapshot: () => call<Snapshot>("get_snapshot"),
  assets: () => call<Assets | null>("get_assets"),
  settings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<void>("save_settings", { settings }),
  lookup: (championId: number, lane: string | null) => call<void>("lookup", { championId, lane }),
  setVs: (vsChampionId: number | null, vsLane: string | null) => call<void>("set_vs", { vsChampionId, vsLane }),
  reimport: () => call<ImportReport>("reimport"),
  composition: (allies: number[], enemies: number[]) => call<Composition>("get_composition", { allies, enemies }),
  /** Na sua vez de banir: marca e trava o ban desse campeão no cliente. */
  banChampion: (championId: number) => call<void>("ban_champion", { championId }),
  /** Páginas de runas próprias (todas). */
  ownPages: () => call<OwnPage[]>("get_own_pages"),
  /** Copia a página atual do cliente (só GET) como página sua do campeão: nova (`pageId` null) ou por cima. */
  saveOwnRunes: (championId: number, pageId: number | null, name: string | null) =>
    call<OwnPage[]>("save_own_runes", { championId, pageId, name }),
  /** Página montada na janela (funciona sem o cliente aberto); `pageId` null = nova. */
  saveOwnRunesPage: (championId: number, pageId: number | null, name: string, primaryStyle: number, subStyle: number, perks: number[]) =>
    call<OwnPage[]>("save_own_runes_page", { championId, pageId, name, primaryStyle, subStyle, perks }),
  deleteOwnRunes: (championId: number, pageId: number) => call<OwnPage[]>("delete_own_runes", { championId, pageId }),
  /** Mais jogada / Maior WR / uma página sua no campeão em tela (o backend leva para o cliente). */
  chooseRunes: (choice: "pick" | "win" | "own", pageId: number | null) => call<OwnPage[]>("choose_runes", { choice, pageId }),
  /** Estado do overlay da partida (depois vem no evento "overlay"). */
  overlay: () => call<OverlayState>("get_overlay"),
  onOverlay: (cb: (s: OverlayState) => void): Promise<UnlistenFn> =>
    inTauri ? listen<OverlayState>("overlay", (e) => cb(e.payload)) : Promise.resolve(() => {}),
  onSnapshot: (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
    inTauri ? listen<Snapshot>("snapshot", (e) => cb(e.payload)) : Promise.resolve(() => {}),
  /** Atualização já encontrada nesta execução (a janela é recriada a cada seleção). */
  update: () => call<UpdateInfo | null>("get_update"),
  /** Consulta o GitHub agora. */
  checkUpdate: () => call<UpdateInfo | null>("check_update"),
  /** Baixa, confere a assinatura e instala; o app fecha e o instalador o abre de novo. */
  installUpdate: () => call<void>("install_update"),
  onUpdate: (cb: (u: UpdateInfo) => void): Promise<UnlistenFn> =>
    inTauri ? listen<UpdateInfo>("update-available", (e) => cb(e.payload)) : Promise.resolve(() => {}),
  onUpdateProgress: (cb: (p: UpdateProgress) => void): Promise<UnlistenFn> =>
    inTauri ? listen<UpdateProgress>("update-progress", (e) => cb(e.payload)) : Promise.resolve(() => {}),
  /** Versão instalada ("dev" no navegador). */
  version: async (): Promise<string> => (inTauri ? (await import("@tauri-apps/api/app")).getVersion() : "dev"),
};

/** Esta janela é o overlay da partida? (No navegador: `?view=overlay`.) */
export function isOverlayWindow(): boolean {
  if (typeof window === "undefined") return false;
  if (!inTauri) return new URLSearchParams(location.search).get("view") === "overlay";
  const internals = (window as unknown as { __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } } })
    .__TAURI_INTERNALS__;
  return internals?.metadata?.currentWindow?.label === "overlay";
}

/** Ícones servidos pelo backend a partir do cliente (protocolo `lcu`). */
export function icon(path: string | undefined | null): string {
  if (!path) return "";
  if (!inTauri) return cdragonUrl(path);
  const windows = navigator.userAgent.includes("Windows");
  return (windows ? "http://lcu.localhost" : "lcu://localhost") + path;
}

export function championIcon(id: number): string {
  return icon(`/lol-game-data/assets/v1/champion-icons/${id}.png`);
}

export { pct, pts, num } from "./i18n/index.svelte";

const LANE_IDS = ["top", "jungle", "middle", "bottom", "support"];
const PHASE_IDS = [
  "None", "Lobby", "Matchmaking", "ReadyCheck", "ChampSelect", "GameStart", "InProgress", "Reconnect",
  "WaitingForStats", "PreEndOfGame", "EndOfGame",
];

/** Objeto de leitura que traduz na hora (`LANES.jungle` = "Selva" em pt_BR); `undefined` fora da lista. */
function translated(ids: string[], prefix: string): Record<string, string> {
  return new Proxy({} as Record<string, string>, {
    get: (_, k) => (typeof k === "string" && ids.includes(k) ? t(`${prefix}.${k}`) : undefined),
    has: (_, k) => typeof k === "string" && ids.includes(k),
    ownKeys: () => ids,
    getOwnPropertyDescriptor: (_, k) =>
      typeof k === "string" && ids.includes(k)
        ? { enumerable: true, configurable: true, value: t(`${prefix}.${k}`) }
        : undefined,
  });
}

/** Nomes das lanes do lolalytics no idioma da janela. */
export const LANES = translated(LANE_IDS, "lane");
/** Fases do cliente (`gameflow-phase`) no idioma da janela. */
export const PHASES = translated(PHASE_IDS, "phase");

/** Filtros de elo (os nomes vêm da tradução). */
export const tiers = (): [string, string][] =>
  ["emerald_plus", "platinum_plus", "gold_plus", "diamond_plus", "d2_plus", "master_plus", "all"].map((v) => [v, t(`tier.${v}`)]);

export const regions = (): [string, string][] => [
  ["all", t("region.all")],
  ["br", "BR"],
  ["na", "NA"],
  ["euw", "EUW"],
  ["kr", "KR"],
];

export const patches = (): [string, string][] => [
  ["", t("patch.current")],
  ["7", t("patch.days", { count: 7 })],
  ["30", t("patch.days", { count: 30 })],
];

// Mesmas regras de `import/itemset.rs` (núcleo, primeira volta, situacionais), para a janela
// mostrar o mesmo que vai para o conjunto de itens.
const NOT_BUILD = ["Consumable", "Trinket", "Jungle"];
const EXTRA_ITEMS = 4;

export function itemHas(assets: Assets | null, id: number, category: string): boolean {
  return assets?.items[id]?.categories?.includes(category) ?? false;
}

function isBuildItem(assets: Assets | null, id: number): boolean {
  return !!assets?.items[id] && !NOT_BUILD.some((c) => itemHas(assets, id, c));
}

/** `pick_items`: até 3 mais jogados com amostra mínima (se nenhum alcança, o mais jogado). */
function pickItems(list: ItemStat[], assets: Assets | null, minGames: number): ItemStat[] {
  const known = list.filter((i) => assets?.items[i.id]);
  const out = known.filter((i) => i.n >= minGames).slice(0, 3);
  return out.length ? out : known.slice(0, 1);
}

/** Núcleos com amostra mínima, só com itens conhecidos (o 1º vai para o conjunto). */
export function coreSets(build: Build, assets: Assets | null, minGames: number): { combo: Combo; ids: number[] }[] {
  return (build.core ?? [])
    .filter((c) => c.n >= minGames)
    .map((combo) => ({ combo, ids: combo.ids.map(Number).filter((id) => assets?.items[id]) }))
    .filter((c) => c.ids.length > 0);
}

/** Primeira volta: `earlyItem` por partidas, sem consumíveis, berloques e o que já está nos inícios. */
export function firstBack(build: Build, assets: Assets | null, minGames: number): ItemStat[] {
  const starts = build.startSets.slice(0, 2).flatMap((s) => s.ids.map(Number));
  return (build.earlyItems ?? [])
    .filter((i) => i.n >= minGames && !starts.includes(i.id) && isBuildItem(assets, i.id))
    .slice(0, EXTRA_ITEMS);
}

/**
 * Situacionais, fora dos mais jogados (slots, botas, núcleo) e sem botas. WR: maior WR com amostra
 * mínima. WPA: maior WPA contra o WR do campeão (sem corte de amostra), só os positivos.
 */
export function situationalItems(build: Build, assets: Assets | null, minGames: number, metric: Metric): ItemStat[] {
  const usual = new Set<number>([...build.slots, build.boots].flatMap((s) => pickItems(s, assets, minGames).map((i) => i.id)));
  coreSets(build, assets, minGames)[0]?.ids.forEach((id) => usual.add(id));
  const open = (i: ItemStat) => !usual.has(i.id) && isBuildItem(assets, i.id) && !itemHas(assets, i.id, "Boots");
  const list = build.winningItems ?? [];
  if (metric === "wr") return list.filter((i) => i.n >= minGames && open(i)).slice(0, EXTRA_ITEMS);
  const score = itemWpa(list, build.wr, false);
  const s = (i: ItemStat) => score.get(i.id) ?? 0;
  return list
    .filter((i) => open(i) && s(i) > 0)
    .sort((a, b) => s(b) - s(a) || a.id - b.id)
    .slice(0, EXTRA_ITEMS);
}


/** A página própria no formato das do lolalytics (sem WR nem amostra). */
export function ownRuneSet(p: OwnPage): RuneSet {
  return { primary: p.perks.slice(0, 4), secondary: p.perks.slice(4, 6), shards: p.perks.slice(6, 9), wr: 0, n: 0 };
}

/** WR da runa na posição `i` da página (0–3 primária, 4–5 secundária, 6–8 fragmentos), como em `RuneAlternatives`. */
export function perkStat(stats: RuneStats | null | undefined, id: number, i: number): PerkStat | undefined {
  if (!stats) return undefined;
  const source = i >= 4 && i <= 5 ? stats.secondary : i === 7 ? stats.flex : stats.primary;
  return source[id];
}

export interface RuneDiff { mine: number; other: number; delta: number | null; n: number | null }

/** Runas da sua página que diferem de `other` na mesma posição, com a diferença em pontos de WR. */
export function runeDiffs(mine: RuneSet, other: RuneSet | null, stats: RuneStats | null): RuneDiff[] {
  if (!other) return [];
  const a = [...mine.primary, ...mine.secondary, ...mine.shards];
  const b = [...other.primary, ...other.secondary, ...other.shards];
  return a.flatMap((id, i) => {
    if (id === b[i]) return [];
    const sa = perkStat(stats, id, i);
    const sb = perkStat(stats, b[i], i);
    return [{ mine: id, other: b[i], delta: sa && sb ? sa.wr - sb.wr : null, n: sa?.n ?? null }];
  });
}
