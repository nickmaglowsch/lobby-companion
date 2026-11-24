// Dados sintéticos para ver a interface no navegador (`npm run dev`), fora do Tauri.
// Números inventados: não redistribuímos dados do lolalytics.
import type { Assets, Composition, Settings, Snapshot } from "./api";

const item = (name: string, categories: string[] = []) => ({ name, icon: "", categories });

export const mockAssets: Assets = {
  champions: {
    "31": { id: 31, name: "Cho'Gath", alias: "Chogath" }, "62": { id: 62, name: "Wukong", alias: "MonkeyKing" },
    "234": { id: 234, name: "Viego", alias: "Viego" }, "122": { id: 122, name: "Darius", alias: "Darius" },
    "876": { id: 876, name: "Lillia", alias: "Lillia" }, "99": { id: 99, name: "Lux", alias: "Lux" },
    "64": { id: 64, name: "Lee Sin", alias: "LeeSin" }, "60": { id: 60, name: "Elise", alias: "Elise" },
    "19": { id: 19, name: "Warwick", alias: "Warwick" }, "14": { id: 14, name: "Sion", alias: "Sion" },
  },
  items: {
    "1102": item("Filhote de Brasa", ["Jungle"]), "2003": item("Poção de Vida", ["Consumable"]), "3009": item("Botas da Rapidez", ["Boots"]),
    "3111": item("Passos de Mercúrio", ["Boots", "SpellBlock"]), "3047": item("Botas Galvanizadas de Aço", ["Armor", "Boots"]),
    "3084": item("Coração de Aço", ["Health"]), "3152": item("Cinto-foguete Hextec", ["SpellDamage"]),
    "3742": item("Couraça do Defunto", ["Health", "Armor"]), "4401": item("Força da Natureza", ["Health", "SpellBlock"]),
    "3075": item("Armadura de Espinhos", ["Health", "Armor"]), "6665": item("Jak'Sho", ["Health", "Armor", "SpellBlock"]),
    "2502": item("Égide Imperecível", ["Health", "Armor"]), "1028": item("Cristal de Rubi", ["Health"]), "1001": item("Botas", ["Boots"]),
    "1011": item("Cinto do Gigante", ["Health"]), "3065": item("Semblante Espiritual", ["Health", "SpellBlock"]),
    "3068": item("Égide de Fogo Solar", ["Health", "Armor"]), "3340": item("Sentinela Invisível", ["Trinket"]),
  },
  perks: Object.fromEntries([9923, 8112, 8128, 8126, 8139, 8143, 8140, 8136, 8106, 8135, 8224, 8226, 8234, 8233, 8236, 8237,
    5008, 5005, 5007, 5010, 5001, 5011, 5013, 8446, 8242].map((id) => [String(id), item(`Runa ${id}`)])),
  styles: [
    { id: 8100, name: "Dominação", icon: "", perks: [9923, 8112, 8128, 8126, 8139, 8143, 8140, 8136, 8106, 8135],
      slots: [[9923, 8112, 8128], [8126, 8139, 8143], [8140, 8136], [8106, 8135]] },
    { id: 8200, name: "Feitiçaria", icon: "", perks: [8224, 8226, 8234, 8233, 8236, 8237],
      slots: [[], [8224, 8226], [8234, 8233], [8236, 8237]] },
    { id: 8400, name: "Determinação", icon: "", perks: [8446, 8242], slots: [[], [8446, 8242]] },
  ],
  spells: { "4": item("Flash"), "11": item("Golpear"), "6": item("Fantasma") },
  shardSlots: [[5008, 5005, 5007], [5008, 5010, 5001], [5011, 5013, 5001]],
};

const st = (pr: number, wr: number, n: number) => ({ pr, wr, n });
const days = Array.from({ length: 30 }, (_, i) => `2026-10-${String(i + 1).padStart(2, "0")}`);

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
  enemyPicks: [234, 122, 876, 62],
  allyPicks: [31, 99],
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
    core: [
      { ids: ["3084", "3009", "3742"], wr: 56.1, pr: 9.8, n: 1370 },
      { ids: ["3068", "3047", "2502"], wr: 57.3, pr: 6.1, n: 850 },
      { ids: ["3152", "3111", "4401"], wr: 52.2, pr: 2.0, n: 280 },
    ],
    winningItems: [row(3065, 63.5, 3, 420), row(6665, 63.0, 18, 890), row(3075, 59.9, 20, 980), row(3742, 58.7, 1, 143)],
    earlyItems: [row(1028, 53.9, 70, 9800), row(1001, 54.6, 55, 7700), row(1011, 53.6, 54, 7500), row(3340, 50.1, 90, 12000)],
    skill6: [{ ids: ["E", "Q", "W", "E", "E", "R"], wr: 53.6, pr: 41, n: 5700 }, { ids: ["Q", "E", "W", "E", "E", "R"], wr: 55.4, pr: 12, n: 1680 }],
    skill10: [{ ids: ["E", "Q", "W", "E", "E", "R", "E", "Q", "E", "Q"], wr: 54.0, pr: 33, n: 4600 }],
    runeStats: {
      primary: { "9923": st(70, 53.4, 9800), "8112": st(18, 52.1, 2500), "8128": st(12, 51.0, 1700),
        "8126": st(55, 53.0, 7700), "8139": st(40, 55.6, 5600), "8143": st(5, 50.2, 700),
        "8140": st(60, 53.5, 8400), "8136": st(40, 53.1, 5600), "8106": st(70, 53.3, 9800), "8135": st(30, 52.9, 4200),
        "5008": st(60, 53.2, 8400), "5005": st(35, 53.9, 4900), "5007": st(5, 52.0, 700),
        "5001": st(88, 53.4, 12300), "5011": st(8, 52.8, 1100), "5013": st(4, 54.0, 560) },
      secondary: { "8224": st(30, 53.5, 4200), "8226": st(25, 53.0, 3500), "8234": st(28, 53.4, 3900),
        "8233": st(20, 52.0, 2800), "8236": st(15, 53.2, 2100), "8237": st(2, 51.0, 280) },
      flex: { "5008": st(80, 53.4, 11200), "5010": st(12, 53.1, 1700), "5001": st(8, 54.4, 1100) },
    },
    trend: {
      dates: days,
      wr: days.map((_, i) => 52.4 + Math.sin(i / 3) * 0.8 + i * 0.03),
      wrs: days.map((_, i) => 52.4 + Math.sin(i / 4) * 0.4 + i * 0.03),
      n: days.map((_, i) => (i === 29 ? 120 : 480)),
      wrLast7: 53.4,
      wrPeriod: 52.9,
    },
    damage: { physical: 3100, magic: 14200, true: 2600 },
  },
};

export const mockComposition: Composition = {
  allies: { physical: 0, magic: 2, mixed: 0, unknown: 0, physicalShare: 0, magicShare: 1 },
  enemies: { physical: 3, magic: 1, mixed: 0, unknown: 0, physicalShare: 0.75, magicShare: 0.25 },
  types: { "31": "magic", "99": "magic", "234": "physical", "122": "physical", "876": "magic", "62": "physical" },
  need: "armor",
  items: [
    { id: 3742, wr: 55.6, pr: 52, n: 7300, slot: "2º item" },
    { id: 3075, wr: 52.7, pr: 12, n: 1200, slot: "3º item" },
    { id: 6665, wr: 63.0, pr: 18, n: 890, slot: "4º item" },
  ],
  note: "Dano inimigo concentrado: armadura ajuda. No seu build, Couraça do Defunto tem WR 55,6%.",
};

export const mockSettings: Settings = {
  tier: "emerald_plus", region: "all", patch: "", autoImport: true, importRunes: true, importItems: true,
  importSpells: true, runeChoice: "pick", minGames: 200, replaceCurrentPage: false, flashSlot: 1, closeInGame: true,
  suggestBans: true, suggestPicks: true, pickPool: "mastery",
};
