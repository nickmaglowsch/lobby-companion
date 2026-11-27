<script lang="ts">
  // Janela principal (`$lib/views/Main.svelte`) ou do overlay na partida, que abre em `/#overlay`
  // (no navegador: `?overlay=1`). Cada janela carrega só o código dela.
  const overlay =
    typeof location !== "undefined" && (location.hash === "#overlay" || new URLSearchParams(location.search).has("overlay"));
  const view = overlay ? import("$lib/overlay/Overlay.svelte") : import("$lib/views/Main.svelte");
</script>

{#await view then m}<m.default />{/await}

<style>
  :global(:root) {
    --bg: #010a13; --panel: #0a1428; --panel-2: #091428; --line: #1e2328;
    --text: #f0e6d2; --muted: #a09b8c; --gold: #c8aa6e; --gold-dim: #785a28;
    --good: #2fbf71; --bad: #e84057; --warn: #e5a93b;
    color-scheme: dark;
  }
  :global(body) { margin: 0; background: var(--bg); color: var(--text); font: 14px/1.4 "Segoe UI", system-ui, sans-serif; }
  :global(input), :global(select) { background: var(--panel); color: var(--text); border: 1px solid var(--gold-dim); border-radius: 4px; padding: 4px 6px; font: inherit; }
  :global(button) { background: var(--panel); color: var(--text); border: 1px solid var(--gold-dim); border-radius: 4px; padding: 4px 10px; font: inherit; cursor: pointer; }
  :global(button:hover:not(:disabled)) { border-color: var(--gold); }
  :global(button:disabled) { opacity: 0.5; cursor: default; }
  :global(button.primary) { background: #1e282d; border-color: var(--gold); color: var(--gold); font-weight: 600; }
  :global(button.link) { border: none; background: none; color: var(--gold); }
  /* Cores de texto usadas em quase todo componente (antes repetidas em cada um). */
  :global(.muted) { color: var(--muted); }
  :global(.up) { color: var(--good); }
  :global(.down) { color: var(--bad); }
</style>
