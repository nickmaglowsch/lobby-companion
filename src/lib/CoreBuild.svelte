<script lang="ts">
  import { coreSets, icon, pct, num, type Assets, type Build } from "./api";

  // Núcleo: os 3 primeiros itens (botas incluídas) na ordem de compra, com WR do conjunto.
  let { build, assets, minGames }: { build: Build; assets: Assets | null; minGames: number } = $props();

  const sets = $derived(coreSets(build, assets, minGames).slice(0, 3));
</script>

{#if sets.length}
  <div class="core">
    <div class="label">Núcleo</div>
    <div class="list">
      {#each sets as { combo, ids }, i (combo.ids.join())}
        <div class="row" class:first={i === 0} title="{num(combo.n)} partidas{i === 0 ? ' · vai para o conjunto de itens' : ''}">
          {#each ids as id, j (j)}
            {#if j > 0}<span class="arrow">›</span>{/if}
            <img src={icon(assets?.items[id]?.icon)} alt={assets?.items[id]?.name} title={assets?.items[id]?.name} />
          {/each}
          <span class="wr" class:up={combo.wr - build.wr >= 1} class:down={combo.wr - build.wr <= -1}>{pct(combo.wr)}</span>
          <span class="muted">{pct(combo.pr)} · {num(combo.n)}</span>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .core { display: grid; grid-template-columns: 64px 1fr; gap: 8px; align-items: start; padding: 6px 0; border-top: 1px solid var(--line); }
  .label { color: var(--muted); font-size: 12px; padding-top: 6px; }
  .list { display: grid; gap: 4px; }
  .row { display: flex; align-items: center; gap: 4px; font-variant-numeric: tabular-nums; font-size: 12px; opacity: 0.8; }
  .row.first { opacity: 1; }
  img { width: 28px; height: 28px; border-radius: 4px; border: 1px solid var(--gold-dim); background: var(--panel-2); }
  .arrow { color: var(--muted); }
  .wr { margin-left: 4px; font-size: 13px; }
  .muted { color: var(--muted); }
  .up { color: var(--good); }
  .down { color: var(--bad); }
</style>
