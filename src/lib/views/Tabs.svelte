<script lang="ts">
  // Layout "Abas": cabeçalho fixo com o campeão e o Importar; o corpo é uma aba por vez
  // (Draft · Build · Runas · Confronto). A aba segue a etapa da seleção até você clicar em outra.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { championIcon, icon, pct, num, LANES, coreSets, firstBack, situationalItems } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import Search from "./parts/Search.svelte";
  import Notices from "./parts/Notices.svelte";
  import BanList from "./parts/BanList.svelte";
  import PickList from "./parts/PickList.svelte";
  import MetricToggle from "./parts/MetricToggle.svelte";
  import { banExplain } from "$lib/metric";
  import RunePage from "./parts/RunePage.svelte";
  import SkillGrid from "./parts/SkillGrid.svelte";
  import Item from "./parts/Item.svelte";
  import ImportButton from "./parts/ImportButton.svelte";
  import VsPicker from "./parts/VsPicker.svelte";
  import DamageMix from "./parts/DamageMix.svelte";
  import Trend from "$lib/Trend.svelte";
  import DamageBar from "$lib/DamageBar.svelte";
  import RuneAlternatives from "$lib/RuneAlternatives.svelte";
  import { PHASES } from "$lib/api";

  type Tab = "draft" | "build" | "runas" | "confronto";
  const TABS: [Tab, string][] = [["draft", "Draft"], ["build", "Build"], ["runas", "Runas"], ["confronto", "Confronto"]];
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
  const bans = $derived(app.snap?.bans ?? null);
  const core = $derived(b ? coreSets(b, app.assets, app.minGames)[0] : undefined);
  const item = (id: number | string) => app.assets?.items[String(id)];
</script>

<div class="tabs-layout">
  <header>
    {#if b && app.snap}
      <img class="portrait" src={championIcon(app.snap.championId)} alt="" />
      <div class="who">
        <div class="title">
          <b>{app.champ?.name ?? b.slug}</b>
          {#if b.lanes.filter(([, p]) => p >= 1).length > 1}
            <select class="lane" value={b.lane} onchange={(e) => app.changeLane(e.currentTarget.value)}>
              {#each b.lanes.filter(([, p]) => p >= 1) as [lane, p] (lane)}<option value={lane}>{LANES[lane] ?? lane} {Math.round(p)}%</option>{/each}
            </select>
          {:else}<span class="muted">{LANES[b.lane] ?? b.lane}</span>{/if}
          <span class="muted small">{b.patch}</span>
        </div>
        <div class="muted small">WR <b class="txt">{pct(b.wr)}</b> · PR {pct(b.pr)} · BR {pct(b.br)}{#if b.rank} · #{b.rank}/{b.rankTotal}{/if} · {num(b.n)} partidas</div>
      </div>
    {:else}
      <div class="who"><div class="title"><b>Lobby Companion</b></div><div class="muted small">{app.snap?.inChampSelect ? "Trave um campeão para ver o build." : "Busque um campeão para ver o build."}</div></div>
    {/if}
    <div class="right">
      <span class="phase"><span class="dot" class:on={app.snap?.connected}></span>{#if !app.snap?.connected}Cliente fechado{:else}{PHASES[app.snap.phase] ?? app.snap.phase}{/if}</span>
      <ImportButton labels={false} />
    </div>
  </header>

  <nav>
    {#each TABS as [id, label] (id)}
      <button class:on={tab === id} onclick={() => (chosen = id)}>{label}</button>
    {/each}
    <span class="grow"></span>
    <Search width={120} />
    <button class="icon" title="Configurações" onclick={() => (app.settingsOpen = true)}>⚙</button>
  </nav>
  <Notices />

  <main>
    {#if tab === "draft"}
      <div class="two">
        <section>
          <h2>Bans {#if bans}<MetricToggle />{/if} {#if bans?.loading}<span class="muted">calculando…</span>{/if}</h2>
          {#if bans}
            <div class="bans">
              <div>
                <h3>{bans.byTier ? "Tier list da lane" : "Pelo time"}</h3>
                {#if bans.picks.length}<BanList list={bans.picks} />{:else}<p class="muted small">{bans.note ?? "—"}</p>{/if}
              </div>
              <div>
                <h3>Contra você</h3>
                {#if bans.lanePicks.length}<BanList list={bans.lanePicks} />{:else}<p class="muted small">{bans.laneNote ?? "—"}</p>{/if}
              </div>
            </div>
            <p class="muted small">{banExplain(bans.metric, bans.byTier)} Passe o mouse para ver o porquê.</p>
          {:else}
            <p class="muted small">Os bans aparecem na seleção de campeões.</p>
          {/if}
        </section>
        <section>
          {#if app.snap?.inChampSelect && app.snap.picks}
            <h2>Pick para {LANES[app.snap.picks.lane ?? ""] ?? "sua posição"} <MetricToggle /></h2>
            <PickList variant="detailed" />
          {/if}
          {#if app.snap?.synergy?.lines.length}
            <h2>Sinergia</h2>
            {#each app.snap.synergy.lines as l (l.allyChampionId)}
              <div class="syn"><img src={championIcon(l.allyChampionId)} alt="" /> com {app.champName(l.allyChampionId)} <b class:up={l.delta > 0} class:down={l.delta < 0}>{pct(l.wr)}</b></div>
            {/each}
          {/if}
          {#if app.composition}
            <h2>Dano</h2>
            <DamageMix mode="enemy" />
          {/if}
        </section>
      </div>
    {:else if !b}
      <p class="muted empty">{app.snap?.inChampSelect ? "Trave um campeão: runas, feitiços e o conjunto de itens entram sozinhos." : "A janela abre sozinha na seleção de campeões. Enquanto isso, busque um campeão acima."}</p>
    {:else if tab === "build"}
      <div class="two">
        <section>
          <h2>Itens</h2>
          {#if b.startSets[0]}
            <div class="irow"><span class="lbl">Início</span>
              <div class="icons">{#each b.startSets[0].ids as id, i (i)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
              <span class="small">{pct(b.startSets[0].wr)}</span></div>
          {/if}
          {#if firstBack(b, app.assets, app.minGames).length}
            <div class="irow"><span class="lbl">1ª volta</span>
              <div class="icons">{#each firstBack(b, app.assets, app.minGames).slice(0, 4) as it (it.id)}<img src={icon(item(it.id)?.icon)} alt="" title="{item(it.id)?.name} · {pct(it.wr)}" />{/each}</div></div>
          {/if}
          {#if core}
            <div class="irow"><span class="lbl">Núcleo</span>
              <div class="icons">{#each core.ids as id (id)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
              <span class="small">{pct(core.combo.wr)}</span></div>
          {/if}
          <div class="slots">
            {#each [["Botas", b.boots], ["4º item", b.slots[3] ?? []], ["5º item", b.slots[4] ?? []]] as const as [label, list] (label)}
              <div class="slot">
                <span class="lbl">{label}</span>
                <div class="icons">{#each list.filter((i) => item(i.id)).slice(0, 3) as it (it.id)}<Item {it} size={26} />{/each}</div>
              </div>
            {/each}
          </div>
          {#if situationalItems(b, app.assets, app.minGames).length}
            <div class="irow"><span class="lbl">Situac.</span>
              <div class="icons">{#each situationalItems(b, app.assets, app.minGames) as it (it.id)}<Item {it} size={26} />{/each}</div></div>
          {/if}
        </section>
        <section>
          <h2>Feitiços</h2>
          {#each b.spells.slice(0, 2) as c, i (c.ids.join())}
            <div class="spells" class:alt={i > 0}>
              {#each c.ids as id (id)}<img src={icon(app.assets?.spells[id]?.icon)} alt="" title={app.assets?.spells[id]?.name} />{/each}
              <span class="small">{pct(c.wr)}</span><span class="muted small">{pct(c.pr)}</span>
            </div>
          {/each}
          <h2>Habilidades</h2>
          <SkillGrid cell={15} numbers />
          {#if b.trend}<h2>Tendência</h2><Trend trend={b.trend} />{/if}
          {#if b.damage}<DamageBar damage={b.damage} />{/if}
          {#if app.snap?.pageUrl}<button class="link more" onclick={() => openUrl(app.snap!.pageUrl!)}>Ver no lolalytics ↗</button>{/if}
        </section>
      </div>
    {:else if tab === "runas"}
      <div class="two">
        <section>
          <h2>Página</h2>
          <RunePage size={32} />
        </section>
        <section class="alts">
          <h2>Alternativas</h2>
          {#if app.runeSet}<RuneAlternatives set={app.runeSet} stats={b.runeStats} assets={app.assets} minGames={app.minGames} open />{/if}
        </section>
      </div>
    {:else}
      <div class="two">
        <section>
          <h2>Build contra</h2>
          <VsPicker full size={40} />
        </section>
        <section>
          {#if app.composition}<h2>Composição</h2><DamageMix />{/if}
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
  .right { margin-left: auto; display: grid; justify-items: end; gap: 4px; }
  .phase { font-size: 11px; color: var(--muted); display: flex; align-items: center; gap: 5px; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--bad); }
  .dot.on { background: var(--good); }
  nav { display: flex; align-items: center; gap: 2px; padding: 0 8px; border-bottom: 1px solid var(--gold-dim); }
  nav > button:not(.icon) { align-self: stretch; border: none; border-bottom: 2px solid transparent; border-radius: 0; background: none; color: var(--muted); padding: 0 12px; }
  nav > button.on { color: var(--gold); border-bottom-color: var(--gold); }
  .icon { width: 28px; height: 28px; padding: 0; font-size: 15px; margin-left: 6px; }
  .grow { flex: 1; }
  main { min-height: 0; overflow: hidden; }
  .two { display: grid; grid-template-columns: 1fr 1fr; height: 100%; }
  section { padding: 10px 14px; overflow: hidden; min-width: 0; }
  .two section:first-child { border-right: 1px solid var(--line); }
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
  .up { color: var(--good); }
  .down { color: var(--bad); }
  .muted { color: var(--muted); }
  .small { font-size: 11px; }
</style>
