<script lang="ts">
  // Layout "Clássico": a coluna completa com rolagem (a interface original).
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    api, championIcon, icon, pct, num, LANES, PHASES, firstBack, situationalItems, type BanPick, type RuneChoice,
  } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import Runes from "$lib/Runes.svelte";
  import ItemRow from "$lib/ItemRow.svelte";
  import Trend from "$lib/Trend.svelte";
  import DamageBar from "$lib/DamageBar.svelte";
  import CoreBuild from "$lib/CoreBuild.svelte";
  import SkillPaths from "$lib/SkillPaths.svelte";
  import RuneAlternatives from "$lib/RuneAlternatives.svelte";
  import Composition from "$lib/Composition.svelte";
  import PickSuggest from "$lib/PickSuggest.svelte";
  import SettingsForm from "./parts/SettingsForm.svelte";
  import OwnRunes from "./parts/OwnRunes.svelte";

  const snap = $derived(app.snap);
  const assets = $derived(app.assets);
  const build = $derived(app.build);
  const champ = $derived(app.champ);
  const champions = $derived(app.champions);
  const runeTab = $derived(app.runeTab);
  const runeSet = $derived(app.runeSet);
  const minGames = $derived(app.minGames);
  const vsLane = $derived(app.vsLane);
  const importing = $derived(app.importing);
  const importError = $derived(app.importError);
  const defHighlight = $derived(app.defHighlight);
  const skillLetters = ["Q", "W", "E", "R"];
  const champName = (id: number) => app.champName(id);
  const signed = (x: number) => `+${x.toFixed(2).replace(".", ",")}`;
  let search = $state("");
  let vsSearch = $state("");

  const findChampion = (text: string) => app.findChampion(text);
  function chooseVs(id: number | null) {
    vsSearch = "";
    app.chooseVs(id);
  }
  const changeVsLane = (lane: string) => app.changeVsLane(lane);
  function pickChampion() {
    if (app.search(search)) search = "";
  }
  const changeLane = (lane: string) => app.changeLane(lane);
  const importNow = () => app.importNow();
  const chooseRunes = (c: RuneChoice) => app.chooseRunes(c);
</script>

<main>
  <header>
    <span class="dot" class:on={snap?.connected}></span>
    <span class="status">
      {#if !snap?.connected}Cliente do League fechado{:else}{PHASES[snap.phase] ?? snap.phase}{/if}
    </span>
    <form class="search" onsubmit={(e) => { e.preventDefault(); pickChampion(); }}>
      <input list="champs" placeholder="Buscar campeão…" bind:value={search} disabled={!assets} />
      <datalist id="champs">
        {#each champions as c (c.id)}<option value={c.name}></option>{/each}
      </datalist>
    </form>
  </header>

  {#if snap?.error}<p class="msg bad">{snap.error}</p>{/if}
  {#if snap?.warning}<p class="msg warn">{snap.warning}</p>{/if}

  {#if !build && !snap?.loading}
    <section class="empty">
      {#if snap?.inChampSelect}
        <p>Trave um campeão: runas, feitiços e o conjunto de itens entram sozinhos.</p>
      {:else}
        <p>A janela abre sozinha na seleção de campeões e fecha quando a partida começa.</p>
        <p class="muted">Enquanto isso, busque um campeão acima para ver o build.</p>
      {/if}
    </section>
  {/if}

  {#if snap?.loading}<p class="msg">Buscando no lolalytics…</p>{/if}

  {#if snap?.bans}
    {@const bans = snap.bans}
    <section class="card">
      <div class="card-head">
        <h2>Sugestão de bans</h2>
        {#if bans.loading}<span class="muted small">calculando…</span>{/if}
      </div>
      {#snippet banList(list: BanPick[])}
        {#each list as p (p.championId)}
          <div class="ban">
            <img class="ban-icon" src={championIcon(p.championId)} alt={champName(p.championId)} />
            <div>
              <div><b>{champName(p.championId)}</b> <span class="gain" title="pontos de win rate recuperados, em média, sem ele no jogo">{signed(p.gain)} pts</span></div>
              {#if p.tier}
                <div class="reason muted small">{pct(p.tier.wr)} na lane · pick {pct(p.tier.pr)} · ban {pct(p.tier.br)} · {num(p.tier.games)} partidas</div>
              {/if}
              {#each p.reasons as r (r.allyChampionId + r.lane)}
                <div class="reason muted small">
                  <img class="mini" src={championIcon(r.allyChampionId)} alt={champName(r.allyChampionId)} />
                  {champName(r.allyChampionId)}: {pct(r.wr)} contra ele ({LANES[r.lane] ?? r.lane}) · geral {pct(r.baseWr)} · {num(r.n)} partidas
                </div>
              {/each}
            </div>
          </div>
        {/each}
      {/snippet}

      <h3 class="ban-group">Pelo time</h3>
      {#if bans.byTier}
        <div class="based muted small">Ninguém do time declarou ainda: pela força geral na sua lane{bans.lane ? ` (${LANES[bans.lane] ?? bans.lane})` : ""}.</div>
      {/if}
      {#if bans.basedOn.length}
        <div class="based muted small">
          {snap.inChampSelect ? "Campeões do seu time:" : "Para:"}
          {#each bans.basedOn as a (a.championId)}
            <img class="mini" class:me={a.me} src={championIcon(a.championId)} alt={champName(a.championId)}
              title="{champName(a.championId)}{a.lane ? ` (${LANES[a.lane] ?? a.lane})` : ''}{a.me ? ' · você' : ''}" />
          {/each}
        </div>
      {/if}
      {@render banList(bans.picks)}
      {#if bans.note}<p class="muted small">{bans.note}</p>{/if}

      <h3 class="ban-group">
        Contra o seu pick{#if bans.laneFor}: <img class="mini me" src={championIcon(bans.laneFor.championId)} alt={champName(bans.laneFor.championId)} />
        {champName(bans.laneFor.championId)}{bans.laneFor.lane ? ` (${LANES[bans.laneFor.lane] ?? bans.laneFor.lane})` : ""}{/if}
      </h3>
      {#if bans.lanePicks.length}
        <div class="based muted small">Oponentes da sua lane que vão melhor contra o seu campeão e aparecem com frequência (sem repetir os de cima).</div>
      {/if}
      {@render banList(bans.lanePicks)}
      {#if bans.laneNote}<p class="muted small">{bans.laneNote}</p>{/if}
      {#if bans.picks.length && bans.byTier}
        <p class="muted small">Pts = win rate acima da média da lane × chance de ele cair na sua lane (pick rate); amostra pequena conta menos. Troca sozinho para a conta pelo time quando alguém declarar.
          {#if bans.pageUrl}<button class="link" onclick={() => openUrl(bans.pageUrl!)}>Tier list ↗</button>{/if}
        </p>
      {:else if bans.picks.length || bans.lanePicks.length}
        <p class="muted small">Pts = win rate recuperado, em média, sem esse campeão no jogo: frequência dele × queda do WR (amostra pequena conta menos). Tendência estatística, não leitura do outro time.</p>
      {/if}
    </section>
  {/if}

  {#if snap?.inChampSelect && (snap.picks || snap.synergy)}
    <PickSuggest advice={snap.picks} synergy={snap.synergy} {assets} locked={snap.locked} onPick={(id, lane) => api.lookup(id, lane)} />
  {/if}

  {#if build && snap}
    <section class="hero">
      <img class="champ" src={championIcon(snap.championId)} alt={champ?.name} />
      <div>
        <h1>{champ?.name ?? build.slug} <small>{LANES[build.lane] ?? build.lane} · {build.patch}</small></h1>
        <div class="stats">
          <span>WR <b>{pct(build.wr)}</b></span>
          <span>PR {pct(build.pr)}</span>
          <span>BR {pct(build.br)}</span>
          {#if build.rank}<span>#{build.rank}/{build.rankTotal}</span>{/if}
          <span class="muted">{num(build.n)} partidas</span>
        </div>
        <div class="lanes">
          {#each build.lanes.filter(([, p]) => p >= 1) as [lane, p] (lane)}
            <button class:active={lane === build.lane} onclick={() => changeLane(lane)}>{LANES[lane] ?? lane} {Math.round(p)}%</button>
          {/each}
        </div>
        <Trend trend={build.trend} />
        <DamageBar damage={build.damage} />
      </div>
    </section>

    <section class="actions">
      <button class="primary" onclick={importNow} disabled={importing || !snap.connected}>
        {importing ? "Importando…" : "Importar para o cliente"}
      </button>
      {#if snap.pageUrl}
        <button class="link" onclick={() => openUrl(snap!.pageUrl!)}>Ver no lolalytics ↗</button>
      {/if}
    </section>
    {#if importError}<p class="msg bad">{importError}</p>{/if}
    {#if snap.import}
      <div class="chips">
        {#each [["Runas", snap.import.runes], ["Itens", snap.import.items], ["Feitiços", snap.import.spells]] as [label, o] (label)}
          {#if o && typeof o === "object"}
            <span class="chip" class:ok={o.ok} class:bad={!o.ok} title={o.msg}>{o.ok ? "✓" : "✗"} {label}: {o.msg}</span>
          {/if}
        {/each}
      </div>
    {/if}

    <section class="card">
      <div class="card-head">
        <h2>Confronto</h2>
        {#if snap.vsChampionId}<button class="link" onclick={() => chooseVs(null)}>Voltar ao build geral</button>{/if}
      </div>
      {#if snap.enemyPicks.length}
        <div class="picks">
          {#each snap.enemyPicks as id (id)}
            <button class="pick" class:active={id === snap.vsChampionId} onclick={() => chooseVs(id)} title="Build contra {champName(id)}">
              <img src={championIcon(id)} alt={champName(id)} />
            </button>
          {/each}
        </div>
      {/if}
      <form class="vs-form" onsubmit={(e) => { e.preventDefault(); const c = findChampion(vsSearch); if (c) chooseVs(c.id); }}>
        <input list="champs" placeholder="Campeão do outro lado…" bind:value={vsSearch} disabled={!assets} />
        <select value={vsLane ?? ""} onchange={(e) => changeVsLane(e.currentTarget.value)} title="Lane do campeão do outro lado">
          {#each Object.entries(LANES) as [v, l] (v)}<option value={v}>{l}</option>{/each}
        </select>
      </form>
      {#if build.vs}
        <p class="vs-line">
          <img class="mini" src={championIcon(build.vs.championId)} alt={champName(build.vs.championId)} />
          Contra <b>{champName(build.vs.championId)}</b> ({LANES[build.vs.lane] ?? build.vs.lane}):
          WR <b class:up={build.wr - build.vs.usualWr >= 1} class:down={build.wr - build.vs.usualWr <= -1}>{pct(build.wr)}</b>
          <span class="muted">· geral {pct(build.vs.usualWr)} · {num(build.n)} partidas · região global</span>
        </p>
        {#if build.n < 1000}<p class="muted small">Amostra pequena: itens com menos de {minGames} partidas ficam apagados.</p>{/if}
      {:else if !snap.vsChampionId}
        <p class="muted small">Escolha o campeão do outro lado para ver runas, feitiços e itens desse confronto.</p>
      {/if}
    </section>

    <Composition />

    <section class="card">
      <div class="card-head">
        <h2>Runas</h2>
        <div class="tabs">
          {#if app.ownPage}<button class:active={runeTab === "mine"} onclick={() => chooseRunes("mine")} title="A sua página para este campeão e lane">Minha</button>{/if}
          <button class:active={runeTab === "pick"} onclick={() => chooseRunes("pick")}>Mais jogada</button>
          <button class:active={runeTab === "win"} onclick={() => chooseRunes("win")}>Maior WR</button>
        </div>
      </div>
      {#if runeSet}<Runes set={runeSet} {assets} />{:else}<p class="muted">Sem runas.</p>{/if}
      {#if runeTab === "mine" && app.ownPage}<p class="muted small own-meta">Sua página · salva em {new Date(app.ownPage.savedAt * 1000).toLocaleDateString("pt-BR")}</p>{/if}
      <OwnRunes compact={false} />
      {#if runeSet}<RuneAlternatives set={runeSet} stats={build.runeStats} {assets} {minGames} />{/if}
    </section>

    <section class="card row2">
      <div>
        <h2>Feitiços</h2>
        {#each build.spells.slice(0, 2) as c (c.ids.join())}
          <div class="combo">
            {#each c.ids as id (id)}<img src={icon(assets?.spells[id]?.icon)} alt={assets?.spells[id]?.name} title={assets?.spells[id]?.name} />{/each}
            <span>{pct(c.wr)}</span><span class="muted">{pct(c.pr)}</span>
          </div>
        {/each}
      </div>
      <div>
        <h2>Habilidades</h2>
        {#if build.skillPriority[0]}
          <p class="prio">{build.skillPriority[0].ids.join(" › ")} <span class="muted">{pct(build.skillPriority[0].wr)}</span></p>
        {/if}
        {#if build.skillOrder}
          <div class="skills">
            {#each skillLetters as k (k)}
              <div class="skill-row">
                <span class="k">{k}</span>
                {#each build.skillOrder.split("") as s, lvl (lvl)}<span class="cell" class:on={s === k}>{s === k ? lvl + 1 : ""}</span>{/each}
              </div>
            {/each}
          </div>
        {/if}
        <SkillPaths {build} {minGames} />
      </div>
    </section>

    <section class="card">
      <h2>Itens <small class="muted">WR · PR por posição na build</small></h2>
      {#each build.startSets.slice(0, 2) as s, i (i)}
        <div class="start">
          <span class="label">{i === 0 ? "Início" : "Alt."}</span>
          <div class="combo">
            {#each s.ids as id, j (j)}<img src={icon(assets?.items[id]?.icon)} alt={assets?.items[id]?.name} title={assets?.items[id]?.name} />{/each}
            <span>{pct(s.wr)}</span><span class="muted">{pct(s.pr)}</span>
          </div>
        </div>
      {/each}
      <CoreBuild {build} {assets} {minGames} />
      <ItemRow label="1ª volta" items={firstBack(build, assets, minGames)} {assets} avgWr={build.wr} {minGames} />
      <ItemRow label="Botas" items={build.boots} {assets} avgWr={build.wr} {minGames} highlight={defHighlight} />
      {#each build.slots as slot, i (i)}
        <ItemRow label="{i + 1}º item" items={slot} {assets} avgWr={build.wr} {minGames} highlight={defHighlight} />
      {/each}
      <ItemRow label="Situac." items={situationalItems(build, assets, minGames)} {assets} avgWr={build.wr} {minGames} />
      <p class="muted small">Verde/vermelho = ≥1 ponto acima/abaixo do WR do campeão. Apagado = menos de {minGames} partidas.</p>
    </section>
  {/if}

  <details class="card settings">
    <summary>Configurações</summary>
    <SettingsForm />
  </details>

</main>

<style>
  main { padding: 10px 12px 16px; display: grid; gap: 10px; }
  header { display: flex; align-items: center; gap: 8px; }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--bad); flex: none; }
  .dot.on { background: var(--good); }
  .status { white-space: nowrap; }
  .search { margin-left: auto; }
  input, select { background: var(--panel); color: var(--text); border: 1px solid var(--gold-dim); border-radius: 4px; padding: 4px 6px; font: inherit; }
  .search input { width: 160px; }
  button { background: var(--panel); color: var(--text); border: 1px solid var(--gold-dim); border-radius: 4px; padding: 4px 10px; font: inherit; cursor: pointer; }
  button:hover:not(:disabled) { border-color: var(--gold); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.active { border-color: var(--gold); color: var(--gold); }
  button.primary { background: #1e282d; border-color: var(--gold); color: var(--gold); font-weight: 600; }
  button.link { border: none; background: none; color: var(--gold); }
  .msg { margin: 0; padding: 6px 8px; border-radius: 4px; background: var(--panel); }
  .msg.bad { color: var(--bad); }
  .msg.warn { color: var(--warn); }
  .empty { padding: 24px 8px; text-align: center; }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  .hero { display: flex; gap: 12px; align-items: center; }
  .champ { width: 64px; height: 64px; border: 2px solid var(--gold); border-radius: 6px; }
  h1 { margin: 0; font-size: 20px; }
  h1 small { font-size: 13px; color: var(--muted); font-weight: normal; }
  h2 { margin: 0 0 6px; font-size: 14px; color: var(--gold); }
  .stats { display: flex; gap: 10px; flex-wrap: wrap; font-size: 13px; }
  .lanes { display: flex; gap: 4px; margin-top: 6px; flex-wrap: wrap; }
  .lanes button { padding: 1px 6px; font-size: 12px; }
  .actions { display: flex; gap: 8px; align-items: center; }
  .chips { display: flex; gap: 6px; flex-wrap: wrap; }
  .chip { font-size: 12px; padding: 2px 6px; border-radius: 10px; background: var(--panel); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chip.ok { color: var(--good); }
  .chip.bad { color: var(--bad); }
  .card { background: var(--panel); border: 1px solid var(--line); border-radius: 6px; padding: 10px; }
  .card-head { display: flex; justify-content: space-between; align-items: center; }
  .tabs { display: flex; gap: 4px; }
  .tabs button { padding: 1px 8px; font-size: 12px; }
  .own-meta { margin: 4px 0; }
  .row2 { display: grid; grid-template-columns: auto 1fr; gap: 16px; }
  .combo { display: flex; align-items: center; gap: 6px; margin-bottom: 4px; font-variant-numeric: tabular-nums; }
  .combo img { width: 28px; height: 28px; border-radius: 4px; border: 1px solid var(--gold-dim); }
  .prio { margin: 0 0 6px; font-weight: 600; }
  .skills { display: grid; gap: 2px; }
  .skill-row { display: flex; gap: 2px; }
  .k { width: 14px; color: var(--gold); font-weight: 600; font-size: 12px; }
  .cell { width: 15px; height: 15px; font-size: 9px; display: grid; place-items: center; background: var(--line); border-radius: 2px; color: var(--bg); }
  .cell.on { background: var(--gold); }
  .start { display: grid; grid-template-columns: 64px 1fr; gap: 8px; align-items: center; padding: 4px 0; }
  .start .label { color: var(--muted); font-size: 12px; }
  .settings summary { cursor: pointer; color: var(--gold); }
  .mini { width: 18px; height: 18px; border-radius: 3px; vertical-align: middle; border: 1px solid var(--gold-dim); }
  .mini.me { border-color: var(--gold); }
  .based { display: flex; align-items: center; gap: 4px; flex-wrap: wrap; margin-bottom: 6px; }
  .ban-group { margin: 10px 0 4px; font-size: 13px; color: var(--text); display: flex; align-items: center; gap: 4px; flex-wrap: wrap; }
  .ban-group:first-of-type { margin-top: 0; }
  .ban { display: grid; grid-template-columns: 36px 1fr; gap: 8px; align-items: start; padding: 5px 0; border-top: 1px solid var(--line); }
  .ban-icon { width: 36px; height: 36px; border-radius: 4px; border: 1px solid var(--bad); }
  .gain { color: var(--good); font-size: 12px; font-variant-numeric: tabular-nums; }
  .reason { display: flex; align-items: center; gap: 4px; }
  .picks { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 6px; }
  .pick { padding: 0; line-height: 0; }
  .pick img { width: 32px; height: 32px; border-radius: 3px; }
  .vs-form { display: flex; gap: 6px; }
  .vs-form input { flex: 1; min-width: 0; }
  .vs-line { margin: 8px 0 0; display: flex; align-items: center; gap: 4px; flex-wrap: wrap; }
  .up { color: var(--good); }
  .down { color: var(--bad); }
</style>
