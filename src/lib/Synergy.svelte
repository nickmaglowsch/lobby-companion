<script lang="ts">
  // Sinergia do seu campeão (escolhido ou declarado) com cada aliado declarado, pelo
  // `ep=build-team` do lolalytics: WR da dupla, o WR geral do seu campeão e a amostra.
  import { championIcon, pct, pts, num, LANES, type Assets, type SynergyAdvice } from "./api";

  let { synergy, assets }: { synergy: SynergyAdvice; assets: Assets | null } = $props();

  const name = (id: number) => assets?.champions[id]?.name ?? `#${id}`;
</script>

<div class="synergy">
  <div class="title muted small">
    <img class="mini me" src={championIcon(synergy.championId)} alt={name(synergy.championId)} />
    Sinergia de {name(synergy.championId)} ({LANES[synergy.lane] ?? synergy.lane}) com o time
  </div>
  {#each synergy.lines as l (l.allyChampionId + l.lane)}
    <div class="line small" title="Além da força geral dos dois: {pts(l.delta)} pts">
      <img class="mini" src={championIcon(l.allyChampionId)} alt={name(l.allyChampionId)} />
      você + {name(l.allyChampionId)} ({LANES[l.lane] ?? l.lane}):
      <b class:up={l.delta >= 1} class:down={l.delta <= -1}>{pct(l.wr)}</b>
      <span class="muted">· geral {synergy.baseWr !== null ? pct(synergy.baseWr) : "?"} · {num(l.n)} partidas</span>
    </div>
  {:else}
    <p class="muted small">Sem partidas suficientes com os aliados declarados.</p>
  {/each}
</div>

<style>
  .synergy { display: grid; gap: 3px; padding-top: 6px; margin-top: 6px; border-top: 1px solid var(--line); }
  .title, .line { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; }
  .mini { width: 18px; height: 18px; border-radius: 3px; vertical-align: middle; border: 1px solid var(--gold-dim); }
  .mini.me { border-color: var(--gold); }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  p { margin: 0; }
  .up { color: var(--good); }
  .down { color: var(--bad); }
</style>
