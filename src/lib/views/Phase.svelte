<script lang="ts">
  // Layout "Por fase": a tela inteira é da etapa atual da seleção (bans → escolha → build).
  // A etapa segue o cliente sozinha; dá para olhar outra clicando nos passos de cima.
  import { championIcon, icon, pct, laneName, api } from "$lib/api";
  import { app, type Stage } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import TopBar from "./parts/TopBar.svelte";
  import Notices from "./parts/Notices.svelte";
  import BanList from "./parts/BanList.svelte";
  import PickList from "./parts/PickList.svelte";
  import MetricToggle from "./parts/MetricToggle.svelte";
  import RunePage from "./parts/RunePage.svelte";
  import Item from "./parts/Item.svelte";
  import ImportButton from "./parts/ImportButton.svelte";
  import VsPicker from "./parts/VsPicker.svelte";
  import DamageMix from "./parts/DamageMix.svelte";

  const STEPS: [Stage, string][] = [["bans", "views.bans"], ["picks", "vPhase.pick"], ["build", "views.build"]];
  let chosen = $state<Stage | null>(null);
  let lastStage = app.stage;
  $effect(() => {
    if (app.stage !== lastStage) {
      lastStage = app.stage;
      chosen = null;
    }
  });
  const step = $derived<Stage>(chosen ?? app.stage);
  const stageIndex = $derived(STEPS.findIndex(([s]) => s === app.stage));
  const b = $derived(app.build);
  const core = $derived(app.cores[0]);
  const bans = $derived(app.snap?.bans ?? null);
  const item = (id: number | string) => app.assets?.items[String(id)];
  const me = $derived(bans?.laneFor ?? null);
</script>

<div class="phase-layout">
  <TopBar status={!app.snap?.inChampSelect}>
    {#if app.snap?.inChampSelect}
      <ol>
        {#each STEPS as [id, label], i (id)}
          <li class:on={step === id} class:done={stageIndex > i}>
            <button onclick={() => (chosen = id)}><span class="n">{stageIndex > i ? "✓" : i + 1}</span>{t(label)}</button>
          </li>
        {/each}
      </ol>
    {/if}
  </TopBar>
  <Notices />

  {#if step === "bans"}
    <main class="bans">
      {#if bans}
        <section>
          <h2>{bans.byTier ? t("vPhase.strongestInLane") : t("vPhase.byYourTeam")} <MetricToggle /> {#if bans.loading}<span class="muted small">{t("common.calculating")}</span>{/if}</h2>
          <p class="muted small sub">
            {#if bans.byTier}
              {bans.lane ? t("vPhase.noneDeclaredLane", { lane: laneName(bans.lane) }) : t("vPhase.noneDeclared")}
              {#if bans.pageUrl}<button class="link" onclick={() => api.openUrl(bans.pageUrl!)}>{t("vPhase.view")}</button>{/if}
            {:else}
              {#each bans.basedOn as a (a.championId)}<img class="mini" class:me={a.me} src={championIcon(a.championId)} alt="" title={app.champName(a.championId)} />{/each}
              {t("vPhase.dropsYourWr")}
            {/if}
          </p>
          {#if bans.picks.length}<BanList list={bans.picks} variant="cards" />{:else}<p class="muted small">{bans.note ?? "—"}</p>{/if}
        </section>
        <section>
          <h2>{t("views.vsYourPick")}</h2>
          <p class="muted small sub">
            {#if me}<img class="mini me" src={championIcon(me.championId)} alt="" /> {me.lane
              ? t("vPhase.badMatchupLane", { champ: app.champName(me.championId), lane: laneName(me.lane) })
              : t("vPhase.badMatchup", { champ: app.champName(me.championId) })}{:else}&nbsp;{/if}
          </p>
          {#if bans.lanePicks.length}<BanList list={bans.lanePicks} variant="cards" />{:else}<p class="muted small">{bans.laneNote ?? "—"}</p>{/if}
        </section>
      {:else}
        <p class="muted empty">{t("vPhase.noBans")}</p>
      {/if}
    </main>
  {:else if step === "picks"}
    <main class="picks">
      {#if app.snap?.picks}
        {@const p = app.snap.picks}
        {@const laneLabel = laneName(p.lane)}
        <h2>{laneLabel ? t("vPhase.forLane", { lane: laneLabel }) : t("vPhase.forYourPosition")}
          <span class="muted small">· {p.enemies.length
            ? t(p.pool === "tier" ? "vPhase.poolTierVs" : "vPhase.poolOwnVs", { enemies: p.enemies.map(app.champName).join(", ") })
            : t(p.pool === "tier" ? "vPhase.poolTier" : "vPhase.poolOwn")}</span>
          <MetricToggle />
          {#if p.loading}<span class="muted small">{t("common.calculating")}</span>{/if}</h2>
        <PickList variant="cards" />
        <p class="muted small">{t("vPhase.pickHint")}</p>
      {:else}
        <p class="muted empty">{t("vPhase.picksOff")}</p>
      {/if}
      {#if app.composition || app.snap?.synergy?.lines.length}
        <div class="bottom">
          {#if app.composition}<DamageMix columns />{/if}
          {#if app.snap?.synergy?.lines.length}
            <div class="syn">
              <span class="muted small">{t("vPhase.synergyOf", { champ: app.champName(app.snap.synergy.championId) })}</span>
              {#each app.snap.synergy.lines as l (l.allyChampionId)}
                <span class="small"><img class="mini" src={championIcon(l.allyChampionId)} alt="" /> {t("views.withAlly", { champ: app.champName(l.allyChampionId) })} <b class:up={l.delta > 0} class:down={l.delta < 0}>{pct(l.wr)}</b></span>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </main>
  {:else if !b}
    <main><p class="muted empty">{app.snap?.inChampSelect ? t("common.emptyLocked") : t("common.emptyIdle")}</p></main>
  {:else}
    <main class="build">
      <section class="left">
        <div class="hero">
          <img src={championIcon(app.snap!.championId)} alt="" />
          <div>
            <div class="bname">{app.champ?.name ?? b.slug}</div>
            <div class="muted small">
              {#if app.lanes.length > 1}
                <select class="lane" value={b.lane} onchange={(e) => app.changeLane(e.currentTarget.value)}>
                  {#each app.lanes as [lane, x] (lane)}<option value={lane}>{laneName(lane)} {Math.round(x)}%</option>{/each}
                </select>
              {:else}{laneName(b.lane)}{/if}
              · WR <b class="txt">{pct(b.wr)}</b>{#if b.rank} · #{b.rank}{/if} · {b.patch}
            </div>
          </div>
        </div>
        <RunePage size={28} layout="key" />
        <div class="spells">
          {#each (b.spells[0]?.ids ?? []) as id (id)}<img src={icon(app.assets?.spells[id]?.icon)} alt="" title={app.assets?.spells[id]?.name} />{/each}
          {#if b.skillPriority[0]}<span class="prio" title="{t('common.skillOrder')} · {pct(b.skillPriority[0].wr)}">{b.skillPriority[0].ids.join(" › ")}</span>{/if}
        </div>
        <ImportButton big />
      </section>
      <section class="right">
        <h2>{t("vPhase.path")} <MetricToggle /></h2>
        <div class="path">
          {#if b.startSets[0]}
            <div class="pstep"><span class="lbl">{t("common.start")} · {app.startText(0)}</span>
              <div class="icons">{#each b.startSets[0].ids as id, i (i)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div></div>
          {/if}
          {#if app.firstBackItems.length}
            <span class="arrow">›</span>
            <div class="pstep"><span class="lbl">{t("common.firstBack")}</span>
              <div class="icons">{#each app.firstBackItems.slice(0, 2) as it (it.id)}<img src={icon(item(it.id)?.icon)} alt="" title="{item(it.id)?.name} · {pct(it.wr)}" />{/each}</div></div>
          {/if}
          {#if core}
            <span class="arrow">›</span>
            <div class="pstep"><span class="lbl">{t("common.core")} · {app.coreText(core.combo)}</span>
              <div class="icons">{#each core.ids as id (id)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div></div>
          {/if}
        </div>
        <h2>{t("vPhase.later")}</h2>
        <div class="table">
          {#each [[t("common.boots"), b.boots, b.boots, true], [t("common.itemN", { n: 4 }), b.slots[3] ?? [], b.slots[3] ?? [], true], [t("common.itemN", { n: 5 }), b.slots[4] ?? [], b.slots[4] ?? [], true], [t("common.situationalShort"), app.situational, b.winningItems ?? [], false]] as const as [label, list, from, slot] (label)}
            {#if list.length}
              <span class="lbl">{label}</span>
              <div class="icons">{#each list.filter((i) => item(i.id)).slice(0, 3) as it (it.id)}<Item {it} {from} {slot} size={32} />{/each}</div>
            {/if}
          {/each}
        </div>
        {#if app.composition?.need}<DamageMix mode="def" />{/if}
        <div class="foot">
          <VsPicker size={26} />
          {#if app.snap?.pageUrl}<button class="link" onclick={() => api.openUrl(app.snap!.pageUrl!)}>lolalytics ↗</button>{/if}
        </div>
      </section>
    </main>
  {/if}
</div>

<style>
  .phase-layout { height: 100vh; display: grid; grid-template-rows: 48px auto 1fr; overflow: hidden; }
  ol { display: flex; gap: 4px; list-style: none; margin: 0; padding: 0; }
  ol button { display: flex; align-items: center; gap: 6px; border: none; background: none; color: var(--muted); padding: 4px 10px; border-radius: 14px; }
  .n { width: 18px; height: 18px; border-radius: 50%; border: 1px solid var(--muted); display: grid; place-items: center; font-size: 11px; }
  li.done .n { border-color: var(--good); color: var(--good); }
  li.on button { color: var(--gold); background: #1e282d; }
  li.on .n { border-color: var(--gold); }
  main { min-height: 0; overflow: hidden; padding: 14px 16px; }
  h2 { margin: 0 0 8px; font-size: 13px; color: var(--gold); text-transform: uppercase; letter-spacing: 0.04em; display: flex; gap: 8px; align-items: baseline; flex-wrap: wrap; }
  h2 .small { text-transform: none; letter-spacing: 0; font-weight: normal; }
  .sub { display: flex; align-items: center; gap: 4px; margin: -4px 0 8px; min-height: 18px; }
  .mini { width: 18px; height: 18px; border-radius: 3px; vertical-align: middle; }
  .mini.me { box-shadow: 0 0 0 1px var(--gold); }
  .bans { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  .picks { display: grid; grid-template-rows: auto auto auto 1fr; gap: 8px; align-content: start; }
  .picks p { margin: 0; }
  .bottom { display: grid; gap: 10px; padding-top: 12px; border-top: 1px solid var(--line); align-self: start; }
  .syn { display: flex; gap: 12px; align-items: center; flex-wrap: wrap; }
  .build { display: grid; grid-template-columns: 250px 1fr; gap: 18px; }
  .left { display: grid; align-content: start; gap: 14px; padding-inline-end: 16px; border-inline-end: 1px solid var(--line); }
  .right { display: grid; align-content: start; min-width: 0; }
  .hero { display: flex; gap: 10px; align-items: center; }
  .hero img { width: 52px; height: 52px; border-radius: 6px; border: 2px solid var(--gold); }
  .bname { font-weight: 600; font-size: 16px; }
  .lane { font-size: 11px; padding: 0 2px; color: var(--gold); }
  .txt { color: var(--text); }
  .spells { display: flex; gap: 6px; align-items: center; }
  .spells img { width: 34px; height: 34px; border-radius: 4px; }
  .prio { margin-inline-start: 8px; font-size: 16px; font-weight: 600; color: var(--gold); letter-spacing: 0.06em; }
  .path { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; flex-wrap: wrap; }
  .pstep { display: grid; gap: 4px; }
  .arrow { color: var(--gold-dim); font-size: 26px; }
  .arrow:dir(rtl) { display: inline-block; transform: scaleX(-1); }
  .lbl { font-size: 11px; color: var(--muted); }
  .icons { display: flex; gap: 5px; }
  .icons img { width: 38px; height: 38px; border-radius: 4px; border: 1px solid var(--gold-dim); }
  .table { display: grid; grid-template-columns: 60px 1fr; align-items: center; gap: 6px 8px; margin-bottom: 12px; }
  .table .icons { gap: 10px; }
  .foot { display: flex; align-items: center; gap: 10px; margin-top: 10px; flex-wrap: wrap; }
  .empty { padding: 40px 24px; text-align: center; }
  .small { font-size: 11px; }
</style>
