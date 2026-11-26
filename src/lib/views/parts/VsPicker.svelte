<script lang="ts">
  // Confronto: atalhos dos inimigos visíveis (+ busca e lane no modo completo) e o resumo do WR contra ele.
  import { championIcon, pct, LANES } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";

  let { full = false, size = 24 }: { full?: boolean; size?: number } = $props();
  let text = $state("");
  const b = $derived(app.build);
  const id = `vs-${Math.random().toString(36).slice(2, 8)}`;
</script>

<div class="vs" class:full style="--s: {size}px">
  <div class="icons">
    {#if !full}<span class="muted small">{t("vsPicker.against")}</span>{/if}
    {#each app.snap?.enemyPicks ?? [] as e (e)}
      <button class="pick" class:on={e === app.snap?.vsChampionId} onclick={() => app.chooseVs(e === app.snap?.vsChampionId ? null : e)}
        title={e === app.snap?.vsChampionId ? t("vsPicker.backToGeneral") : t("vsPicker.buildVs", { name: app.champName(e) })}>
        <img src={championIcon(e)} alt={app.champName(e)} />
        {#if full}<span>{app.champName(e)}</span>{/if}
      </button>
    {/each}
    {#if !full && !app.snap?.enemyPicks.length}<span class="muted small">{t("vsPicker.noEnemy")}</span>{/if}
  </div>
  {#if full}
    <form onsubmit={(e) => { e.preventDefault(); const c = app.findChampion(text); if (c) { app.chooseVs(c.id); text = ""; } }}>
      <input list={id} placeholder={t("vsPicker.otherChampion")} bind:value={text} disabled={!app.assets} />
      <datalist {id}>{#each app.champions as c (c.id)}<option value={c.name}></option>{/each}</datalist>
      <select value={app.vsLane ?? ""} onchange={(e) => app.changeVsLane(e.currentTarget.value)} title={t("vsPicker.laneTip")}>
        {#each Object.entries(LANES) as [v, l] (v)}<option value={v}>{l}</option>{/each}
      </select>
    </form>
  {/if}
  {#if b?.vs}
    {@const head = t("vsPicker.vsHead", { lane: LANES[b.vs.lane] ?? b.vs.lane }).split("{name}")}
    <!-- O nome fica em negrito: o texto traduzido é cortado no "{name}". -->
    <p class="line">
      <span>{head[0]}<b>{app.champName(b.vs.championId)}</b>{head.slice(1).join("")}</span>
      <b class:up={b.wr - b.vs.usualWr >= 1} class:down={b.wr - b.vs.usualWr <= -1}>{pct(b.wr)}</b>
      <span class="muted">{t("vsPicker.vsRest", { base: pct(b.vs.usualWr), count: b.n })}</span>
      <button class="link" onclick={() => app.chooseVs(null)}>{t("vsPicker.general")}</button>
    </p>
  {:else if full}
    <p class="muted small">{t("vsPicker.hint")}</p>
  {/if}
</div>

<style>
  .vs { display: grid; gap: 6px; }
  .icons { display: flex; gap: 4px; align-items: center; flex-wrap: wrap; }
  .pick { padding: 0; line-height: 0; border: 1px solid transparent; background: none; border-radius: 4px; }
  .pick img { width: var(--s); height: var(--s); border-radius: 4px; }
  .pick.on { border-color: var(--gold); box-shadow: 0 0 0 1px var(--gold); }
  .full .pick { display: grid; justify-items: center; gap: 2px; padding: 6px; width: 76px; line-height: 1.2; border-color: var(--gold-dim); background: var(--panel); }
  .full .pick span { font-size: 11px; }
  form { display: flex; gap: 6px; }
  form input { flex: 1; min-width: 0; }
  .line { margin: 0; font-size: 12px; display: flex; gap: 4px; align-items: center; flex-wrap: wrap; }
  .up { color: var(--good); }
  .down { color: var(--bad); }
  .muted { color: var(--muted); }
  .small { font-size: 11px; }
  .link { font-size: 11px; padding: 0 4px; }
</style>
