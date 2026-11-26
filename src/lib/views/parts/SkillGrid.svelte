<script lang="ts">
  // Ordem de habilidades por nível (até o 18) com a prioridade de cima.
  import { pct } from "$lib/api";
  import { app } from "$lib/app.svelte";

  let { cell = 12, numbers = false, priority = true }: { cell?: number; numbers?: boolean; priority?: boolean } = $props();
  const b = $derived(app.build);
  const letters = ["Q", "W", "E", "R"];
</script>

{#if b}
  {#if priority && b.skillPriority[0]}
    <div class="prio">{b.skillPriority[0].ids.join(" › ")} <span class="muted">{pct(b.skillPriority[0].wr)}</span></div>
  {/if}
  {#if b.skillOrder}
    <div class="grid" style="--c: {cell}px">
      {#each letters as k (k)}
        <div class="row">
          <span class="k">{k}</span>
          {#each b.skillOrder.split("") as s, lvl (lvl)}<span class="cell" class:on={s === k}>{numbers && s === k ? lvl + 1 : ""}</span>{/each}
        </div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .prio { font-weight: 600; font-size: 13px; margin-bottom: 4px; }
  .muted { color: var(--muted); font-weight: normal; font-size: 11px; }
  .grid { display: grid; gap: 2px; }
  .row { display: flex; gap: 2px; align-items: center; }
  .k { width: 14px; font-size: 11px; color: var(--gold); font-weight: 600; }
  .cell { width: var(--c); height: var(--c); background: var(--line); border-radius: 2px; font-size: 9px; display: grid; place-items: center; color: var(--bg); }
  .cell.on { background: var(--gold); }
</style>
