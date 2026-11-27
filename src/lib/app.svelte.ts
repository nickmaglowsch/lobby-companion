// Estado da janela e ações, compartilhados pelos layouts (Clássico, Grade, Abas, Por fase).
// Os dados vêm do backend (`api`): snapshot do cliente + build do lolalytics, assets e configurações.
import {
  api, coreSets, firstBack, minGamesFor, ownRuneSet, situationalItems,
  type Assets, type Champion, type Combo, type Composition, type OwnPage, type RuneChoice, type Settings, type Snapshot,
} from "./api";
import { compare, lower, setLocale, t } from "./i18n/index.svelte";
import { comboWpa, scoreText, type Metric } from "./metric";

export type Layout = "phase" | "grid" | "tabs" | "classic";
/** Layouts com o nome no idioma da janela. */
export const layouts = (): [Layout, string][] =>
  (["phase", "grid", "tabs", "classic"] as Layout[]).map((l) => [l, t(`layout.${l}`)]);

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
  /** Editor da página própria aberto (funciona sem o cliente): `pageId` null = página nova. */
  editor = $state<{ pageId: number | null } | null>(null);
  ownError = $state<string | null>(null);
  /** Campeão sendo banido agora (clique na sugestão) e o erro do último ban. */
  banBusy = $state<number | null>(null);
  banError = $state<string | null>(null);
  private compSeq = 0;
  private savedLanguage = "";
  private compKey = "";
  /** `get_assets` em andamento (um por vez, até vir). */
  private assetsLoad: Promise<void> | null = null;
  /** Gravações (configurações e páginas próprias) uma depois da outra, na ordem dos cliques. */
  private queue: Promise<unknown> = Promise.resolve();
  private ownSeq = 0;
  private updateSeq = 0;

  build = $derived(this.snap?.build ?? null);
  champ = $derived(this.snap && this.assets ? this.assets.champions[this.snap.championId] : undefined);
  champions = $derived<Champion[]>(
    this.assets ? Object.values(this.assets.champions).sort((a, b) => compare(a.name, b.name)) : [],
  );
  layout = $derived<Layout>((this.settings?.layout as Layout) ?? "phase");
  /** Suas páginas do campeão em tela (cada uma vira uma opção ao lado de Mais jogada e Maior WR). */
  championPages = $derived(this.ownPages.filter((p) => p.championId === this.snap?.championId));
  /** A escolhida do campeão em tela (vai para o cliente), se houver. */
  ownPage = $derived(this.championPages.find((p) => p.active) ?? null);
  runeTab = $derived<RuneChoice>(this.ownPage ? `own:${this.ownPage.id}` : this.settings?.runeChoice ?? "pick");
  /** Métrica escolhida (o botão); os números de bans e picks vêm com a métrica de cada conta (`bans.metric`). */
  metric = $derived<Metric>(this.settings?.metric ?? "wpa");
  runeSet = $derived(
    !this.build ? null
    : this.ownPage ? ownRuneSet(this.ownPage)
    : this.runeTab === "win" ? this.build.runesWin ?? this.build.runesPick
    : this.build.runesPick,
  );
  minGames = $derived(
    this.build ? minGamesFor(this.build, this.settings?.minGames ?? 200) : this.settings?.minGames ?? 200,
  );
  /** Lanes do build com ao menos 1% das partidas (o menu de troca de lane). */
  lanes = $derived(this.build?.lanes.filter(([, p]) => p >= 1) ?? []);
  /** Núcleos, primeira volta e situacionais do build em tela (mesmas regras do conjunto de itens). */
  cores = $derived(this.build ? coreSets(this.build, this.assets, this.minGames) : []);
  firstBackItems = $derived(this.build ? firstBack(this.build, this.assets, this.minGames) : []);
  situational = $derived(this.build ? situationalItems(this.build, this.assets, this.minGames, this.metric) : []);
  vsLane = $derived(this.vsLaneChoice ?? this.snap?.vsLane ?? this.build?.vs?.lane ?? this.build?.lane ?? null);
  /** Itens de defesa do build contra o dano do time inimigo (destacados nas listas de itens). */
  defHighlight = $derived(this.composition?.items.map((i) => i.id) ?? []);
  stage = $derived<Stage>(
    !this.snap?.inChampSelect
      ? this.build ? "build" : "idle"
      : this.snap.banning ? "bans" : this.snap.locked ? "build" : "picks",
  );

  /** Número do início `i` (WR ou WPA contra o início típico). */
  startText = (i: number) => {
    const b = this.build;
    const s = b?.startSets[i];
    return b && s ? scoreText(this.metric, s.wr, comboWpa(b.startSets, b.wr)[i]) : "";
  };

  /** Número de um núcleo (WR ou WPA contra o núcleo típico). */
  coreText = (c: Combo) => {
    const b = this.build;
    if (!b) return "";
    const list = b.core ?? [];
    const k = list.findIndex((x) => x.ids.join() === c.ids.join());
    return scoreText(this.metric, c.wr, k >= 0 ? comboWpa(list, b.wr)[k] : undefined);
  };

  champName = (id: number) => this.assets?.champions[id]?.name ?? `#${id}`;

  findChampion(text: string): Champion | undefined {
    const q = lower(text.trim());
    if (!q) return undefined;
    return (
      this.champions.find((c) => lower(c.name) === q) ??
      this.champions.find((c) => lower(c.name).startsWith(q))
    );
  }

  /** Carrega tudo e passa a ouvir o backend. Devolve a função que para de ouvir. */
  async init(): Promise<() => void> {
    // Ouve antes de ler: um snapshot que chega durante a carga não se perde e vale mais que a
    // resposta do `get_snapshot` (que pode ter sido montada antes dele).
    let live = false;
    const unlisten = await api.onSnapshot((s) => {
      live = true;
      this.applySnapshot(s);
    });
    try {
      const [snap, settings, version, ownPages] = await Promise.all([
        api.snapshot(), api.settings(), api.version(), api.ownPages(),
      ]);
      if (!live) this.snap = snap;
      // Idioma antes de qualquer texto: a tela só aparece com as configurações carregadas.
      await setLocale(this.snap!.locale);
      this.version = version;
      this.ownPages = ownPages;
      this.savedLanguage = settings.language;
      this.settings = settings;
    } catch (e) {
      unlisten();
      throw e;
    }
    this.refreshAssets();
    this.refreshComposition();
    return unlisten;
  }

  private applySnapshot(s: Snapshot) {
    // Seleção nova: esquece a lane de confronto escolhida na anterior.
    if (s.phase === "ChampSelect" && this.snap?.phase !== "ChampSelect") this.vsLaneChoice = null;
    // Idioma trocado (configuração ou cliente aberto em outro idioma): carrega e só então mostra.
    setLocale(s.locale);
    this.snap = s;
    if (s.connected) this.refreshAssets();
    this.refreshComposition();
  }

  /** Dados do cliente (ou a cópia da última conexão): um pedido por vez; sem eles, tenta de novo na próxima conexão. */
  refreshAssets() {
    if (this.assets || this.assetsLoad) return;
    this.assetsLoad = api.assets()
      .then((a) => void (this.assets = a), () => {})
      .finally(() => (this.assetsLoad = null));
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
    // O botão já fica desabilitado; isto segura um segundo clique antes de a tela atualizar.
    if (this.importing) return;
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

  /** Roda `f` depois das gravações anteriores (configurações e páginas próprias): o backend recebe
   *  na ordem dos cliques e a última mudança é a que fica. */
  private enqueue<T>(f: () => Promise<T>): Promise<T> {
    const run = this.queue.then(f);
    this.queue = run.catch(() => {});
    return run;
  }

  /** Grava as configurações (na fila, com o estado da hora em que sai). */
  save(): Promise<void> {
    return this.enqueue(async () => {
      if (!this.settings) return;
      await api.saveSettings($state.snapshot(this.settings));
      // Idioma trocado: o backend também manda o locale novo no snapshot; lendo aqui, a troca
      // aparece logo (e no navegador, onde não há evento).
      if (this.settings.language === this.savedLanguage) return;
      this.savedLanguage = this.settings.language;
      await setLocale((await api.snapshot()).locale);
    });
  }

  /** Mais jogada / Maior WR / uma página sua. A troca já aparece; o backend grava e leva para o cliente. */
  async chooseRunes(choice: RuneChoice) {
    if (!this.settings || choice === this.runeTab) return;
    const ownId = choice.startsWith("own:") ? Number(choice.slice(4)) : null;
    for (const p of this.championPages) p.active = p.id === ownId;
    if (choice === "pick" || choice === "win") this.settings.runeChoice = choice;
    await this.ownCall(() => api.chooseRunes(ownId === null ? (choice as "pick" | "win") : "own", ownId));
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
      this.banError = t("app.banFailed", { error: String(e) });
    } finally {
      this.banBusy = null;
    }
  }

  /** Copia a página atual do cliente como página nova do campeão em tela (com o nome dela no cliente). */
  async copyFromClient() {
    const id = this.snap?.championId;
    if (id) await this.ownCall(() => api.saveOwnRunes(id, null, null));
  }

  openEditor(pageId: number | null) {
    this.ownError = null;
    this.editor = { pageId };
  }

  /** Salva a página montada no editor para o campeão em tela; devolve se deu certo. */
  async saveOwnRunesPage(pageId: number | null, name: string, primaryStyle: number, subStyle: number, perks: number[]): Promise<boolean> {
    const id = this.snap?.championId;
    if (!id) return false;
    await this.ownCall(() => api.saveOwnRunesPage(id, pageId, name, primaryStyle, subStyle, perks));
    return !this.ownError;
  }

  async deleteOwnRunes(championId: number, pageId: number) {
    await this.ownCall(() => api.deleteOwnRunes(championId, pageId));
  }

  /** Comando das páginas próprias, na fila; a lista (e o erro) que vale é a da última chamada. */
  private async ownCall(f: () => Promise<OwnPage[]>) {
    const mine = ++this.ownSeq;
    const last = () => mine === this.ownSeq;
    this.ownBusy = true;
    this.ownError = null;
    try {
      const pages = await this.enqueue(f);
      if (last()) this.ownPages = pages;
    } catch (e) {
      if (!last()) return;
      this.ownError = String(e);
      // Volta ao que o backend tem (a troca otimista pode ter falhado).
      const [pages, settings] = await Promise.all([api.ownPages().catch(() => null), api.settings().catch(() => null)]);
      if (!last()) return;
      if (pages) this.ownPages = pages;
      if (settings) this.settings = settings;
    } finally {
      if (last()) this.ownBusy = false;
    }
  }

  /** WPA ↔ WR: grava e o backend recalcula bans e picks (com o cache, sem requisição nova). */
  setMetric(m: Metric) {
    if (!this.settings || this.settings.metric === m) return;
    this.settings.metric = m;
    this.save();
  }

  async checkUpdateNow() {
    // Dois cliques seguidos: vale a resposta da última consulta.
    const mine = ++this.updateSeq;
    const show = (text: string) => void (mine === this.updateSeq && (this.updateCheck = text));
    show(t("app.updateChecking"));
    try {
      // Procurar à mão mostra o aviso de novo, mesmo se você clicou em "Depois".
      try { localStorage.removeItem("lc-update-later"); } catch { /* sem armazenamento */ }
      const u = await api.checkUpdate();
      show(u ? t("app.updateFound", { version: u.version }) : t("app.updateNone"));
    } catch (e) {
      show(String(e));
    }
  }
}

export const app = new AppState();

// Só em desenvolvimento: `__app` no console do navegador para inspecionar o estado.
if (import.meta.env.DEV) (globalThis as Record<string, unknown>).__app = app;
