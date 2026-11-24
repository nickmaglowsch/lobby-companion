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
}

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
  bans: BanAdvice | null;
}

export interface Named { name: string; icon: string }
export interface Champion { id: number; name: string; alias: string }
export interface Style { id: number; name: string; icon: string; perks: number[] }
export interface Assets {
  champions: Record<string, Champion>;
  items: Record<string, Named>;
  perks: Record<string, Named>;
  styles: Style[];
  spells: Record<string, Named>;
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
