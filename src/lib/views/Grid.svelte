<script lang="ts">
  // Layout "Grade": três colunas fixas (draft · runas · itens), tudo numa tela, sem rolagem.
  // A coluna da etapa atual da seleção fica destacada; detalhes ficam no tooltip.
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { icon, pct, LANES, coreSets, firstBack, situationalItems } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import TopBar from "./parts/TopBar.svelte";
  import ChampLine from "./parts/ChampLine.svelte";
  import Notices from "./parts/Notices.svelte";
  import BanList from "./parts/BanList.svelte";
  import PickList from "./parts/PickList.svelte";
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
        <h2>Bans <span class="muted">{bans.byTier ? `tier list${bans.lane ? ` · ${LANES[bans.lane] ?? bans.lane}` : ""}` : "pelo time"}</span>
          {#if bans.loading}<span class="muted">…</span>{/if}</h2>
        {#if bans.picks.length}<BanList list={bans.picks} variant="strip" />{:else if bans.note}<p class="muted small">{bans.note}</p>{/if}
        <h2>Bans <span class="muted">contra você</span></h2>
        {#if bans.lanePicks.length}<BanList list={bans.lanePicks} variant="strip" />{:else}<p class="muted small">{bans.laneNote ?? "—"}</p>{/if}
      {/if}
      {#if app.snap?.inChampSelect && app.snap.picks}
        <h2>Pick <span class="muted">{LANES[app.snap.picks.lane ?? ""] ?? ""} · {app.snap.picks.pool === "tier" ? "tier list" : "seu pool"}</span></h2>
        <PickList variant="rows" max={3} />
      {/if}
      {#if app.composition}
        <h2>Dano</h2>
        <DamageMix mode="enemy" />
      {/if}
      {#if !bans && !app.snap?.picks && !app.composition}
        <p class="muted small empty">Na seleção de campeões aparecem aqui os bans, as sugestões de pick e a composição dos times.</p>
      {/if}
    </section>

    <section class:focus={app.stage === "build"}>
      {#if b}
        <h2>Runas</h2>
        <RunePage size={27} />
        <h2>Feitiços</h2>
        {#each b.spells.slice(0, 2) as c, i (c.ids.join())}
          <div class="spells" class:alt={i > 0}>
            {#each c.ids as id (id)}<img src={icon(app.assets?.spells[id]?.icon)} alt={app.assets?.spells[id]?.name} title={app.assets?.spells[id]?.name} />{/each}
            <span class="small">{pct(c.wr)}</span><span class="muted small">{pct(c.pr)}</span>
          </div>
        {/each}
        <h2>Habilidades</h2>
        <SkillGrid cell={11} />
      {:else}
        <p class="muted small empty">{app.snap?.inChampSelect ? "Trave um campeão: runas, feitiços e o conjunto de itens entram sozinhos." : "Busque um campeão acima para ver o build."}</p>
      {/if}
    </section>

    <section class:focus={app.stage === "build"}>
      {#if b}
        <h2>Itens</h2>
        {#if b.startSets[0]}
          <div class="irow">
            <span class="lbl">Início</span>
            <div class="icons">{#each b.startSets[0].ids as id, i (i)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
            <span class="small">{pct(b.startSets[0].wr)}</span>
          </div>
        {/if}
        {#if firstBack(b, app.assets, app.minGames).length}
          <div class="irow">
            <span class="lbl">1ª volta</span>
            <div class="icons">{#each firstBack(b, app.assets, app.minGames).slice(0, 3) as it (it.id)}<img src={icon(item(it.id)?.icon)} alt="" title="{item(it.id)?.name} · {pct(it.wr)}" />{/each}</div>
          </div>
        {/if}
        {#if core}
          <div class="irow">
            <span class="lbl">Núcleo</span>
            <div class="icons">{#each core.ids as id (id)}<img src={icon(item(id)?.icon)} alt="" title={item(id)?.name} />{/each}</div>
            <span class="small">{pct(core.combo.wr)}</span>
          </div>
        {/if}
        {#each [["Botas", b.boots], ["4º item", b.slots[3] ?? []], ["5º item", b.slots[4] ?? []]] as const as [label, list] (label)}
          {#if list.length}
            <div class="irow">
              <span class="lbl">{label}</span>
              <div class="icons">{#each list.filter((i) => item(i.id)).slice(0, 3) as it (it.id)}<Item {it} size={28} />{/each}</div>
            </div>
          {/if}
        {/each}
        {#if situationalItems(b, app.assets, app.minGames).length}
          <div class="irow">
            <span class="lbl">Situac.</span>
            <div class="icons">{#each situationalItems(b, app.assets, app.minGames).slice(0, 3) as it (it.id)}<Item {it} size={28} />{/each}</div>
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
  section { padding: 8px 12px; border-right: 1px solid var(--line); overflow: hidden; opacity: 0.75; transition: opacity 0.2s; }
  section:last-child { border-right: none; }
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
