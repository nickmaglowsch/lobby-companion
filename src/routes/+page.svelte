<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    api, championIcon, icon, pct, num, LANES, PHASES, TIERS, REGIONS, PATCHES,
    type Assets, type Settings, type Snapshot,
  } from "$lib/api";
  import Runes from "$lib/Runes.svelte";
  import ItemRow from "$lib/ItemRow.svelte";

  let snap = $state<Snapshot | null>(null);
  let assets = $state<Assets | null>(null);
  let settings = $state<Settings | null>(null);
  let runeTab = $state<"pick" | "win">("pick");
  let search = $state("");
  let importing = $state(false);
  let importError = $state<string | null>(null);

  const build = $derived(snap?.build ?? null);
  const champ = $derived(snap && assets ? assets.champions[snap.championId] : undefined);
  const champions = $derived(assets ? Object.values(assets.champions).sort((a, b) => a.name.localeCompare(b.name, "pt-BR")) : []);
  const runeSet = $derived(build ? (runeTab === "win" ? build.runesWin ?? build.runesPick : build.runesPick) : null);
  const minGames = $derived(settings?.minGames ?? 200);
  const skillLetters = ["Q", "W", "E", "R"];

  async function refreshAssets() {
    if (!assets) assets = await api.assets();
  }

  onMount(() => {
    let un: (() => void) | undefined;
    (async () => {
      snap = await api.snapshot();
      settings = await api.settings();
      if (settings) runeTab = settings.runeChoice;
      await refreshAssets();
      un = await api.onSnapshot((s) => {
        snap = s;
        if (s.connected) refreshAssets();
      });
    })();
    return () => un?.();
  });

  function pickChampion() {
    const q = search.trim().toLocaleLowerCase("pt-BR");
    const c = champions.find((c) => c.name.toLocaleLowerCase("pt-BR") === q)
      ?? champions.find((c) => c.name.toLocaleLowerCase("pt-BR").startsWith(q));
    if (c) {
      api.lookup(c.id, null);
      search = "";
    }
  }

  function changeLane(lane: string) {
    if (snap?.championId) api.lookup(snap.championId, lane);
  }

  async function importNow() {
    importing = true;
    importError = null;
    try {
      await api.reimport();
    } catch (e) {
      importError = String(e);
    } finally {
      importing = false;
    }
  }

  async function save() {
    if (settings) await api.saveSettings($state.snapshot(settings));
  }
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
        <h2>Runas</h2>
        <div class="tabs">
          <button class:active={runeTab === "pick"} onclick={() => (runeTab = "pick")}>Mais jogada</button>
          <button class:active={runeTab === "win"} onclick={() => (runeTab = "win")}>Maior WR</button>
        </div>
      </div>
      {#if runeSet}<Runes set={runeSet} {assets} />{:else}<p class="muted">Sem runas.</p>{/if}
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
      <ItemRow label="Botas" items={build.boots} {assets} avgWr={build.wr} {minGames} />
      {#each build.slots as slot, i (i)}
        <ItemRow label="{i + 1}º item" items={slot} {assets} avgWr={build.wr} {minGames} />
      {/each}
      <p class="muted small">Verde/vermelho = ≥1 ponto acima/abaixo do WR do campeão. Apagado = menos de {minGames} partidas.</p>
    </section>
  {/if}

  {#if settings}
    <details class="card settings">
      <summary>Configurações</summary>
      <div class="grid">
        <label>Elo <select bind:value={settings.tier} onchange={save}>{#each TIERS as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
        <label>Região <select bind:value={settings.region} onchange={save}>{#each REGIONS as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
        <label>Período <select bind:value={settings.patch} onchange={save}>{#each PATCHES as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
        <label>Runas <select bind:value={settings.runeChoice} onchange={save}><option value="pick">Mais jogada</option><option value="win">Maior WR</option></select></label>
        <label>Amostra mínima <input type="number" min="0" step="50" bind:value={settings.minGames} onchange={save} /></label>
        <label>Flash na tecla <select bind:value={settings.flashSlot} onchange={save}><option value={1}>D</option><option value={2}>F</option></select></label>
      </div>
      <div class="checks">
        <label><input type="checkbox" bind:checked={settings.autoImport} onchange={save} /> Importar ao travar o campeão</label>
        <label><input type="checkbox" bind:checked={settings.importRunes} onchange={save} /> Página de runas</label>
        <label><input type="checkbox" bind:checked={settings.importItems} onchange={save} /> Conjunto de itens (WR no título dos blocos)</label>
        <label><input type="checkbox" bind:checked={settings.importSpells} onchange={save} /> Feitiços</label>
        <label><input type="checkbox" bind:checked={settings.replaceCurrentPage} onchange={save} /> Sem espaço para página nova: substituir a página atual</label>
        <label><input type="checkbox" bind:checked={settings.closeInGame} onchange={save} /> Fechar a janela quando a partida começar</label>
      </div>
    </details>
  {/if}

  <footer class="muted small">Dados: lolalytics.com (ranqueada solo). Não afiliado à Riot Games nem ao lolalytics.</footer>
</main>

<style>
  :global(:root) {
    --bg: #010a13; --panel: #0a1428; --panel-2: #091428; --line: #1e2328;
    --text: #f0e6d2; --muted: #a09b8c; --gold: #c8aa6e; --gold-dim: #785a28;
    --good: #2fbf71; --bad: #e84057; --warn: #e5a93b;
    color-scheme: dark;
  }
  :global(body) { margin: 0; background: var(--bg); color: var(--text); font: 14px/1.4 "Segoe UI", system-ui, sans-serif; }
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
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin: 10px 0; }
  .grid label { display: grid; gap: 2px; font-size: 12px; color: var(--muted); }
  .checks { display: grid; gap: 4px; font-size: 13px; }
  footer { text-align: center; }
</style>
