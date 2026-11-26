<script lang="ts">
  // Páginas de runas próprias (OTP, várias por campeão): o que a escolhida muda em relação à mais
  // jogada, em pontos de WR (estatística de cada runa naquele lugar da página), e os botões de
  // criar, editar, copiar do cliente e apagar. Copiar só LÊ a página atual do cliente.
  import { icon, num, pts, runeDiffs, ownRuneSet, type RuneDiff } from "$lib/api";
  import { app } from "$lib/app.svelte";

  /** `compact` = diferenças como ícones (Grade, Por fase); senão uma linha por runa (Abas, Clássico). */
  let { compact = true }: { compact?: boolean } = $props();

  const own = $derived(app.ownPage);
  const b = $derived(app.build);
  const showing = $derived(!!own);
  const diffs = $derived(own && b ? runeDiffs(ownRuneSet(own), b.runesPick, b.runeStats) : []);
  const perk = (id: number) => app.assets?.perks[id];
  const name = (id: number) => perk(id)?.name ?? `#${id}`;
  const champ = $derived(app.champ?.name ?? "o campeão");
  const saved = $derived(own ? new Date(own.savedAt * 1000).toLocaleDateString("pt-BR") : "");
  let confirming = $state(false);

  const tip = (d: RuneDiff) =>
    `${name(d.mine)} no lugar de ${name(d.other)} (mais jogada): ` +
    (d.delta === null ? "sem dados do lolalytics" : `${pts(d.delta, 1)} pts de WR · ${num(d.n ?? 0)} partidas com ela`);

  async function remove() {
    confirming = false;
    if (own) await app.deleteOwnRunes(own.championId, own.id);
  }
</script>

{#if b}
  <div class="own">
    {#if showing}
      {#if !diffs.length}
        <p class="line muted">Igual à mais jogada.</p>
      {:else if compact}
        <div class="chips">
          <span class="muted">vs mais jogada</span>
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
            <img src={icon(perk(d.mine)?.icon)} alt="" /> {name(d.mine)} <span class="muted">no lugar de {name(d.other)}:</span>
            {#if d.delta === null}<span class="muted">sem dados</span>
            {:else}<span class="nw"><b class:up={d.delta > 0} class:down={d.delta < 0}>{pts(d.delta, 1)} pts</b> <span class="muted">({num(d.n ?? 0)} partidas)</span></span>{/if}
          </p>
        {/each}
      {/if}
    {/if}
    <div class="acts">
      {#if confirming}
        <span>Apagar "{own?.name}" de {champ}?</span>
        <button class="link bad" onclick={remove}>Apagar</button>
        <button class="link" onclick={() => (confirming = false)}>Não</button>
      {:else}
        <button class="link" onclick={() => app.openEditor(null)} disabled={app.ownBusy}
          title="Monte uma página nova para {champ} aqui na janela (funciona sem o cliente aberto); ela aparece ao lado de Mais jogada e Maior WR">
          + Nova página
        </button>
        {#if app.snap?.connected}
          <button class="link" onclick={() => app.copyFromClient()} disabled={app.ownBusy}
            title="Lê a página de runas atual do cliente (sem mudar nada lá) e guarda como página nova de {champ}, com o nome que ela tem no cliente">
            Copiar do cliente
          </button>
        {/if}
        {#if own}
          <button class="link" onclick={() => app.openEditor(own.id)} disabled={app.ownBusy} title="Trocar runas ou nome de &quot;{own.name}&quot;">Editar</button>
          <button class="link" onclick={() => (confirming = true)} title="Salva em {saved}">Apagar</button>
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
