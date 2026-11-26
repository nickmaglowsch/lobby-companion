<script lang="ts">
  import { icon, pct, num, pts, type Assets, type PerkStat, type RuneSet, type RuneStats } from "./api";

  // WR de cada runa escolhida e das alternativas da mesma fileira. Só exibição: não muda a importação.
  let { set, stats, assets, minGames, open = false }: {
    set: RuneSet;
    stats: RuneStats | null;
    assets: Assets | null;
    minGames: number;
    /** Já aberto (aba Runas do layout Abas). */
    open?: boolean;
  } = $props();

  /** Alternativa "claramente melhor": ≥ 2 pontos de WR acima, com amostra mínima. */
  const BETTER = 2;

  type Alt = { id: number; stat: PerkStat | undefined; better: boolean };
  type Row = { id: number; stat: PerkStat | undefined; alts: Alt[] };

  const perk = (id: number) => assets?.perks[id];

  function row(chosen: number, slot: number[] | undefined, source: Record<string, PerkStat> | undefined): Row | null {
    if (!slot || !source) return null;
    const stat = source[chosen];
    const alts = slot
      .filter((id) => id !== chosen)
      .map((id) => {
        const s = source[id];
        const better = !!s && !!stat && s.n >= minGames && s.wr - stat.wr >= BETTER;
        return { id, stat: s, better };
      })
      .filter((a) => a.stat && a.stat.n > 0)
      .sort((a, b) => (b.stat?.pr ?? 0) - (a.stat?.pr ?? 0));
    return stat || alts.length ? { id: chosen, stat, alts } : null;
  }

  const rows = $derived.by(() => {
    if (!assets || !stats) return [];
    const primary = assets.styles.find((s) => s.perks.includes(set.primary[0]));
    const secondary = assets.styles.find((s) => s.id !== primary?.id && set.secondary.every((p) => s.perks.includes(p)));
    const slotOf = (slots: number[][] | undefined, id: number) => slots?.find((r) => r.includes(id));
    const out: (Row | null)[] = [
      ...set.primary.map((id) => row(id, slotOf(primary?.slots, id), stats.primary)),
      ...set.secondary.map((id) => row(id, slotOf(secondary?.slots, id), stats.secondary)),
      // Fragmentos: fileira do meio tem estatística própria (`flex`); ataque e defesa ficam em `primary`.
      ...set.shards.map((id, i) => row(id, assets.shardSlots?.[i], i === 1 ? stats.flex : stats.primary)),
    ];
    return out.filter((r): r is Row => r !== null);
  });

  const hint = $derived(
    rows.flatMap((r) => r.alts.filter((a) => a.better).map((a) => ({ from: r, to: a }))),
  );
</script>

{#if rows.length}
  <details class="alts" {open}>
    <summary>WR de cada runa e alternativas{#if hint.length}<span class="flag">&nbsp;· {hint.length} com WR maior</span>{/if}</summary>
    <div class="grid">
      {#each rows as r, i (i)}
        <div class="row">
          <div class="chosen" title="{perk(r.id)?.name}{r.stat ? ` · ${num(r.stat.n)} partidas` : ''}">
            <img src={icon(perk(r.id)?.icon)} alt={perk(r.id)?.name} />
            <span class="wr">{r.stat ? pct(r.stat.wr) : "—"}</span>
            <span class="pr">{r.stat ? pct(r.stat.pr) : ""}</span>
          </div>
          <div class="others">
            {#each r.alts as a (a.id)}
              <div class="alt" class:better={a.better} class:low={(a.stat?.n ?? 0) < minGames}
                title="{perk(a.id)?.name} · {num(a.stat?.n ?? 0)} partidas{a.better ? ' · WR claramente maior' : ''}">
                <img src={icon(perk(a.id)?.icon)} alt={perk(a.id)?.name} />
                <span class="wr">{pct(a.stat?.wr ?? 0)}</span>
                <span class="pr">{pct(a.stat?.pr ?? 0)}</span>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
    {#each hint as h (h.to.id)}
      <p class="small better-line">
        {perk(h.to.id)?.name ?? h.to.id} no lugar de {perk(h.from.id)?.name ?? h.from.id}:
        {pts((h.to.stat?.wr ?? 0) - (h.from.stat?.wr ?? 0), 1)} pts ({num(h.to.stat?.n ?? 0)} partidas)
      </p>
    {/each}
    <p class="small muted">WR · PR de cada runa nesse lugar da página. Apagado = menos de {minGames} partidas. Só informação: a importação segue a aba escolhida.</p>
  </details>
{/if}

<style>
  .alts { margin-top: 8px; font-size: 12px; }
  summary { cursor: pointer; color: var(--muted); }
  .flag { color: var(--good); }
  .grid { display: grid; gap: 4px; margin-top: 6px; }
  .row { display: grid; grid-template-columns: 52px 1fr; gap: 8px; align-items: center; border-top: 1px solid var(--line); padding-top: 4px; }
  .others { display: flex; gap: 4px; flex-wrap: wrap; }
  .chosen, .alt { display: grid; justify-items: center; width: 48px; }
  .alt { opacity: 0.85; }
  .alt.low { opacity: 0.4; }
  .alt.better { opacity: 1; outline: 1px solid var(--good); border-radius: 4px; }
  .alt.better .wr { color: var(--good); }
  img { width: 24px; height: 24px; border-radius: 50%; background: var(--panel-2); }
  .chosen img { width: 28px; height: 28px; border: 1px solid var(--gold); }
  .wr { font-variant-numeric: tabular-nums; }
  .pr { font-size: 10px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .better-line { color: var(--good); margin: 4px 0 0; }
  .small { font-size: 12px; }
  .muted { color: var(--muted); }
</style>
