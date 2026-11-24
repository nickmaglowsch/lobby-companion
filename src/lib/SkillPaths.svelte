<script lang="ts">
  import { pct, num, type Build, type Combo } from "./api";

  // Habilidades até o nível 6 e até o 10: a mais jogada e, se for outra, a de maior WR com amostra.
  let { build, minGames }: { build: Build; minGames: number } = $props();

  function rows(list: Combo[]): { top: Combo; best: Combo | null } | null {
    const top = list[0];
    if (!top) return null;
    const best = list.filter((c) => c.n >= minGames).reduce<Combo | null>((a, c) => (!a || c.wr > a.wr ? c : a), null);
    return { top, best: best && best !== top && best.wr - top.wr >= 1 ? best : null };
  }

  const lines = $derived(
    [
      ["Até o 6", rows(build.skill6 ?? [])],
      ["Até o 10", rows(build.skill10 ?? [])],
    ].filter((l): l is [string, { top: Combo; best: Combo | null }] => l[1] !== null),
  );
</script>

{#if lines.length}
  <div class="paths">
    {#each lines as [label, r] (label)}
      <div class="line">
        <span class="label">{label}</span>
        <span class="seq">{#each r.top.ids as k, i (i)}<span class="k k{k}">{k}</span>{/each}</span>
        <span class="wr">{pct(r.top.wr)}</span>
        <span class="muted">{num(r.top.n)}</span>
      </div>
      {#if r.best}
        <div class="line alt" title="Maior WR com pelo menos {minGames} partidas">
          <span class="label">maior WR</span>
          <span class="seq">{#each r.best.ids as k, i (i)}<span class="k k{k}">{k}</span>{/each}</span>
          <span class="wr up">{pct(r.best.wr)}</span>
          <span class="muted">{num(r.best.n)}</span>
        </div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .paths { display: grid; gap: 3px; margin-top: 8px; font-size: 12px; }
  .line { display: flex; align-items: center; gap: 6px; font-variant-numeric: tabular-nums; }
  .line.alt { opacity: 0.85; }
  .label { width: 58px; color: var(--muted); }
  .seq { display: flex; gap: 1px; }
  .k { width: 14px; height: 15px; display: grid; place-items: center; font-size: 10px; font-weight: 600; border-radius: 2px; background: var(--line); }
  .kR { background: var(--gold); color: var(--bg); }
  .muted { color: var(--muted); }
  .up { color: var(--good); }
</style>
