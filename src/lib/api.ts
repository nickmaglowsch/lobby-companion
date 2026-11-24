// Tipos espelhando o serde do backend (src-tauri/src/app.rs, lola/model.rs, assets.rs, settings.rs).
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ItemStat { id: number; wr: number; pr: number; n: number }
export interface Combo { ids: string[]; wr: number; pr: number; n: number }
export interface RuneSet { primary: number[]; secondary: number[]; shards: number[]; wr: number; n: number }

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
export interface BanPick { championId: number; gain: number; reasons: BanReason[] }
export interface BanAdvice { loading: boolean; basedOn: Ally[]; picks: BanPick[]; note: string | null }

export interface Outcome { ok: boolean; msg: string }
export interface ImportReport { runes: Outcome | null; items: Outcome | null; spells: Outcome | null }

export interface Snapshot {
  connected: boolean;
  phase: string;
  inChampSelect: boolean;
  locked: boolean;
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
  suggestBans: boolean;
}

/** Fora do Tauri (navegador com `npm run dev`) a página usa dados sintéticos de `mock.ts`. */
const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  const m = await import("./mock");
  const fake: Record<string, unknown> = {
    get_snapshot: m.mockSnapshot,
    get_assets: m.mockAssets,
    get_settings: m.mockSettings,
    reimport: m.mockSnapshot.import,
    get_composition: m.mockComposition,
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
  onSnapshot: (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
    inTauri ? listen<Snapshot>("snapshot", (e) => cb(e.payload)) : Promise.resolve(() => {}),
};

/** Ícones servidos pelo backend a partir do cliente (protocolo `lcu`). */
export function icon(path: string | undefined | null): string {
  if (!path) return "";
  const windows = navigator.userAgent.includes("Windows");
  return (windows ? "http://lcu.localhost" : "lcu://localhost") + path;
}

export function championIcon(id: number): string {
  return icon(`/lol-game-data/assets/v1/champion-icons/${id}.png`);
}

export const pct = (x: number) => `${x.toFixed(1).replace(".", ",")}%`;
export const num = (x: number) => x.toLocaleString("pt-BR");

export const LANES: Record<string, string> = {
  top: "Topo",
  jungle: "Selva",
  middle: "Meio",
  bottom: "Atirador",
  support: "Suporte",
};

export const PHASES: Record<string, string> = {
  None: "Cliente aberto",
  Lobby: "No lobby",
  Matchmaking: "Na fila",
  ReadyCheck: "Partida encontrada",
  ChampSelect: "Seleção de campeões",
  GameStart: "Carregando",
  InProgress: "Em partida",
  Reconnect: "Reconectar",
  WaitingForStats: "Fim de jogo",
  PreEndOfGame: "Fim de jogo",
  EndOfGame: "Fim de jogo",
};

export const TIERS: [string, string][] = [
  ["emerald_plus", "Esmeralda+"],
  ["platinum_plus", "Platina+"],
  ["gold_plus", "Ouro+"],
  ["diamond_plus", "Diamante+"],
  ["d2_plus", "Diamante 2+"],
  ["master_plus", "Mestre+"],
  ["all", "Todos"],
];

export const REGIONS: [string, string][] = [
  ["all", "Global"],
  ["br", "BR"],
  ["na", "NA"],
  ["euw", "EUW"],
  ["kr", "KR"],
];

export const PATCHES: [string, string][] = [
  ["", "Patch atual"],
  ["7", "Últimos 7 dias"],
  ["30", "Últimos 30 dias"],
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

/** Situacionais: maior WR com amostra mínima, fora dos mais jogados (slots, botas, núcleo) e sem botas. */
export function situationalItems(build: Build, assets: Assets | null, minGames: number): ItemStat[] {
  const usual = new Set<number>([...build.slots, build.boots].flatMap((s) => pickItems(s, assets, minGames).map((i) => i.id)));
  coreSets(build, assets, minGames)[0]?.ids.forEach((id) => usual.add(id));
  return (build.winningItems ?? [])
    .filter((i) => i.n >= minGames && !usual.has(i.id) && isBuildItem(assets, i.id) && !itemHas(assets, i.id, "Boots"))
    .slice(0, EXTRA_ITEMS);
}

/** `1.234` → `"+1,2"` (pontos de WR com sinal). */
export const pts = (x: number) => `${x >= 0 ? "+" : "−"}${Math.abs(x).toFixed(1).replace(".", ",")}`;
