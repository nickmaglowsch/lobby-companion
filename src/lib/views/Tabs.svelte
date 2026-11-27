<script lang="ts">
  // Layout "Abas": cabeçalho fixo com o campeão e o Importar; o corpo é uma aba por vez
  // (Draft · Build · Runas · Confronto). A aba segue a etapa da seleção até você clicar em outra.
  import { championIcon, icon, pct, laneName, phaseName, api } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import Search from "./parts/Search.svelte";
  import Notices from "./parts/Notices.svelte";
  import BanList from "./parts/BanList.svelte";
  import PickList from "./parts/PickList.svelte";
  import MetricToggle from "./parts/MetricToggle.svelte";
  import { banShort } from "$lib/metric";
  import RunePage from "./parts/RunePage.svelte";
  import SkillGrid from "./parts/SkillGrid.svelte";
  import Item from "./parts/Item.svelte";
  import ImportButton from "./parts/ImportButton.svelte";
  import VsPicker from "./parts/VsPicker.svelte";
  import DamageMix from "./parts/DamageMix.svelte";
  import Trend from "$lib/Trend.svelte";
  import DamageBar from "$lib/DamageBar.svelte";
  import RuneAlternatives from "$lib/RuneAlternatives.svelte";
  import { t } from "$lib/i18n/index.svelte";

  type Tab = "draft" | "build" | "runas" | "confronto";
  const TABS: [Tab, string][] = [["draft", "vTabs.draft"], ["build", "views.build"], ["runas", "common.runes"], ["confronto", "views.matchup"]];
  let chosen = $state<Tab | null>(null);
  let lastStage = app.stage;
  $effect(() => {
    // Etapa nova na seleção: volta a seguir o cliente.
    if (app.stage !== lastStage) {
      lastStage = app.stage;
      chosen = null;
    }
  });
  const tab = $derived<Tab>(chosen ?? (app.stage === "bans" || app.stage === "picks" ? "draft" : "build"));
  const b = $derived(app.build);
  const core = $derived(app.cores[0]);
  const bans = $derived(app.snap?.bans ?? null);
  const item = (id: number | string) => app.assets?.items[String(id)];
</script>

<div class="tabs-layout">
  <header>
    {#if b && app.snap}
      <img class="portrait" src={championIcon(app.snap.championId)} alt="" />
      <div class="who">
        <div class="title">
          <b>{app.champ?.name ?? b.slug}</b>
          {#if app.lanes.length > 1}
            <select class="lane" value={b.lane} onchange={(e) => app.changeLane(e.currentTarget.value)}>
              {#each app.lanes as [lane, p] (lane)}<option value={lane}>{laneName(lane)} {Math.round(p)}%</option>{/each}
            </select>
          {:else}<span class="muted">{laneName(b.lane)}</span>{/if}
          <span class="muted small">{b.patch}</span>
        </div>
        <div class="muted small">WR <b class="txt">{pct(b.wr)}</b> · PR {pct(b.pr)} · BR {pct(b.br)}{#if b.rank} · #{b.rank}/{b.rankTotal}{/if} · {t("common.games", { count: b.n })}</div>
      </div>
    {:else}
      <div class="who"><div class="title"><b>Lobby Companion</b></div><div class="muted small">{app.snap?.inChampSelect ? t("vTabs.lockToSee") : t("vTabs.searchToSee")}</div></div>
    {/if}
    <div class="right">
      <span class="phase"><span class="dot" class:on={app.snap?.connected}></span>{#if !app.snap?.connected}{t("common.clientClosed")}{:else}{phaseName(app.snap.phase)}{/if}</span>
      <ImportButton labels={false} />
    </div>
  </header>

  <nav>
    {#each TABS as [id, label] (id)}
      <button class:on={tab === id} onclick={() => (chosen = id)}>{t(label)}</button>
    {/each}
    <span class="grow"></span>
    <Search width={120} />
    <button class="icon" title={t("views.settings")} aria-label={t("views.settings")} onclick={() => (app.settingsOpen = true)}>⚙</button>
  </nav>
  <Notices />

  <main>
    {#if tab === "draft"}
      <div class="two">
        <section>
          <h2>{t("views.bans")} {#if bans}<MetricToggle />{/if} {#if bans?.loading}<span class="muted">{t("common.calculating")}</span>{/if}</h2>
          {#if bans}
            <div class="bans">
              <div>
                <h3>{bans.byTier ? t("vTabs.laneTierList") : t("views.byTeam")}</h3>
                {#if bans.picks.length}<BanList list={bans.picks} />{:else}<p class="muted small">{bans.note ?? "—"}</p>{/if}
              </div>
              <div>
                <h3>{t("vTabs.vsYou")}</h3>
                {#if bans.lanePicks.length}<BanList list={bans.lanePicks} />{:else}<p class="muted small">{bans.laneNote ?? "—"}</p>{/if}
              </div>
            </div>
            <p class="muted small">{banShort(bans.metric)}</p>
          {:else}
            <p class="muted small">{t("vTabs.bansLater")}</p>
          {/if}
        </section>
        <section>
          {#if app.snap?.inChampSelect && app.snap.picks}
            {@const laneLabel = laneName(app.snap.picks.lane)}
            <h2>{laneLabel ? t("vTabs.pickFor", { lane: laneLabel }) : t("vTabs.pickForPosition")} <MetricToggle /></h2>
            <PickList variant="detailed" />
          {/if}
          {#if app.snap?.synergy?.lines.length}
            <h2>{t("vTabs.synergy")}</h2>
            {#each app.snap.synergy.lines as l (l.allyChampionId)}
              <div class="syn"><img src={championIcon(l.allyChampionId)} alt="" /> {t("views.withAlly", { champ: app.champName(l.allyChampionId) })} <b class:up={l.delta > 0} class:down={l.delta < 0}>{pct(l.wr)}</b></div>
            {/each}
          {/if}
          {#if app.composition}
            <h2>{t("views.damage")}</h2>
            <DamageMix mode="enemy" />
          {/if}
        </section>
      </div>
    {:else if !b}
      <p class="muted empty">{app.snap?.inChampSelect ? t("common.emptyLocked") : t("vTabs.emptyIdle")}</p>
    {:else if tab === "build"}
      <div class="two">
        <section>
          <h2>{t("common.items")} <MetricToggle /></h2>
          {#if b.startSets[0]}
            <div class="irow"><span class="lbl">{t("common.start")}</span>
              <div class="icons">{#each b.startSets[0].ids as id, i (i)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
              <span class="small">{app.startText(0)}</span></div>
          {/if}
          {#if app.firstBackItems.length}
            <div class="irow"><span class="lbl">{t("common.firstBack")}</span>
              <div class="icons">{#each app.firstBackItems.slice(0, 4) as it (it.id)}<img src={icon(item(it.id)?.icon)} alt="" title="{item(it.id)?.name} · {pct(it.wr)}" />{/each}</div></div>
          {/if}
          {#if core}
            <div class="irow"><span class="lbl">{t("common.core")}</span>
              <div class="icons">{#each core.ids as id (id)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
              <span class="small">{app.coreText(core.combo)}</span></div>
          {/if}
          <div class="slots">
            {#each [[t("common.boots"), b.boots], [t("common.itemN", { n: 4 }), b.slots[3] ?? []], [t("common.itemN", { n: 5 }), b.slots[4] ?? []]] as const as [label, list] (label)}
              <div class="slot">
                <span class="lbl">{label}</span>
                <div class="icons">{#each list.filter((i) => item(i.id)).slice(0, 3) as it (it.id)}<Item {it} from={list} size={26} />{/each}</div>
              </div>
            {/each}
          </div>
          {#if app.situational.length}
            <div class="irow"><span class="lbl">{t("common.situationalShort")}</span>
              <div class="icons">{#each app.situational as it (it.id)}<Item {it} from={b.winningItems ?? []} slot={false} size={26} />{/each}</div></div>
          {/if}
        </section>
        <section>
          <h2>{t("common.spells")}</h2>
          {#each b.spells.slice(0, 2) as c, i (c.ids.join())}
            <div class="spells" class:alt={i > 0}>
              {#each c.ids as id (id)}<img src={icon(app.assets?.spells[id]?.icon)} alt="" title={app.assets?.spells[id]?.name} />{/each}
              <span class="small">{pct(c.wr)}</span><span class="muted small">{pct(c.pr)}</span>
            </div>
          {/each}
          <h2>{t("common.skills")}</h2>
          <SkillGrid cell={15} numbers />
          {#if b.trend}<h2>{t("vTabs.trend")}</h2><Trend trend={b.trend} />{/if}
          {#if b.damage}<DamageBar damage={b.damage} />{/if}
          {#if app.snap?.pageUrl}<button class="link more" onclick={() => api.openUrl(app.snap!.pageUrl!)}>{t("views.openLolalytics")}</button>{/if}
        </section>
      </div>
    {:else if tab === "runas"}
      <div class="two">
        <section>
          <h2>{t("vTabs.page")}</h2>
          <RunePage size={32} compact={false} />
        </section>
        <section class="alts">
          <h2>{t("vTabs.alternatives")}</h2>
          {#if app.runeSet}<RuneAlternatives set={app.runeSet} stats={b.runeStats} assets={app.assets} minGames={app.minGames} open />{/if}
        </section>
      </div>
    {:else}
      <div class="two">
        <section>
          <h2>{t("vTabs.buildVs")}</h2>
          <VsPicker full size={40} />
        </section>
        <section>
          {#if app.composition}<h2>{t("vTabs.composition")}</h2><DamageMix />{/if}
        </section>
      </div>
    {/if}
  </main>
</div>

<style>
  .tabs-layout { height: 100vh; display: grid; grid-template-rows: 64px 38px auto 1fr; overflow: hidden; }
  header { display: flex; align-items: center; gap: 10px; padding: 0 12px; background: var(--panel); min-width: 0; }
  .portrait { width: 46px; height: 46px; border: 2px solid var(--gold); border-radius: 6px; }
  .who { min-width: 0; }
  .title { font-size: 17px; display: flex; align-items: center; gap: 8px; }
  .lane { font-size: 12px; padding: 1px 4px; color: var(--gold); }
  .txt { color: var(--text); }
  .right { margin-inline-start: auto; display: grid; justify-items: end; gap: 4px; }
  .phase { font-size: 11px; color: var(--muted); display: flex; align-items: center; gap: 5px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--bad); }
  .dot.on { background: var(--good); }
  nav { display: flex; align-items: center; gap: 2px; padding: 0 8px; border-bottom: 1px solid var(--gold-dim); }
  nav > button:not(.icon) { align-self: stretch; border: none; border-bottom: 2px solid transparent; border-radius: 0; background: none; color: var(--muted); padding: 0 12px; }
  nav > button.on { color: var(--gold); border-bottom-color: var(--gold); }
  .icon { width: 28px; height: 28px; padding: 0; font-size: 15px; margin-inline-start: 6px; }
  .grow { flex: 1; }
  main { min-height: 0; overflow: hidden; }
  .two { display: grid; grid-template-columns: 1fr 1fr; height: 100%; }
  section { padding: 10px 14px; overflow: hidden; min-width: 0; }
  .two section:first-child { border-inline-end: 1px solid var(--line); }
  h2 { margin: 12px 0 6px; font-size: 12px; color: var(--gold); text-transform: uppercase; letter-spacing: 0.04em; }
  h2 .muted { text-transform: none; font-weight: normal; }
  section > h2:first-child { margin-top: 0; }
  h3 { margin: 0 0 4px; font-size: 11px; color: var(--muted); font-weight: normal; }
  .bans { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .syn { display: flex; align-items: center; gap: 6px; font-size: 13px; }
  .syn img { width: 20px; height: 20px; border-radius: 3px; }
  .irow { display: grid; grid-template-columns: 56px 1fr auto; align-items: center; gap: 6px; padding: 5px 0; border-top: 1px solid var(--line); }
  .lbl { font-size: 11px; color: var(--muted); }
  .icons { display: flex; gap: 4px; }
  .icons img { width: 28px; height: 28px; border-radius: 4px; border: 1px solid var(--gold-dim); }
  .slots { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; padding: 6px 0; border-top: 1px solid var(--line); }
  .slot { display: grid; gap: 2px; }
  .spells { display: flex; gap: 6px; align-items: center; margin-bottom: 4px; font-variant-numeric: tabular-nums; }
  .spells img { width: 30px; height: 30px; border-radius: 4px; }
  .spells.alt img { width: 22px; height: 22px; }
  .more { margin-top: 10px; padding: 0; }
  /* Única coluna que rola: as alternativas de runa passam da altura da janela. */
  .alts { overflow-y: auto; }
  .empty { padding: 40px 24px; text-align: center; }
  .small { font-size: 11px; }
</style>
