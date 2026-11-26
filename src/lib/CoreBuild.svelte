<script lang="ts">
  import { coreSets, icon, pct, num, type Assets, type Build } from "./api";
  import { t } from "./i18n/index.svelte";
  import { app } from "./app.svelte";
  import { comboWpa, pp } from "./metric";

  // Núcleo: os 3 primeiros itens (botas incluídas) na ordem de compra, com WR do conjunto.
  let { build, assets, minGames }: { build: Build; assets: Assets | null; minGames: number } = $props();

  const sets = $derived(coreSets(build, assets, minGames).slice(0, 3));
  // WPA contra o núcleo típico (todos os núcleos guardados, pelo pick rate)
  const wpa = $derived(app.metric === "wpa" ? comboWpa(build.core ?? [], build.wr) : null);
  const scoreOf = (ids: string[]) => {
    const k = (build.core ?? []).findIndex((c) => c.ids.join() === ids.join());
    return wpa && k >= 0 ? wpa[k] : undefined;
  };
</script>

{#if sets.length}
  <div class="core">
    <div class="label">{t("common.core")}</div>
    <div class="list">
      {#each sets as { combo, ids }, i (combo.ids.join())}
        {@const sc = scoreOf(combo.ids)}
        <div class="row" class:first={i === 0} title="{t('common.games', { count: combo.n })}{i === 0 ? ` · ${t('coreBuild.toItemSet')}` : ''}">
          {#each ids as id, j (j)}
            {#if j > 0}<span class="arrow">›</span>{/if}
            <img src={icon(assets?.items[id]?.icon)} alt={assets?.items[id]?.name} title={assets?.items[id]?.name} />
          {/each}
          {#if sc !== undefined}
            <span class="wr" class:up={sc >= 0.5} class:down={sc <= -0.5} title="WR {pct(combo.wr)}">{t("metric.pointsPp", { value: pp(sc) })}</span>
          {:else}
            <span class="wr" class:up={combo.wr - build.wr >= 1} class:down={combo.wr - build.wr <= -1}>{pct(combo.wr)}</span>
          {/if}
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
  .arrow:dir(rtl) { display: inline-block; transform: scaleX(-1); }
  .wr { margin-inline-start: 4px; font-size: 13px; }
  .muted { color: var(--muted); }
  .up { color: var(--good); }
  .down { color: var(--bad); }
</style>
