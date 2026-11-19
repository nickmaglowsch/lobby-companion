// Dados sintéticos para ver a interface no navegador (`npm run dev`), fora do Tauri.
// Números inventados: não redistribuímos dados do lolalytics.
import type { Assets, Settings, Snapshot } from "./api";

const item = (name: string) => ({ name, icon: "" });

export const mockAssets: Assets = {
  champions: { "31": { id: 31, name: "Cho'Gath", alias: "Chogath" }, "62": { id: 62, name: "Wukong", alias: "MonkeyKing" } },
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
  },
};

export const mockSettings: Settings = {
  tier: "emerald_plus", region: "all", patch: "", autoImport: true, importRunes: true, importItems: true,
  importSpells: true, runeChoice: "pick", minGames: 200, replaceCurrentPage: false, flashSlot: 1, closeInGame: true,
};
