<script lang="ts">
  // Páginas de runas próprias (OTP, várias por campeão): o que a escolhida muda em relação à mais
  // jogada, em pontos de WR (estatística de cada runa naquele lugar da página), e os botões de
  // criar, editar, copiar do cliente e apagar. Copiar só LÊ a página atual do cliente.
  import { icon, pts, runeDiffs, ownRuneSet, type RuneDiff } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { i18n, t } from "$lib/i18n/index.svelte";

  /** `compact` = diferenças como ícones (Grade, Por fase); senão uma linha por runa (Abas, Clássico). */
  let { compact = true }: { compact?: boolean } = $props();

  const own = $derived(app.ownPage);
  const b = $derived(app.build);
  const showing = $derived(!!own);
  const diffs = $derived(own && b ? runeDiffs(ownRuneSet(own), b.runesPick, b.runeStats) : []);
  const perk = (id: number) => app.assets?.perks[id];
  const name = (id: number) => perk(id)?.name ?? `#${id}`;
  const champ = $derived(app.champ?.name ?? t("ownRunes.theChampion"));
  const saved = $derived(own ? new Date(own.savedAt * 1000).toLocaleDateString(i18n.tag) : "");
  let confirming = $state(false);

  const tip = (d: RuneDiff) =>
    d.delta === null
      ? t("ownRunes.tipNoData", { mine: name(d.mine), other: name(d.other) })
      : t("ownRunes.tip", { mine: name(d.mine), other: name(d.other), delta: pts(d.delta, 1), count: d.n ?? 0 });

  async function remove() {
    confirming = false;
    if (own) await app.deleteOwnRunes(own.championId, own.id);
  }
</script>

{#if b}
  <div class="own">
    {#if showing}
      {#if !diffs.length}
        <p class="line muted">{t("ownRunes.same")}</p>
      {:else if compact}
        <div class="chips">
          <span class="muted">{t("ownRunes.vsMostPlayed")}</span>
          {#each diffs as d, i (i)}
            <span class="chip" title={tip(d)}>
              <img src={icon(perk(d.mine)?.icon)} alt={name(d.mine)} />
              <b class:up={(d.delta ?? 0) > 0} class:down={(d.delta ?? 0) < 0}>{d.delta === null ? "?" : pts(d.delta, 1)}</b>
            </span>
          {/each}
        </div>
      {:else}
        {#each diffs as d, i (i)}
          <p class="line" title={tip(d)}>
            <img src={icon(perk(d.mine)?.icon)} alt="" /> {name(d.mine)} <span class="muted">{t("ownRunes.insteadOf", { other: name(d.other) })}</span>
            {#if d.delta === null}<span class="muted">{t("ownRunes.noData")}</span>
            {:else}<span class="nw"><b class:up={d.delta > 0} class:down={d.delta < 0}>{t("ownRunes.pts", { delta: pts(d.delta, 1) })}</b> <span class="muted">({t("common.games", { count: d.n ?? 0 })})</span></span>{/if}
          </p>
        {/each}
      {/if}
    {/if}
    <div class="acts">
      {#if confirming}
        <span>{t("ownRunes.confirmDeleteNamed", { page: own?.name ?? "", name: champ })}</span>
        <button class="link bad" onclick={remove}>{t("parts.delete")}</button>
        <button class="link" onclick={() => (confirming = false)}>{t("parts.no")}</button>
      {:else}
        <button class="link" onclick={() => app.openEditor(null)} disabled={app.ownBusy}
          title={t("ownRunes.newTip", { name: champ })}>
          {t("parts.newPage")}
        </button>
        {#if app.snap?.connected}
          <button class="link" onclick={() => app.copyFromClient()} disabled={app.ownBusy}
            title={t("ownRunes.copyTipNew", { name: champ })}>
            {t("parts.copyFromClient")}
          </button>
        {/if}
        {#if own}
          <button class="link" onclick={() => app.openEditor(own.id)} disabled={app.ownBusy} title={t("ownRunes.editTip", { page: own.name })}>{t("ownRunes.editPage")}</button>
          <button class="link" onclick={() => (confirming = true)} title={t("ownRunes.savedOn", { date: saved })}>{t("parts.delete")}</button>
        {/if}
      {/if}
    </div>
    {#if app.ownError}<p class="line bad">{app.ownError}</p>{/if}
  </div>
{/if}

<style>
  .own { display: grid; gap: 3px; font-size: 11px; }
  .line { margin: 0; display: flex; align-items: center; gap: 4px; flex-wrap: wrap; }
  .line img { width: 16px; height: 16px; border-radius: 50%; }
  .chips { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .chip { display: inline-flex; align-items: center; gap: 2px; font-variant-numeric: tabular-nums; }
  .chip img { width: 18px; height: 18px; border-radius: 50%; background: var(--panel-2); }
  .acts { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .acts button.link { border: none; background: none; color: var(--gold); padding: 0; font-size: 11px; cursor: pointer; }
  .acts button.link:disabled { opacity: 0.5; cursor: default; }
  .acts button.bad, .bad { color: var(--bad); }
  .up { color: var(--good); }
  .down { color: var(--bad); }
  .muted { color: var(--muted); }
  .nw { white-space: nowrap; }
</style>
