<script lang="ts">
  // Layout "Grade": três colunas fixas (draft · runas · itens), tudo numa tela, sem rolagem.
  // A coluna da etapa atual da seleção fica destacada; detalhes ficam no tooltip.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { icon, pct, LANES, coreSets, firstBack, situationalItems } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import TopBar from "./parts/TopBar.svelte";
  import ChampLine from "./parts/ChampLine.svelte";
  import Notices from "./parts/Notices.svelte";
  import BanList from "./parts/BanList.svelte";
  import PickList from "./parts/PickList.svelte";
  import MetricToggle from "./parts/MetricToggle.svelte";
  import RunePage from "./parts/RunePage.svelte";
  import SkillGrid from "./parts/SkillGrid.svelte";
  import Item from "./parts/Item.svelte";
  import ImportButton from "./parts/ImportButton.svelte";
  import VsPicker from "./parts/VsPicker.svelte";
  import DamageMix from "./parts/DamageMix.svelte";

  const b = $derived(app.build);
  const bans = $derived(app.snap?.bans ?? null);
  const draftFocus = $derived(app.stage === "bans" || app.stage === "picks");
  const core = $derived(b ? coreSets(b, app.assets, app.minGames)[0] : undefined);
  const item = (id: number | string) => app.assets?.items[String(id)];
</script>

<div class="grid-layout">
  <TopBar status={!b}><ChampLine /></TopBar>
  <Notices />

  <div class="cols">
    <section class:focus={draftFocus || !b}>
      {#if bans}
        <h2>{t("views.bans")} <span class="muted">{bans.byTier
          ? (bans.lane ? t("vGrid.tierListLane", { lane: LANES[bans.lane] ?? bans.lane }) : t("vGrid.tierList"))
          : t("vGrid.byTeam")}</span>
          <MetricToggle />
          {#if bans.loading}<span class="muted">…</span>{/if}</h2>
        {#if bans.picks.length}<BanList list={bans.picks} variant="strip" />{:else if bans.note}<p class="muted small">{bans.note}</p>{/if}
        <h2>{t("views.bans")} <span class="muted">{t("vGrid.vsYou")}</span></h2>
        {#if bans.lanePicks.length}<BanList list={bans.lanePicks} variant="strip" />{:else}<p class="muted small">{bans.laneNote ?? "—"}</p>{/if}
      {/if}
      {#if app.snap?.inChampSelect && app.snap.picks}
        <h2>{t("vGrid.pick")} <span class="muted">{LANES[app.snap.picks.lane ?? ""] ?? ""} · {app.snap.picks.pool === "tier" ? t("vGrid.tierList") : t("vGrid.ownPool")}</span>{#if !bans}<MetricToggle />{/if}</h2>
        <PickList variant="rows" max={3} />
      {/if}
      {#if app.composition}
        <h2>{t("views.damage")}</h2>
        <DamageMix mode="enemy" />
      {/if}
      {#if !bans && !app.snap?.picks && !app.composition}
        <p class="muted small empty">{t("vGrid.emptyDraft")}</p>
      {/if}
    </section>

    <section class:focus={app.stage === "build"}>
      {#if b}
        <h2>{t("common.runes")}</h2>
        <RunePage size={27} />
        <h2>{t("common.spells")}</h2>
        {#each b.spells.slice(0, 2) as c, i (c.ids.join())}
          <div class="spells" class:alt={i > 0}>
            {#each c.ids as id (id)}<img src={icon(app.assets?.spells[id]?.icon)} alt={app.assets?.spells[id]?.name} title={app.assets?.spells[id]?.name} />{/each}
            <span class="small">{pct(c.wr)}</span><span class="muted small">{pct(c.pr)}</span>
          </div>
        {/each}
        <h2>{t("common.skills")}</h2>
        <SkillGrid cell={11} />
      {:else}
        <p class="muted small empty">{app.snap?.inChampSelect ? t("common.emptyLocked") : t("common.emptySearch")}</p>
      {/if}
    </section>

    <section class:focus={app.stage === "build"}>
      {#if b}
        <h2>{t("common.items")} <MetricToggle /></h2>
        {#if b.startSets[0]}
          <div class="irow">
            <span class="lbl">{t("common.start")}</span>
            <div class="icons">{#each b.startSets[0].ids as id, i (i)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
            <span class="small">{app.startText(0)}</span>
          </div>
        {/if}
        {#if firstBack(b, app.assets, app.minGames).length}
          <div class="irow">
            <span class="lbl">{t("common.firstBack")}</span>
            <div class="icons">{#each firstBack(b, app.assets, app.minGames).slice(0, 3) as it (it.id)}<img src={icon(item(it.id)?.icon)} alt="" title="{item(it.id)?.name} · {pct(it.wr)}" />{/each}</div>
          </div>
        {/if}
        {#if core}
          <div class="irow">
            <span class="lbl">{t("common.core")}</span>
            <div class="icons">{#each core.ids as id (id)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
            <span class="small">{app.coreText(core.combo)}</span>
          </div>
        {/if}
        {#each [[t("common.boots"), b.boots], [t("common.itemN", { n: 4 }), b.slots[3] ?? []], [t("common.itemN", { n: 5 }), b.slots[4] ?? []]] as const as [label, list] (label)}
          {#if list.length}
            <div class="irow">
              <span class="lbl">{label}</span>
              <div class="icons">{#each list.filter((i) => item(i.id)).slice(0, 3) as it (it.id)}<Item {it} from={list} size={28} />{/each}</div>
            </div>
          {/if}
        {/each}
        {#if situationalItems(b, app.assets, app.minGames, app.metric).length}
          <div class="irow">
            <span class="lbl">{t("common.situationalShort")}</span>
            <div class="icons">{#each situationalItems(b, app.assets, app.minGames, app.metric).slice(0, 3) as it (it.id)}<Item {it} from={b.winningItems ?? []} slot={false} size={28} />{/each}</div>
          </div>
        {/if}
      {/if}
    </section>
  </div>

  <footer>
    <ImportButton />
    <span class="grow"></span>
    {#if b}<VsPicker />{/if}
    {#if app.snap?.pageUrl}<button class="link" onclick={() => openUrl(app.snap!.pageUrl!)}>lolalytics ↗</button>{/if}
  </footer>
</div>

<style>
  .grid-layout { height: 100vh; display: grid; grid-template-rows: 44px auto 1fr 44px; overflow: hidden; }
  .cols { display: grid; grid-template-columns: 236px 1fr 250px; min-height: 0; }
  section { padding: 8px 12px; border-inline-end: 1px solid var(--line); overflow: hidden; opacity: 0.75; transition: opacity 0.2s; }
  section:last-child { border-inline-end: none; }
  section.focus, section:hover { opacity: 1; }
  h2 { margin: 10px 0 6px; font-size: 12px; color: var(--gold); text-transform: uppercase; letter-spacing: 0.04em; display: flex; gap: 6px; align-items: baseline; }
  h2 .muted { text-transform: none; letter-spacing: 0; font-weight: normal; font-size: 11px; }
  section > h2:first-child { margin-top: 0; }
  .spells { display: flex; gap: 6px; align-items: center; margin-bottom: 4px; font-variant-numeric: tabular-nums; }
  .spells img { width: 28px; height: 28px; border-radius: 4px; }
  .spells.alt img { width: 20px; height: 20px; }
  .irow { display: grid; grid-template-columns: 52px 1fr auto; align-items: center; gap: 6px; padding: 4px 0; border-top: 1px solid var(--line); }
  .lbl { font-size: 11px; color: var(--muted); }
  .icons { display: flex; gap: 4px; }
  .icons img { width: 28px; height: 28px; border-radius: 4px; border: 1px solid var(--gold-dim); }
  footer { display: flex; align-items: center; gap: 10px; padding: 0 12px; background: var(--panel); border-top: 1px solid var(--line); }
  .grow { flex: 1; }
  .muted { color: var(--muted); }
  .small { font-size: 11px; }
  .empty { margin-top: 24px; text-align: center; }
</style>
