<script lang="ts">
  // Janela do app: escolhe o layout pela configuração (Por fase, Grade, Abas ou Clássico).
  // O estado e as ações ficam em `$lib/app.svelte.ts`, compartilhados pelos layouts.
  import { onMount } from "svelte";
  import { app } from "$lib/app.svelte";
  import UpdateBanner from "$lib/UpdateBanner.svelte";
  import Classic from "$lib/views/Classic.svelte";
  import Grid from "$lib/views/Grid.svelte";
  import Tabs from "$lib/views/Tabs.svelte";
  import Phase from "$lib/views/Phase.svelte";
  import SettingsDialog from "$lib/views/parts/SettingsDialog.svelte";
  import RuneEditor from "$lib/views/parts/RuneEditor.svelte";

  onMount(() => {
    let un: (() => void) | undefined;
    app.init().then((u) => (un = u));
    return () => un?.();
  });
</script>

{#if app.settings}
  {#if app.layout === "classic"}
    <div class="classic-banner"><UpdateBanner inChampSelect={app.snap?.inChampSelect ?? false} /></div>
    <Classic />
  {:else}
    {#if app.layout === "grid"}<Grid />{:else if app.layout === "tabs"}<Tabs />{:else}<Phase />{/if}
    <UpdateBanner inChampSelect={app.snap?.inChampSelect ?? false} floating />
    <SettingsDialog />
  {/if}
  <RuneEditor />
{/if}

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
  .classic-banner { padding: 10px 12px 0; }
  .classic-banner:empty { display: none; }
</style>
