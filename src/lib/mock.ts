// Dados sintéticos para ver a interface no navegador (`npm run dev`), fora do Tauri.
// Números inventados: não redistribuímos dados do lolalytics.
import type { Assets, Settings, Snapshot } from "./api";

const item = (name: string) => ({ name, icon: "" });

export const mockAssets: Assets = {
  champions: {
    "31": { id: 31, name: "Cho'Gath", alias: "Chogath" }, "62": { id: 62, name: "Wukong", alias: "MonkeyKing" },
    "234": { id: 234, name: "Viego", alias: "Viego" }, "122": { id: 122, name: "Darius", alias: "Darius" },
    "876": { id: 876, name: "Lillia", alias: "Lillia" }, "99": { id: 99, name: "Lux", alias: "Lux" },
    "64": { id: 64, name: "Lee Sin", alias: "LeeSin" }, "60": { id: 60, name: "Elise", alias: "Elise" },
    "19": { id: 19, name: "Warwick", alias: "Warwick" }, "14": { id: 14, name: "Sion", alias: "Sion" },
  },
  items: {
    "1102": item("Filhote de Brasa"), "2003": item("Poção de Vida"), "3009": item("Botas da Rapidez"),
    "3111": item("Passos de Mercúrio"), "3047": item("Botas Galvanizadas de Aço"), "3084": item("Coração de Aço"),
    "3152": item("Cinto-foguete Hextec"), "3742": item("Couraça do Defunto"), "4401": item("Força da Natureza"),
    "3075": item("Armadura de Espinhos"), "6665": item("Jak'Sho"), "2502": item("Égide Imperecível"),
  },
  perks: Object.fromEntries([9923, 8126, 8140, 8106, 8224, 8234, 5008, 5010, 5001, 8446, 8242, 5005, 5013].map((id) => [String(id), item(`Runa ${id}`)])),
  styles: [
    { id: 8100, name: "Dominação", icon: "", perks: [9923, 8126, 8140, 8106] },
    { id: 8200, name: "Feitiçaria", icon: "", perks: [8224, 8234] },
    { id: 8400, name: "Determinação", icon: "", perks: [8446, 8242] },
  ],
  spells: { "4": item("Flash"), "11": item("Golpear"), "6": item("Fantasma") },
};

const row = (id: number, wr: number, pr: number, n: number) => ({ id, wr, pr, n });

export const mockSnapshot: Snapshot = {
  connected: true,
  phase: "ChampSelect",
  inChampSelect: true,
  locked: true,
  championId: 31,
  lane: "jungle",
  loading: false,
  pageUrl: "https://lolalytics.com/lol/chogath/build/?tier=emerald_plus&lane=jungle",
  warning: null,
  error: null,
  import: {
    runes: { ok: true, msg: "página criada" },
    items: { ok: true, msg: "conjunto gravado (9 blocos)" },
    spells: { ok: false, msg: "fora da seleção de campeões" },
  },
  vsChampionId: 0,
  vsLane: null,
  enemyPicks: [234, 122],
  bans: {
    loading: false,
    basedOn: [{ championId: 31, lane: "jungle", me: true }, { championId: 99, lane: "middle", me: false }],
    picks: [
      { championId: 876, gain: 0.21, tier: null, reasons: [{ allyChampionId: 31, lane: "jungle", wr: 46.1, baseWr: 53.3, n: 900 }] },
      { championId: 62, gain: 0.12, tier: null, reasons: [
        { allyChampionId: 31, lane: "jungle", wr: 49.0, baseWr: 53.3, n: 700 },
        { allyChampionId: 99, lane: "middle", wr: 48.2, baseWr: 51.0, n: 300 },
      ] },
    ],
    note: null,
    byTier: false,
    lane: null,
    pageUrl: null,
  },
  picks: {
    loading: false,
    lane: "jungle",
    pool: "mastery",
    enemies: [234, 122],
    allies: [{ championId: 99, lane: "middle", me: false }],
    note: null,
    pageUrl: "https://lolalytics.com/lol/tierlist/?tier=emerald_plus&lane=jungle",
    suggestions: [
      { championId: 19, total: 3.1, base: 1.4, wr: 53.1, games: 30000, mastery: 98000, parts: [
        { kind: "counter", championId: 234, lane: "jungle", delta: 1.2, wr: 52.4, n: 1500 },
        { kind: "synergy", championId: 99, lane: "middle", delta: 0.6, wr: 55.0, n: 300 },
        { kind: "counter", championId: 122, lane: "top", delta: -0.1, wr: 50.2, n: 400 },
      ] },
      { championId: 31, total: 2.2, base: 1.9, wr: 53.5, games: 60000, mastery: 580000, parts: [
        { kind: "synergy", championId: 99, lane: "middle", delta: 0.5, wr: 56.1, n: 420 },
        { kind: "counter", championId: 234, lane: "jungle", delta: -0.2, wr: 50.8, n: 1200 },
      ] },
      { championId: 60, total: 0.4, base: 1.3, wr: 52.9, games: 39000, mastery: 12000, parts: [
        { kind: "counter", championId: 234, lane: "jungle", delta: -0.9, wr: 49.0, n: 1700 },
      ] },
    ],
  },
  synergy: {
    championId: 31,
    lane: "jungle",
    baseWr: 53.5,
    pageUrl: "https://lolalytics.com/lol/chogath/build/?tier=emerald_plus&lane=jungle",
    lines: [{ allyChampionId: 99, lane: "middle", wr: 55.0, delta: 0.8, n: 300 }],
  },
  build: {
    slug: "chogath", lane: "jungle", patch: "16.23", wr: 53.3, pr: 5.1, br: 7.0, n: 14000, rank: 7, rankTotal: 79,
    runesPick: { primary: [9923, 8126, 8140, 8106], secondary: [8224, 8234], shards: [5008, 5010, 5001], wr: 53.4, n: 13000 },
    runesWin: { primary: [9923, 8126, 8140, 8106], secondary: [8446, 8242], shards: [5005, 5008, 5013], wr: 54.2, n: 900 },
    spells: [{ ids: ["4", "11"], wr: 53.4, pr: 98.8, n: 13100 }, { ids: ["6", "11"], wr: 53.3, pr: 1.1, n: 150 }],
    skillPriority: [{ ids: ["E", "Q", "W"], wr: 54.1, pr: 78, n: 10000 }],
    skillOrder: "EQWEEREQEQRQQWW",
    startSets: [{ ids: ["1102", "2003"], wr: 51.8, pr: 32.7, n: 4300 }, { ids: ["1102"], wr: 53.6, pr: 27.2, n: 3600 }],
    boots: [row(3009, 55.4, 58.6, 7800), row(3111, 51.7, 19.3, 2500), row(3047, 51.6, 18.7, 2480)],
    slots: [
      [row(3084, 53.3, 69, 9700), row(3152, 53.1, 27, 3800), row(3742, 58.7, 1, 143)],
      [row(3742, 55.6, 52, 7300), row(4401, 53.4, 9, 1300)],
      [row(4401, 59.5, 30, 3100), row(3742, 61.3, 16, 1600), row(3075, 52.7, 12, 1200)],
      [row(3075, 59.9, 20, 980), row(6665, 63.0, 18, 890), row(4401, 61.9, 14, 690)],
      [row(6665, 67.6, 15, 300), row(2502, 61.2, 14, 290), row(3075, 58.6, 12, 150)],
    ],
    lanes: [["jungle", 60], ["top", 32], ["middle", 5.5], ["support", 2]],
    vs: null,
  },
};

export const mockSettings: Settings = {
  tier: "emerald_plus", region: "all", patch: "", autoImport: true, importRunes: true, importItems: true,
  importSpells: true, runeChoice: "pick", minGames: 200, replaceCurrentPage: false, flashSlot: 1, closeInGame: true,
  suggestBans: true, suggestPicks: true, pickPool: "mastery",
};
