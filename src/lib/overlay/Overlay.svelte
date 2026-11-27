<script lang="ts">
  // Janela do overlay na partida (transparente, por cima do jogo, deixa o clique passar).
  // Só desenha o `OverlayFrame` que o Rust calcula (`overlay.rs`): posições em pixels físicos da
  // área do jogo, convertidas aqui para a janela (`r`). No modo de ajuste, o CS/min e o próximo item
  // se arrastam e a posição é salva em fração da tela.
  import { onMount } from "svelte";
  import { api, icon, type Assets, type OverlayFrame } from "$lib/api";
  import { fixed, num, setLocale, t } from "$lib/i18n/index.svelte";

  type Widget = "cs" | "item";
  const DEFAULT_POS: Record<Widget, [number, number]> = { cs: [0.885, 0.055], item: [0.6, 0.8] };

  let frame = $state<OverlayFrame | null>(null);
  let tab = $state(false);
  let assets = $state<Assets | null>(null);
  let vw = $state(typeof window === "undefined" ? 1920 : window.innerWidth);
  /** Posições arrastadas nesta sessão (antes de o próximo quadro trazer as salvas). */
  let moved = $state<Partial<Record<Widget, [number, number]>>>({});
  let drag: { widget: Widget; dx: number; dy: number } | null = null;

  onMount(() => {
    document.documentElement.classList.add("overlay");
    if (!("__TAURI_INTERNALS__" in window)) {
      // Navegador (`npm run dev`): placar "segurado", a não ser com &tab=0.
      document.documentElement.classList.add("overlay-dev");
      tab = new URLSearchParams(location.search).get("tab") !== "0";
    }
    api.snapshot().then((s) => setLocale(s.locale)).catch(() => {});
    api.assets().then((a) => (assets = a)).catch(() => {});
    api.overlayFrame().then((f) => f && (frame = f)).catch(() => {});
    const uns = [api.onOverlay((f) => (frame = f)), api.onOverlayTab((d) => (tab = d))];
    const resize = () => (vw = window.innerWidth);
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      uns.forEach((u) => u.then((f) => f()));
    };
  });

  /** Pixels da janela por pixel do jogo. */
  const r = $derived(frame && frame.width > 0 ? vw / frame.width : 1);
  /** Tamanho base das janelinhas, acompanhando a escala do HUD do jogo. */
  const k = $derived(frame ? Math.max(frame.scale, 0.5) * r : 1);
  const s = $derived(frame?.settings);
  const editing = $derived(!!frame && (frame.edit || frame.preview));
  const showTab = $derived(!!frame && (tab || editing));

  function pos(w: Widget): [number, number] {
    const saved = w === "cs" ? s?.csPos : s?.itemPos;
    return moved[w] ?? saved ?? DEFAULT_POS[w];
  }

  const mmss = (sec: number) => {
    const x = Math.max(0, Math.ceil(sec));
    return `${Math.floor(x / 60)}:${String(x % 60).padStart(2, "0")}`;
  };

  function down(e: PointerEvent, w: Widget) {
    if (!editing) return;
    const el = e.currentTarget as HTMLElement;
    const box = el.getBoundingClientRect();
    drag = { widget: w, dx: e.clientX - box.left, dy: e.clientY - box.top };
    el.setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag) return;
    const x = (e.clientX - drag.dx) / window.innerWidth;
    const y = (e.clientY - drag.dy) / window.innerHeight;
    moved = { ...moved, [drag.widget]: [clamp(x), clamp(y)] };
  }
  function up() {
    if (!drag) return;
    const [x, y] = pos(drag.widget);
    api.saveOverlayPos(drag.widget, x, y);
    drag = null;
  }
  const clamp = (v: number) => Math.min(0.98, Math.max(0, v));

  /** Caminho do gráfico: x pelo tempo (0 → agora), y simétrico pelo maior valor. */
  function graphPaths(points: [number, number][], w: number, h: number) {
    const end = Math.max(60, points.at(-1)?.[0] ?? 60);
    const top = Math.max(1000, ...points.map(([, v]) => Math.abs(v)));
    const pad = 6;
    const X = (t: number) => (t / end) * w;
    const Y = (v: number) => h / 2 - (v / top) * (h / 2 - pad);
    const line = points.map(([t, v], i) => `${i ? "L" : "M"}${X(t).toFixed(1)},${Y(v).toFixed(1)}`).join(" ");
    const area = points.length ? `${line} L${X(points.at(-1)![0]).toFixed(1)},${h / 2} L${X(points[0][0]).toFixed(1)},${h / 2} Z` : "";
    return { line, area, top, mid: h / 2 };
  }
</script>

<svelte:window onpointermove={move} onpointerup={up} />

{#if frame && frame.visible && s && (s.enabled || editing)}
  {@const f = frame}
  <div class="root" style:--k={k}>
    {#if editing}
      <!-- Onde o app acha que o jogo desenha o minimapa e o placar (para conferir o cálculo). -->
      {#if f.minimap}<div class="guide" style:left="{f.minimap.x * r}px" style:top="{f.minimap.y * r}px" style:width="{f.minimap.w * r}px" style:height="{f.minimap.h * r}px"></div>{/if}
      {#if f.scoreboard && showTab}<div class="guide" style:left="{f.scoreboard.x * r}px" style:top="{f.scoreboard.y * r}px" style:width="{f.scoreboard.w * r}px" style:height="{f.scoreboard.h * r}px"></div>{/if}
      <div class="edit-bar">
        <span>{t("overlay.editHint")}</span>
        <button onclick={() => api.overlayEdit(false)}>{t("overlay.done")}</button>
      </div>
    {/if}

    {#if s.inhibitors}
      {#each f.inhibitors as inh, i (i)}
        <div class="inhib" class:ally={inh.ally} style:left="{inh.x * r}px" style:top="{inh.y * r}px">{mmss(inh.seconds)}</div>
      {/each}
    {/if}

    {#if s.csPerMin && f.cs}
      {@const [x, y] = pos("cs")}
      <div class="widget cs" class:movable={editing} style:left="{x * 100}%" style:top="{y * 100}%" role="presentation" onpointerdown={(e) => down(e, "cs")}>
        <b>{fixed(f.cs.perMin, 1)}</b> <span>{t("overlay.csPerMin")}</span>
        <span class="muted">({num(f.cs.total)})</span>
      </div>
    {/if}

    {#if s.nextItem && f.nextItem}
      {@const [x, y] = pos("item")}
      {@const it = assets?.items[String(f.nextItem.id)]}
      <div class="widget item" class:movable={editing} style:left="{x * 100}%" style:top="{y * 100}%" role="presentation" onpointerdown={(e) => down(e, "item")}>
        {#if it?.icon}<img src={icon(it.icon)} alt="" />{/if}
        <span class="txt">
          <span class="label">{t("overlay.buy")}</span>
          <span class="name">{it?.name ?? `#${f.nextItem.id}`}</span>
        </span>
        <span class="gold">{num(f.nextItem.cost)}</span>
      </div>
    {/if}

    {#if showTab && s.goldDiff}
      {#each f.lanes as lane, i (i)}
        <div class="lane" class:ally={lane.allyAhead} style:left="{lane.x * r}px" style:top="{lane.y * r}px">
          {#if lane.allyAhead}<span class="arrow">◀</span>{/if}
          <span>{num(lane.gold)}</span>
          {#if !lane.allyAhead}<span class="arrow">▶</span>{/if}
        </div>
      {/each}
    {/if}

    {#if showTab && s.goldGraph && f.graph}
      {@const g = f.graph}
      {@const w = g.rect.w * r}
      {@const h = g.rect.h * r}
      {@const p = graphPaths(g.points, w, h)}
      <div class="graph" style:left="{g.rect.x * r}px" style:top="{g.rect.y * r}px" style:width="{w}px" style:height="{h}px">
        <svg width={w} height={h} viewBox="0 0 {w} {h}" aria-hidden="true">
          <defs>
            <clipPath id="above"><rect x="0" y="0" width={w} height={p.mid} /></clipPath>
            <clipPath id="below"><rect x="0" y={p.mid} width={w} height={h - p.mid} /></clipPath>
          </defs>
          <line x1="0" x2={w} y1={p.mid} y2={p.mid} class="zero" />
          <path d={p.area} class="area ally" clip-path="url(#above)" />
          <path d={p.area} class="area enemy" clip-path="url(#below)" />
          <path d={p.line} class="line" />
        </svg>
        <span class="g-title">{t("overlay.graph")}</span>
        <span class="g-top">+{num(Math.round(p.top))}</span>
        <span class="g-bottom">−{num(Math.round(p.top))}</span>
      </div>
    {/if}
  </div>
{/if}

<style>
  :global(html.overlay), :global(html.overlay body) { background: transparent !important; overflow: hidden; }
  :global(html.overlay-dev body) { background: #2a2f36 !important; }
  .root { position: fixed; inset: 0; pointer-events: none; font: 600 calc(13px * var(--k)) / 1.2 "Segoe UI", system-ui, sans-serif; color: #f0e6d2; }
  .guide { position: absolute; border: 1px dashed rgba(200, 170, 110, 0.7); pointer-events: none; }
  .edit-bar {
    position: absolute; top: 12px; left: 50%; transform: translateX(-50%); display: flex; gap: 10px; align-items: center;
    background: rgba(1, 10, 19, 0.92); border: 1px solid #c8aa6e; border-radius: 6px; padding: 6px 10px;
    font: 13px/1.3 "Segoe UI", system-ui, sans-serif; pointer-events: auto; max-width: 90vw;
  }
  .edit-bar button { pointer-events: auto; }
  .inhib {
    position: absolute; transform: translate(-50%, -50%); padding: 0 calc(3px * var(--k)); border-radius: 3px;
    background: rgba(0, 0, 0, 0.75); color: #7fc4ff; font-size: calc(12px * var(--k)); text-shadow: 0 0 2px #000;
  }
  .inhib.ally { color: #ff6b7d; }
  .widget {
    position: absolute; display: flex; align-items: center; gap: calc(6px * var(--k)); white-space: nowrap;
    background: rgba(1, 10, 19, 0.78); border: 1px solid rgba(200, 170, 110, 0.55); border-radius: 4px;
    padding: calc(3px * var(--k)) calc(7px * var(--k)); text-shadow: 0 0 2px #000;
  }
  .widget.movable { pointer-events: auto; cursor: grab; outline: 1px dashed #c8aa6e; }
  .cs b { color: #c8aa6e; font-size: calc(15px * var(--k)); }
  .muted { color: #a09b8c; font-weight: 400; }
  .item { border-color: #2fbf71; box-shadow: 0 0 calc(8px * var(--k)) rgba(47, 191, 113, 0.5); }
  .item img { width: calc(30px * var(--k)); height: calc(30px * var(--k)); border-radius: 3px; }
  .item .txt { display: grid; }
  .item .label { color: #2fbf71; font-size: calc(10px * var(--k)); text-transform: uppercase; }
  .item .gold { color: #e5c76b; }
  .lane {
    position: absolute; transform: translate(-50%, -50%); display: flex; gap: calc(2px * var(--k)); align-items: center;
    color: #ff6b7d; font-size: calc(14px * var(--k)); text-shadow: 0 0 3px #000, 0 0 2px #000;
    background: rgba(0, 0, 0, 0.55); border-radius: 3px; padding: 0 calc(3px * var(--k));
  }
  .lane.ally { color: #5cb8ff; }
  .arrow { font-size: calc(11px * var(--k)); }
  .graph { position: absolute; background: rgba(1, 10, 19, 0.85); border: 1px solid rgba(200, 170, 110, 0.45); border-radius: 6px; }
  .graph svg { display: block; }
  .zero { stroke: rgba(160, 155, 140, 0.6); stroke-dasharray: 4 4; }
  .area.ally { fill: rgba(92, 184, 255, 0.25); }
  .area.enemy { fill: rgba(255, 107, 125, 0.25); }
  .line { fill: none; stroke: #c8aa6e; stroke-width: 2; }
  .g-title { position: absolute; left: 8px; top: 4px; font-size: calc(11px * var(--k)); color: #c8aa6e; text-transform: uppercase; }
  .g-top, .g-bottom { position: absolute; right: 6px; font-size: calc(10px * var(--k)); color: #a09b8c; font-weight: 400; }
  .g-top { top: 4px; }
  .g-bottom { bottom: 4px; }
</style>
