<script lang="ts">
  // Sinergia do seu campeão (escolhido ou declarado) com cada aliado declarado, pelo
  // `ep=build-team` do lolalytics: WR da dupla, o WR geral do seu campeão e a amostra.
  import { championIcon, pct, pts, laneName, type Assets, type SynergyAdvice } from "./api";
  import { t } from "./i18n/index.svelte";

  let { synergy, assets }: { synergy: SynergyAdvice; assets: Assets | null } = $props();

  const name = (id: number) => assets?.champions[id]?.name ?? `#${id}`;
  // Frase inteira traduzida; o WR (em destaque) entra no lugar de `{wr}`.
  function pairLine(allyId: number, lane: string, n: number): [string, string] {
    const s = t("synergy.pair", {
      name: name(allyId),
      lane: laneName(lane),
      base: synergy.baseWr !== null ? pct(synergy.baseWr) : "?",
      count: n,
    });
    const at = s.indexOf("{wr}");
    return at < 0 ? [s, ""] : [s.slice(0, at), s.slice(at + 4)];
  }
</script>

<div class="synergy">
  <div class="title muted small">
    <img class="mini me" src={championIcon(synergy.championId)} alt={name(synergy.championId)} />
    {t("synergy.title", { name: name(synergy.championId), lane: laneName(synergy.lane) })}
  </div>
  {#each synergy.lines as l (l.allyChampionId + l.lane)}
    {@const [before, after] = pairLine(l.allyChampionId, l.lane, l.n)}
    <div class="line small" title={t("synergy.lineTitle", { delta: pts(l.delta) })}>
      <img class="mini" src={championIcon(l.allyChampionId)} alt={name(l.allyChampionId)} />
      {before}
      <b class:up={l.delta >= 1} class:down={l.delta <= -1}>{pct(l.wr)}</b>
      <span class="muted">{after}</span>
    </div>
  {:else}
    <p class="muted small">{t("synergy.empty")}</p>
  {/each}
</div>

<style>
  .synergy { display: grid; gap: 3px; padding-top: 6px; margin-top: 6px; border-top: 1px solid var(--line); }
  .title, .line { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; }
  .mini { width: 18px; height: 18px; border-radius: 3px; vertical-align: middle; border: 1px solid var(--gold-dim); }
  .mini.me { border-color: var(--gold); }
  .small { font-size: 12px; }
  p { margin: 0; }
</style>
