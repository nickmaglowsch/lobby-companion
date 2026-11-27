<script lang="ts">
  import { icon, pct, type Assets, type ItemStat } from "./api";
  import { t } from "./i18n/index.svelte";
  import { app } from "./app.svelte";
  import { itemWpa, pp } from "./metric";

  let { label, items, assets, avgWr, minGames, highlight = [], from, slot = true }: {
    label: string;
    items: ItemStat[];
    assets: Assets | null;
    avgWr: number;
    minGames: number;
    /** Itens destacados (defesa contra o dano do time inimigo, `Composition.svelte`). */
    highlight?: number[];
    /** Lista de onde os itens vieram, para o WPA (padrão: `items`). */
    from?: ItemStat[];
    /** Uma linha por partida (posição, botas) ou várias (1ª volta, situacionais). */
    slot?: boolean;
  } = $props();
  const scores = $derived(app.metric === "wpa" ? itemWpa(from ?? items, avgWr, slot) : null);

  const known = $derived(items.filter((i) => assets?.items[i.id]).slice(0, 5));
  const delta = (wr: number) => wr - avgWr;
</script>

{#if known.length}
  <div class="slot">
    <div class="label">{label}</div>
    <div class="items">
      {#each known as it (it.id)}
        {@const item = assets?.items[it.id]}
        {@const sc = scores?.get(it.id)}
        <div class="item" class:low={it.n < minGames} class:def={highlight.includes(it.id)}
          title="{item?.name} · {t('common.games', { count: it.n })}{highlight.includes(it.id) ? ` · ${t('itemRow.defense')}` : ''}">
          <img src={icon(item?.icon)} alt={item?.name} />
          {#if sc !== undefined}
            <span class="wr" class:up={sc >= 0.5} class:down={sc <= -0.5}>{pp(sc)}</span>
          {:else}
            <span class="wr" class:up={delta(it.wr) >= 1} class:down={delta(it.wr) <= -1}>{pct(it.wr)}</span>
          {/if}
          <span class="pr">{pct(it.pr)}</span>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .slot { display: grid; grid-template-columns: 64px 1fr; gap: 8px; align-items: center; padding: 6px 0; border-top: 1px solid var(--line); }
  .label { color: var(--muted); font-size: 12px; }
  .items { display: flex; gap: 6px; flex-wrap: wrap; }
  .item { display: grid; justify-items: center; width: 52px; }
  .item.low { opacity: 0.45; }
  .item.def img { border-color: var(--good); box-shadow: 0 0 0 1px var(--good); }
  img { width: 36px; height: 36px; border: 1px solid var(--gold-dim); border-radius: 4px; background: var(--panel-2); }
  .wr { font-size: 12px; font-variant-numeric: tabular-nums; }
  .pr { font-size: 10px; color: var(--muted); font-variant-numeric: tabular-nums; }
</style>
