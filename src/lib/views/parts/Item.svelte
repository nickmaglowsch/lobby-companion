<script lang="ts">
  // Um item com o WR embaixo: verde/vermelho = ≥1 ponto acima/abaixo do WR do campeão;
  // apagado = amostra pequena; contorno verde = defesa contra o dano do time inimigo.
  import { icon, pct, num, type ItemStat } from "$lib/api";
  import { app } from "$lib/app.svelte";

  let { it, size = 30 }: { it: ItemStat; size?: number } = $props();
  const item = $derived(app.assets?.items[it.id]);
  const base = $derived(app.build?.wr ?? 50);
  const def = $derived(app.defHighlight.includes(it.id));
</script>

<div class="it" class:low={it.n < app.minGames} class:def style="--s: {size}px"
  title="{item?.name} · {pct(it.wr)} · pick {pct(it.pr)} · {num(it.n)} partidas{def ? ' · defesa contra o time inimigo' : ''}">
  <img src={icon(item?.icon)} alt={item?.name} />
  <span class:up={it.wr - base >= 1} class:down={it.wr - base <= -1}>{pct(it.wr)}</span>
</div>

<style>
  .it { display: grid; justify-items: center; font-size: 10px; font-variant-numeric: tabular-nums; }
  .it.low { opacity: 0.45; }
  img { width: var(--s); height: var(--s); border-radius: 4px; border: 1px solid var(--gold-dim); background: var(--panel-2); }
  .def img { border-color: var(--good); box-shadow: 0 0 0 1px var(--good); }
  .up { color: var(--good); }
  .down { color: var(--bad); }
</style>
