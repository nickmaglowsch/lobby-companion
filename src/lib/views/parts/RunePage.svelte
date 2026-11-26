<script lang="ts">
  // Página de runas compacta com a escolha entre as suas páginas do campeão (pelo nome),
  // Mais jogada e Maior WR (a escolha vai para o cliente).
  // `size` = tamanho das runas comuns.
  import { icon, pct, perkStat } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { i18n, t } from "$lib/i18n/index.svelte";
  import OwnRunes from "./OwnRunes.svelte";

  let { size = 28, layout = "rows", toggle = true, compact = true }: {
    size?: number; layout?: "rows" | "key"; toggle?: boolean;
    /** Diferenças da página própria como ícones (true) ou uma linha por runa. */
    compact?: boolean;
  } = $props();
  const set = $derived(app.runeSet);
  const perk = (id: number) => app.assets?.perks[id];
  const styleOf = (id: number) => app.assets?.styles.find((s) => s.perks.includes(id));
  const primary = $derived(set ? styleOf(set.primary[0]) : undefined);
  const secondary = $derived(
    set ? app.assets?.styles.find((s) => s.id !== primary?.id && set.secondary.every((p) => s.perks.includes(p))) : undefined,
  );
  const hasWin = $derived(!!app.build?.runesWin);
  const mine = $derived(!!app.ownPage);
  /** Nome + WR da runa naquele lugar da página (posição 0–8, como em `perkStat`). */
  function tip(id: number, i: number): string {
    const s = perkStat(app.build?.runeStats, id, i);
    const name = perk(id)?.name ?? String(id);
    return s ? t("runePage.tip", { name, wr: pct(s.wr), count: s.n }) : name;
  }
  const saved = $derived(app.ownPage ? new Date(app.ownPage.savedAt * 1000).toLocaleDateString(i18n.tag) : "");
</script>

{#snippet rune(id: number, i: number, cls = "")}
  <img class={cls} src={icon(perk(id)?.icon)} alt={perk(id)?.name} title={tip(id, i)} />
{/snippet}

{#if set}
  <div class="rp" style="--r: {size}px">
    {#if toggle}
      <div class="seg">
        {#each app.championPages as p (p.id)}
          <button class="own-tab" class:on={app.runeTab === `own:${p.id}`} onclick={() => app.chooseRunes(`own:${p.id}`)}
            title={t("parts.ownTabTip", { name: p.name })}>{p.name}</button>
        {/each}
        <button class:on={app.runeTab === "pick"} onclick={() => app.chooseRunes("pick")}>{t("parts.mostPlayed")}</button>
        <button class:on={app.runeTab === "win"} onclick={() => app.chooseRunes("win")} disabled={!hasWin}>{t("parts.highestWr")}</button>
      </div>
    {/if}
    {#if layout === "key"}
      <div class="keyed">
        {@render rune(set.primary[0], 0, "key big")}
        <div class="col">
          <div class="tree">{#each set.primary.slice(1) as id, i (id)}{@render rune(id, i + 1)}{/each}</div>
          <div class="tree">
            {#if secondary}<img class="style" src={icon(secondary.icon)} alt={secondary.name} title={secondary.name} />{/if}
            {#each set.secondary as id, i (id)}{@render rune(id, i + 4)}{/each}
          </div>
          <div class="tree">{#each set.shards as id, i (i)}{@render rune(id, i + 6, "shard")}{/each}</div>
        </div>
      </div>
    {:else}
      <div class="tree">
        {#if primary}<img class="style" src={icon(primary.icon)} alt={primary.name} title={primary.name} />{/if}
        {#each set.primary as id, i (id)}{@render rune(id, i, i === 0 ? "key" : "")}{/each}
      </div>
      <div class="tree">
        {#if secondary}<img class="style" src={icon(secondary.icon)} alt={secondary.name} title={secondary.name} />{/if}
        {#each set.secondary as id, i (id)}{@render rune(id, i + 4)}{/each}
        <span class="sep"></span>
        {#each set.shards as id, i (i)}{@render rune(id, i + 6, "shard")}{/each}
      </div>
    {/if}
    {#if mine}
      <div class="meta">{t("parts.ownSavedOn", { name: app.ownPage?.name ?? "", date: saved })}</div>
    {:else}
      <div class="meta">{t("runePage.meta", { wr: pct(set.wr), count: set.n })}</div>
    {/if}
    <OwnRunes {compact} />
  </div>
{:else}
  <p class="meta">{t("runePage.none")}</p>
{/if}

<style>
  .rp { display: grid; gap: 6px; }
  .seg { display: flex; flex-wrap: wrap; gap: 3px; }
  .seg button { font-size: 11px; padding: 2px 8px; border-radius: 4px; }
  .seg .own-tab { max-width: 110px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
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
