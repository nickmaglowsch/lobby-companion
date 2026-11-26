// Estado da janela e ações, compartilhados pelos layouts (Clássico, Grade, Abas, Por fase).
// Os dados vêm do backend (`api`): snapshot do cliente + build do lolalytics, assets e configurações.
import {
  api, minGamesFor,
  type Assets, type Champion, type Composition, type Settings, type Snapshot,
} from "./api";

export type Layout = "phase" | "grid" | "tabs" | "classic";
export const LAYOUTS: [Layout, string][] = [
  ["phase", "Por fase"],
  ["grid", "Grade"],
  ["tabs", "Abas"],
  ["classic", "Clássico (lista)"],
];

/** Etapa da seleção para os layouts de uma tela: bans → escolha → build (ou fora da seleção). */
export type Stage = "bans" | "picks" | "build" | "idle";

class AppState {
  snap = $state<Snapshot | null>(null);
  assets = $state<Assets | null>(null);
  settings = $state<Settings | null>(null);
  version = $state("");
  importing = $state(false);
  importError = $state<string | null>(null);
  /** Lane do confronto escolhida na janela (null = a do snapshot/build). */
  vsLaneChoice = $state<string | null>(null);
  settingsOpen = $state(false);
  updateCheck = $state<string | null>(null);
  composition = $state<Composition | null>(null);
  private compSeq = 0;
  private compKey = "";

  build = $derived(this.snap?.build ?? null);
  champ = $derived(this.snap && this.assets ? this.assets.champions[this.snap.championId] : undefined);
  champions = $derived<Champion[]>(
    this.assets ? Object.values(this.assets.champions).sort((a, b) => a.name.localeCompare(b.name, "pt-BR")) : [],
  );
  layout = $derived<Layout>((this.settings?.layout as Layout) ?? "phase");
  runeTab = $derived(this.settings?.runeChoice ?? "pick");
  runeSet = $derived(
    this.build ? (this.runeTab === "win" ? this.build.runesWin ?? this.build.runesPick : this.build.runesPick) : null,
  );
  minGames = $derived(
    this.build ? minGamesFor(this.build, this.settings?.minGames ?? 200) : this.settings?.minGames ?? 200,
  );
  vsLane = $derived(this.vsLaneChoice ?? this.snap?.vsLane ?? this.build?.vs?.lane ?? this.build?.lane ?? null);
  /** Itens de defesa do build contra o dano do time inimigo (destacados nas listas de itens). */
  defHighlight = $derived(this.composition?.items.map((i) => i.id) ?? []);
  stage = $derived<Stage>(
    !this.snap?.inChampSelect
      ? this.build ? "build" : "idle"
      : this.snap.banning ? "bans" : this.snap.locked ? "build" : "picks",
  );

  champName = (id: number) => this.assets?.champions[id]?.name ?? `#${id}`;

  findChampion(text: string): Champion | undefined {
    const q = text.trim().toLocaleLowerCase("pt-BR");
    if (!q) return undefined;
    return (
      this.champions.find((c) => c.name.toLocaleLowerCase("pt-BR") === q) ??
      this.champions.find((c) => c.name.toLocaleLowerCase("pt-BR").startsWith(q))
    );
  }

  /** Carrega tudo e passa a ouvir o backend. Devolve a função que para de ouvir. */
  async init(): Promise<() => void> {
    this.snap = await api.snapshot();
    this.settings = await api.settings();
    this.version = await api.version();
    await this.refreshAssets();
    this.refreshComposition();
    return api.onSnapshot((s) => {
      // Seleção nova: esquece a lane de confronto escolhida na anterior.
      if (s.phase === "ChampSelect" && this.snap?.phase !== "ChampSelect") this.vsLaneChoice = null;
      this.snap = s;
      if (s.connected) this.refreshAssets();
      this.refreshComposition();
    });
  }

  async refreshAssets() {
    if (!this.assets) this.assets = await api.assets();
  }

  /** Composição de dano (tipo de dano pelo cliente) quando mudam os campeões ou o build em tela. */
  refreshComposition() {
    const s = this.snap;
    const allies = s?.allyPicks ?? [];
    const enemies = s?.enemyPicks ?? [];
    const b = s?.build;
    const key = JSON.stringify([allies, enemies, b?.slug, b?.lane, b?.vs?.championId, b?.n, this.minGames]);
    if (key === this.compKey) return;
    this.compKey = key;
    const mine = ++this.compSeq;
    if (!allies.length && !enemies.length) {
      this.composition = null;
      return;
    }
    api.composition(allies, enemies).then(
      (c) => { if (mine === this.compSeq) this.composition = c; },
      () => { if (mine === this.compSeq) this.composition = null; },
    );
  }

  lookup(championId: number, lane: string | null) {
    return api.lookup(championId, lane);
  }

  /** Busca pelo nome digitado; devolve true se achou. */
  search(text: string): boolean {
    const c = this.findChampion(text);
    if (c) api.lookup(c.id, null);
    return !!c;
  }

  changeLane(lane: string) {
    if (this.snap?.championId) api.lookup(this.snap.championId, lane);
  }

  chooseVs(id: number | null) {
    if (id === null) this.vsLaneChoice = null;
    api.setVs(id, id === null ? null : this.vsLane);
  }

  changeVsLane(lane: string) {
    this.vsLaneChoice = lane;
    if (this.snap?.vsChampionId) api.setVs(this.snap.vsChampionId, lane);
  }

  async importNow() {
    this.importing = true;
    this.importError = null;
    try {
      await api.reimport();
    } catch (e) {
      this.importError = String(e);
    } finally {
      this.importing = false;
    }
  }

  async save() {
    if (this.settings) await api.saveSettings($state.snapshot(this.settings));
  }

  chooseRunes(choice: "pick" | "win") {
    if (!this.settings || this.settings.runeChoice === choice) return;
    this.settings.runeChoice = choice;
    this.save();
  }

  async checkUpdateNow() {
    this.updateCheck = "Procurando…";
    try {
      // Procurar à mão mostra o aviso de novo, mesmo se você clicou em "Depois".
      try { localStorage.removeItem("lc-update-later"); } catch { /* sem armazenamento */ }
      const u = await api.checkUpdate();
      this.updateCheck = u ? `Versão ${u.version} disponível: veja o aviso.` : "Você está na versão mais nova.";
    } catch (e) {
      this.updateCheck = String(e);
    }
  }
}

export const app = new AppState();

// Só em desenvolvimento: `__app` no console do navegador para inspecionar o estado.
if (import.meta.env.DEV) (globalThis as Record<string, unknown>).__app = app;
