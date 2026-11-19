<script lang="ts">
  import { icon, pct, num, type Assets, type RuneSet } from "./api";

  let { set, assets }: { set: RuneSet; assets: Assets | null } = $props();

  const perk = (id: number) => assets?.perks[id];
  const styleOf = (id: number) => assets?.styles.find((s) => s.perks.includes(id));
  const primary = $derived(styleOf(set.primary[0]));
  const secondary = $derived(assets?.styles.find((s) => s.id !== primary?.id && set.secondary.every((p) => s.perks.includes(p))));
</script>

<div class="runes">
  <div class="tree">
    {#if primary}<img class="style" src={icon(primary.icon)} alt={primary.name} title={primary.name} />{/if}
    {#each set.primary as id, i (id)}
      <img class:keystone={i === 0} src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />
    {/each}
  </div>
  <div class="tree">
    {#if secondary}<img class="style" src={icon(secondary.icon)} alt={secondary.name} title={secondary.name} />{/if}
    {#each set.secondary as id (id)}
      <img src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />
    {/each}
    <span class="sep"></span>
    {#each set.shards as id, i (i)}
      <img class="shard" src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={perk(id)?.name} />
    {/each}
  </div>
  <div class="meta">WR {pct(set.wr)} · {num(set.n)} partidas</div>
</div>

<style>
  .runes { display: grid; gap: 6px; }
  .tree { display: flex; align-items: center; gap: 6px; }
  img { width: 30px; height: 30px; border-radius: 50%; background: var(--panel-2); }
  img.keystone { width: 44px; height: 44px; }
  img.style { width: 22px; height: 22px; background: none; opacity: 0.8; }
  img.shard { width: 20px; height: 20px; }
  .sep { width: 8px; }
  .meta { color: var(--muted); font-size: 12px; }
</style>
