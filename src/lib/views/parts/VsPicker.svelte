<script lang="ts">
  // Confronto: atalhos dos inimigos visíveis (+ busca e lane no modo completo) e o resumo do WR contra ele.
  import { championIcon, pct, num, LANES } from "$lib/api";
  import { app } from "$lib/app.svelte";

  let { full = false, size = 24 }: { full?: boolean; size?: number } = $props();
  let text = $state("");
  const b = $derived(app.build);
  const id = `vs-${Math.random().toString(36).slice(2, 8)}`;
</script>

<div class="vs" class:full style="--s: {size}px">
  <div class="icons">
    {#if !full}<span class="muted small">Contra:</span>{/if}
    {#each app.snap?.enemyPicks ?? [] as e (e)}
      <button class="pick" class:on={e === app.snap?.vsChampionId} onclick={() => app.chooseVs(e === app.snap?.vsChampionId ? null : e)}
        title={e === app.snap?.vsChampionId ? "Voltar ao build geral" : `Build contra ${app.champName(e)}`}>
        <img src={championIcon(e)} alt={app.champName(e)} />
        {#if full}<span>{app.champName(e)}</span>{/if}
      </button>
    {/each}
    {#if !full && !app.snap?.enemyPicks.length}<span class="muted small">nenhum inimigo visível</span>{/if}
  </div>
  {#if full}
    <form onsubmit={(e) => { e.preventDefault(); const c = app.findChampion(text); if (c) { app.chooseVs(c.id); text = ""; } }}>
      <input list={id} placeholder="Outro campeão…" bind:value={text} disabled={!app.assets} />
      <datalist {id}>{#each app.champions as c (c.id)}<option value={c.name}></option>{/each}</datalist>
      <select value={app.vsLane ?? ""} onchange={(e) => app.changeVsLane(e.currentTarget.value)} title="Lane do campeão do outro lado">
        {#each Object.entries(LANES) as [v, l] (v)}<option value={v}>{l}</option>{/each}
      </select>
    </form>
  {/if}
  {#if b?.vs}
    <p class="line">
      Contra <b>{app.champName(b.vs.championId)}</b> ({LANES[b.vs.lane] ?? b.vs.lane}):
      <b class:up={b.wr - b.vs.usualWr >= 1} class:down={b.wr - b.vs.usualWr <= -1}>{pct(b.wr)}</b>
      <span class="muted">· geral {pct(b.vs.usualWr)} · {num(b.n)} partidas</span>
      <button class="link" onclick={() => app.chooseVs(null)}>build geral</button>
    </p>
  {:else if full}
    <p class="muted small">Escolha o campeão do outro lado: runas, feitiços e itens passam a ser desse confronto.</p>
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
