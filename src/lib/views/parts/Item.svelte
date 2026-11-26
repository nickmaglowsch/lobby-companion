<script lang="ts">
  // Um item com o WR (ou o WPA) embaixo: verde/vermelho = ≥1 ponto acima/abaixo do WR do campeão
  // (WPA: ≥0,5 pp acima/abaixo do típico); apagado = amostra pequena; contorno verde = defesa
  // contra o dano do time inimigo. `from` = a lista de onde ele veio (para o WPA); `slot` = uma
  // linha por partida (posição, botas) ou várias (1ª volta, situacionais).
  import { icon, pct, num, type ItemStat } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { itemWpa, pp } from "$lib/metric";

  let { it, size = 30, from, slot = true }: { it: ItemStat; size?: number; from?: ItemStat[]; slot?: boolean } = $props();
  const item = $derived(app.assets?.items[it.id]);
  const base = $derived(app.build?.wr ?? 50);
  const def = $derived(app.defHighlight.includes(it.id));
  const score = $derived(app.metric === "wpa" && from ? itemWpa(from, base, slot).get(it.id) : undefined);
  const up = $derived(score !== undefined ? score >= 0.5 : it.wr - base >= 1);
  const down = $derived(score !== undefined ? score <= -0.5 : it.wr - base <= -1);
</script>

<div class="it" class:low={it.n < app.minGames} class:def style="--s: {size}px"
  title="{item?.name} · {pct(it.wr)}{score !== undefined ? ` · ${pp(score)} pp (${slot ? 'contra o típico da posição' : 'contra o WR do campeão'})` : ''} · pick {pct(it.pr)} · {num(it.n)} partidas{def ? ' · defesa contra o time inimigo' : ''}">
  <img src={icon(item?.icon)} alt={item?.name} />
  <span class:up class:down>{score !== undefined ? pp(score) : pct(it.wr)}</span>
</div>

<style>
  .it { display: grid; justify-items: center; font-size: 10px; font-variant-numeric: tabular-nums; }
  .it.low { opacity: 0.45; }
  img { width: var(--s); height: var(--s); border-radius: 4px; border: 1px solid var(--gold-dim); background: var(--panel-2); }
  .def img { border-color: var(--good); box-shadow: 0 0 0 1px var(--good); }
  .up { color: var(--good); }
  .down { color: var(--bad); }
</style>
