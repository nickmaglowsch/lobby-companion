<script lang="ts">
  // Troca a métrica das sugestões (WPA ↔ WR) direto na seleção; grava nas configurações e o
  // backend recalcula bans e picks com o que já está em cache.
  import { app } from "$lib/app.svelte";
  import { METRICS } from "$lib/metric";
</script>

{#if app.settings}
  <span class="toggle" role="group" aria-label="Métrica das sugestões">
    {#each METRICS as [v, label, help] (v)}
      <button class:on={app.metric === v} title={help} aria-pressed={app.metric === v} onclick={() => app.setMetric(v)}>{label}</button>
    {/each}
  </span>
{/if}

<style>
  .toggle { display: inline-flex; vertical-align: middle; border: 1px solid var(--line); border-radius: 4px; overflow: hidden; margin-left: 6px; }
  button { font: inherit; font-size: 10px; font-weight: 600; letter-spacing: 0.03em; line-height: 1; padding: 3px 6px; border: none; border-radius: 0; background: none; color: var(--muted); cursor: pointer; }
  button + button { border-left: 1px solid var(--line); }
  button:hover { color: var(--gold); }
  button.on { background: var(--gold); color: var(--bg); }
</style>
