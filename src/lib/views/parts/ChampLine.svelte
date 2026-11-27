<script lang="ts">
  // Campeão em tela numa linha: retrato, nome, lane (troca de lane no menu) e WR/rank.
  import { championIcon, pct, laneName } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";

  const b = $derived(app.build);
  const lanes = $derived(app.lanes);
</script>

{#if b && app.snap}
  <span class="cl">
    <img src={championIcon(app.snap.championId)} alt="" />
    <span class="nm">{app.champ?.name ?? b.slug}</span>
    {#if lanes.length > 1}
      <select class="lane" value={b.lane} onchange={(e) => app.changeLane(e.currentTarget.value)} title={t("champLine.changeLane")}>
        {#each lanes as [lane, p] (lane)}<option value={lane}>{t("champLine.laneOption", { lane: laneName(lane), p: Math.round(p) })}</option>{/each}
      </select>
    {:else}
      <span class="chip">{laneName(b.lane)}</span>
    {/if}
    <span class="muted">
      WR <b class="txt">{pct(b.wr)}</b>{#if b.rank} · #{b.rank}/{b.rankTotal}{/if}
      · {b.patch}
    </span>
  </span>
{/if}

<style>
  .cl { display: flex; align-items: center; gap: 7px; min-width: 0; font-size: 13px; }
  img { width: 26px; height: 26px; border: 1px solid var(--gold); border-radius: 4px; }
  .nm { font-weight: 600; white-space: nowrap; }
  .chip { font-size: 11px; padding: 1px 6px; border: 1px solid var(--gold-dim); border-radius: 10px; color: var(--gold); }
  .lane { font-size: 11px; padding: 1px 4px; color: var(--gold); }
  .muted { color: var(--muted); font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .txt { color: var(--text); }
</style>
