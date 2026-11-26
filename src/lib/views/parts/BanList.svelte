<script lang="ts">
  // Lista de bans em três tamanhos: "strip" (só ícones e pontos), "rows" (linha com nome) e
  // "cards" (cartão com o motivo). O porquê completo fica no tooltip.
  import { championIcon, pct, num, LANES, type BanPick } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { signed } from "./fmt";

  let { list, variant = "rows", max = 5 }: { list: BanPick[]; variant?: "strip" | "rows" | "cards"; max?: number } = $props();

  function why(p: BanPick): string {
    if (p.tier) return `${pct(p.tier.wr)} na lane · pick ${pct(p.tier.pr)} · ban ${pct(p.tier.br)} · ${num(p.tier.games)} partidas`;
    return p.reasons
      .map((r) => `${app.champName(r.allyChampionId)}: ${pct(r.wr)} contra ele (${LANES[r.lane] ?? r.lane}) · geral ${pct(r.baseWr)} · ${num(r.n)} partidas`)
      .join("\n");
  }
  function short(p: BanPick): string {
    if (p.tier) return `${pct(p.tier.wr)} · pick ${pct(p.tier.pr)}`;
    const r = p.reasons[0];
    return r ? `${app.champName(r.allyChampionId)} ${pct(r.wr)} contra · ${num(r.n)}` : "";
  }
  const shown = $derived(list.slice(0, max));
  /** Na sua vez de banir, cada sugestão vira um botão que bane no cliente. */
  const can = $derived(!!app.snap?.canBan);
  const tag = $derived(can ? "button" : "div");
  const act = (p: BanPick) => (can ? `Clique para banir ${app.champName(p.championId)}\n` : "");
</script>

{#if variant === "strip"}
  <div class="strip">
    {#each shown as p (p.championId)}
      <svelte:element this={tag} class="s" class:act={can} class:busy={app.banBusy === p.championId} role={can ? undefined : "listitem"}
        onclick={() => can && app.ban(p.championId)} title="{act(p)}{app.champName(p.championId)} {signed(p.gain)} pts&#10;{why(p)}">
        <img src={championIcon(p.championId)} alt={app.champName(p.championId)} />
        <span class="up">{signed(p.gain)}</span>
      </svelte:element>
    {/each}
  </div>
{:else if variant === "rows"}
  {#each shown as p (p.championId)}
    <svelte:element this={tag} class="row" class:act={can} class:busy={app.banBusy === p.championId} role={can ? undefined : "listitem"}
      onclick={() => can && app.ban(p.championId)} title="{act(p)}{why(p)}">
      <img src={championIcon(p.championId)} alt="" />
      <span class="nm">{app.champName(p.championId)}</span>
      <span class="up">{signed(p.gain)}</span>
    </svelte:element>
  {/each}
{:else}
  {#each shown as p, i (p.championId)}
    <svelte:element this={tag} class="card" class:top={i === 0} class:act={can} class:busy={app.banBusy === p.championId} role={can ? undefined : "listitem"}
      onclick={() => can && app.ban(p.championId)} title="{act(p)}{why(p)}">
      <img src={championIcon(p.championId)} alt="" />
      <div>
        <div class="bn">{app.champName(p.championId)} <span class="up">{signed(p.gain)}</span></div>
        <div class="muted small">{short(p)}</div>
      </div>
      {#if can}<span class="ban-tag">{app.banBusy === p.championId ? "banindo…" : "Banir"}</span>{/if}
    </svelte:element>
  {/each}
{/if}

<style>
  /* Botão (sua vez de banir) com a mesma cara do item normal. */
  button { font: inherit; color: inherit; text-align: left; background: none; border: none; padding: 0; margin: 0; width: 100%; }
  .act { cursor: pointer; }
  .act:hover img { box-shadow: 0 0 0 2px var(--bad); }
  .busy { opacity: 0.6; }
  .ban-tag { margin-left: auto; font-size: 11px; color: var(--bad); border: 1px solid var(--bad); border-radius: 4px; padding: 1px 6px; }
  .strip { display: flex; gap: 4px; }
  .s { display: grid; justify-items: center; width: 40px; font-size: 11px; font-variant-numeric: tabular-nums; }
  .s img { width: 38px; height: 38px; border-radius: 4px; border: 1px solid var(--bad); }
  .row { display: grid; grid-template-columns: 26px 1fr auto; gap: 6px; align-items: center; padding: 2px 0; font-size: 13px; }
  .row img { width: 26px; height: 26px; border-radius: 4px; border: 1px solid var(--bad); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  button.s { width: 40px; }
  button.card { width: 100%; }
  .card { display: flex; gap: 10px; align-items: center; padding: 6px 8px; margin-bottom: 6px; background: var(--panel); border: 1px solid var(--line); border-radius: 6px; }
  .card.top { border-color: var(--gold-dim); }
  .card img { width: 42px; height: 42px; border-radius: 4px; }
  .bn { font-weight: 600; font-size: 15px; }
  .up { color: var(--good); font-variant-numeric: tabular-nums; }
  .muted { color: var(--muted); }
  .small { font-size: 11px; }
</style>
