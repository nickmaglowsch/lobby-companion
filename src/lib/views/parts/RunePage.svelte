<script lang="ts">
  // Página de runas compacta com a escolha Mais jogada × Maior WR (a escolha é a configuração:
  // trocar aqui troca a página no cliente). `size` = tamanho das runas comuns.
  import { icon, pct, num } from "$lib/api";
  import { app } from "$lib/app.svelte";

  let { size = 28, layout = "rows", toggle = true }: { size?: number; layout?: "rows" | "key"; toggle?: boolean } = $props();
  const set = $derived(app.runeSet);
  const perk = (id: number) => app.assets?.perks[id];
  const styleOf = (id: number) => app.assets?.styles.find((s) => s.perks.includes(id));
  const primary = $derived(set ? styleOf(set.primary[0]) : undefined);
  const secondary = $derived(
    set ? app.assets?.styles.find((s) => s.id !== primary?.id && set.secondary.every((p) => s.perks.includes(p))) : undefined,
  );
  const hasWin = $derived(!!app.build?.runesWin);
</script>

{#if set}
  <div class="rp" style="--r: {size}px">
    {#if toggle}
      <div class="seg">
        <button class:on={app.runeTab === "pick"} onclick={() => app.chooseRunes("pick")}>Mais jogada</button>
        <button class:on={app.runeTab === "win"} onclick={() => app.chooseRunes("win")} disabled={!hasWin}>Maior WR</button>
      </div>
    {/if}
    {#if layout === "key"}
      <div class="keyed">
        <img class="key big" src={icon(perk(set.primary[0])?.icon)} alt={perk(set.primary[0])?.name} title={perk(set.primary[0])?.name} />
        <div class="col">
          <div class="tree">{#each set.primary.slice(1) as id (id)}<img src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />{/each}</div>
          <div class="tree">
            {#if secondary}<img class="style" src={icon(secondary.icon)} alt={secondary.name} title={secondary.name} />{/if}
            {#each set.secondary as id (id)}<img src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />{/each}
          </div>
          <div class="tree">{#each set.shards as id, i (i)}<img class="shard" src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />{/each}</div>
        </div>
      </div>
    {:else}
      <div class="tree">
        {#if primary}<img class="style" src={icon(primary.icon)} alt={primary.name} title={primary.name} />{/if}
        {#each set.primary as id, i (id)}<img class:key={i === 0} src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />{/each}
      </div>
      <div class="tree">
        {#if secondary}<img class="style" src={icon(secondary.icon)} alt={secondary.name} title={secondary.name} />{/if}
        {#each set.secondary as id (id)}<img src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />{/each}
        <span class="sep"></span>
        {#each set.shards as id, i (i)}<img class="shard" src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />{/each}
      </div>
    {/if}
    <div class="meta">WR {pct(set.wr)} · {num(set.n)} partidas</div>
  </div>
{:else}
  <p class="meta">Sem runas.</p>
{/if}

<style>
  .rp { display: grid; gap: 6px; }
  .seg { display: flex; }
  .seg button { font-size: 11px; padding: 2px 8px; border-radius: 0; }
  .seg button:first-child { border-radius: 4px 0 0 4px; }
  .seg button:last-child { border-radius: 0 4px 4px 0; }
  .seg button.on { border-color: var(--gold); color: var(--gold); }
  .tree { display: flex; align-items: center; gap: 5px; }
  img { width: var(--r); height: var(--r); border-radius: 50%; background: var(--panel-2); }
  img.key { width: calc(var(--r) * 1.5); height: calc(var(--r) * 1.5); }
  img.big { width: calc(var(--r) * 2.1); height: calc(var(--r) * 2.1); }
  img.style { width: calc(var(--r) * 0.6); height: calc(var(--r) * 0.6); background: none; }
  img.shard { width: calc(var(--r) * 0.68); height: calc(var(--r) * 0.68); }
  .keyed { display: flex; gap: 10px; align-items: center; }
  .col { display: grid; gap: 6px; }
  .sep { width: 4px; }
  .meta { color: var(--muted); font-size: 11px; margin: 0; }
</style>
