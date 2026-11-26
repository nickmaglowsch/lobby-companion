// Estado da janela e ações, compartilhados pelos layouts (Clássico, Grade, Abas, Por fase).
// Os dados vêm do backend (`api`): snapshot do cliente + build do lolalytics, assets e configurações.
import {
  api, minGamesFor, ownRuneSet,
  type Assets, type Champion, type Composition, type OwnPage, type RuneChoice, type Settings, type Snapshot,
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
  /** Páginas de runas próprias (todas); a do build em tela é `ownPage`. */
  ownPages = $state<OwnPage[]>([]);
  ownBusy = $state(false);
  /** Editor da página própria aberto (funciona sem o cliente). */
  editorOpen = $state(false);
  ownError = $state<string | null>(null);
  /** Campeão sendo banido agora (clique na sugestão) e o erro do último ban. */
  banBusy = $state<number | null>(null);
  banError = $state<string | null>(null);
  private compSeq = 0;
  private compKey = "";

  build = $derived(this.snap?.build ?? null);
  champ = $derived(this.snap && this.assets ? this.assets.champions[this.snap.championId] : undefined);
  champions = $derived<Champion[]>(
    this.assets ? Object.values(this.assets.champions).sort((a, b) => a.name.localeCompare(b.name, "pt-BR")) : [],
  );
  layout = $derived<Layout>((this.settings?.layout as Layout) ?? "phase");
  /** Sua página para o campeão e a lane em tela (aparece como "Minha" ao lado das outras duas). */
  ownPage = $derived(
    this.ownPages.find((p) => p.championId === this.snap?.championId && p.lane === this.build?.lane) ?? null,
  );
  runeTab = $derived<RuneChoice>(this.ownPage?.active ? "mine" : this.settings?.runeChoice ?? "pick");
  runeSet = $derived(
    !this.build ? null
    : this.runeTab === "mine" && this.ownPage ? ownRuneSet(this.ownPage)
    : this.runeTab === "win" ? this.build.runesWin ?? this.build.runesPick
    : this.build.runesPick,
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
    this.ownPages = await api.ownPages();
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

  /** Mais jogada / Maior WR / Minha. A troca já aparece; o backend grava e leva para o cliente. */
  async chooseRunes(choice: RuneChoice) {
    if (!this.settings || choice === this.runeTab) return;
    const own = this.ownPage;
    if (own) own.active = choice === "mine";
    if (choice !== "mine") this.settings.runeChoice = choice;
    await this.ownCall(api.chooseRunes(choice));
  }

  /** Clique numa sugestão de ban na sua vez: bane no cliente (marca e trava). */
  async ban(championId: number) {
    if (!this.snap?.canBan || this.banBusy !== null) return;
    this.banBusy = championId;
    this.banError = null;
    try {
      await api.banChampion(championId);
      // O cliente confirma pelo WebSocket; até lá, nada de segundo clique.
      if (this.snap) this.snap.canBan = false;
    } catch (e) {
      this.banError = `Ban não feito: ${e}`;
    } finally {
      this.banBusy = null;
    }
  }

  /** Guarda a página atual do cliente como a sua para o campeão e a lane em tela. */
  async saveOwnRunes() {
    const id = this.snap?.championId;
    const lane = this.build?.lane;
    if (id && lane) await this.ownCall(api.saveOwnRunes(id, lane));
  }

  /** Salva a página montada no editor para o campeão e a lane em tela; devolve se deu certo. */
  async saveOwnRunesPage(primaryStyle: number, subStyle: number, perks: number[]): Promise<boolean> {
    const id = this.snap?.championId;
    const lane = this.build?.lane;
    if (!id || !lane) return false;
    await this.ownCall(api.saveOwnRunesPage(id, lane, primaryStyle, subStyle, perks));
    return !this.ownError;
  }

  async deleteOwnRunes(championId: number, lane: string) {
    await this.ownCall(api.deleteOwnRunes(championId, lane));
  }

  private async ownCall(p: Promise<OwnPage[]>) {
    this.ownBusy = true;
    this.ownError = null;
    try {
      this.ownPages = await p;
    } catch (e) {
      this.ownError = String(e);
      // Volta ao que o backend tem (a troca otimista pode ter falhado).
      this.ownPages = await api.ownPages().catch(() => this.ownPages);
      this.settings = await api.settings().catch(() => this.settings);
    } finally {
      this.ownBusy = false;
    }
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
