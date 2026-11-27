<script lang="ts">
  // Sugestões de pick compactas. Clicar só mostra o build na janela (não muda nada no cliente).
  import { championIcon, pct, laneName, type PickPart, type PickSuggestion } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import { signed } from "./fmt";
  import { gainText, pickExplain } from "$lib/metric";

  let { variant = "rows", max = 5 }: { variant?: "rows" | "detailed" | "cards"; max?: number } = $props();
  const advice = $derived(app.snap?.picks ?? null);
  const list = $derived((advice?.suggestions ?? []).slice(0, max));
  const partText = (p: PickPart) =>
    t(p.kind === "counter" ? "pickList.partCounter" : "pickList.partSynergy", {
      delta: signed(p.delta), name: app.champName(p.championId), lane: laneName(p.lane), wr: pct(p.wr), count: p.n,
    });
  const why = (s: PickSuggestion) =>
    [
      t("metric.total", { value: gainText(advice?.metric, signed(s.total)) }),
      t("pickList.base", { delta: signed(s.base), wr: pct(s.wr), count: s.games }),
      ...s.parts.map(partText),
      "",
      pickExplain(advice?.metric),
    ].join("\n");
  const best = (s: PickSuggestion) => [...s.parts].sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta))[0];
  const pick = (id: number) => app.lookup(id, advice?.lane ?? null);
</script>

{#if advice?.loading && !list.length}
  <p class="muted small">{t("common.calculating")}</p>
{:else if !list.length}
  <p class="muted small">{advice?.note ?? t("pickList.none")}</p>
{:else if variant === "cards"}
  <div class="cards">
    {#each list as s, i (s.championId)}
      <button class="card" class:top={i === 0} onclick={() => pick(s.championId)} title={why(s)}>
        <img src={championIcon(s.championId)} alt="" />
        <b>{app.champName(s.championId)}</b>
        <span class="score" class:up={s.total >= 0} class:down={s.total < 0}>{signed(s.total)}</span>
        <span class="muted small">{pct(s.wr)}{s.mastery !== null ? ` · ${t("pickList.mastery", { k: Math.round(s.mastery / 1000) })}` : ""}</span>
        <span class="parts">
          {#each s.parts.slice(0, 3) as p, j (j)}
            <span class:up={p.delta > 0} class:down={p.delta < 0}><img src={championIcon(p.championId)} alt="" />{signed(p.delta)}</span>
          {/each}
        </span>
      </button>
    {/each}
  </div>
{:else}
  {#each list as s (s.championId)}
    {@const b = best(s)}
    <button class="row" onclick={() => pick(s.championId)} title={why(s)}>
      <img src={championIcon(s.championId)} alt="" />
      <span class="txt">
        <span><b>{app.champName(s.championId)}</b> <span class:up={s.total >= 0} class:down={s.total < 0}>{signed(s.total)}</span>
          {#if variant === "detailed"}<span class="muted small">{pct(s.wr)}</span>{/if}</span>
        {#if variant === "detailed"}
          <span class="parts inline">
            {#each s.parts.slice(0, 3) as p, j (j)}
              <span class:up={p.delta > 0} class:down={p.delta < 0}><img src={championIcon(p.championId)} alt="" />{signed(p.delta)}</span>
            {/each}
          </span>
        {:else if b}
          <span class="muted small">{t(b.kind === "counter" ? "pickList.bestCounter" : "pickList.bestSynergy", { name: app.champName(b.championId), delta: signed(b.delta) })}</span>
        {/if}
      </span>
    </button>
  {/each}
{/if}

<style>
  button { font: inherit; color: inherit; cursor: pointer; }
  .row { display: flex; gap: 8px; align-items: center; width: 100%; padding: 3px 0; border: none; background: none; text-align: start; border-radius: 0; }
  .row > img { width: 32px; height: 32px; border-radius: 4px; border: 1px solid var(--gold-dim); }
  .row:hover > img { border-color: var(--gold); }
  .txt { display: grid; line-height: 1.25; font-size: 13px; }
  .cards { display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px; }
  .card { display: grid; justify-items: center; align-content: start; gap: 3px; padding: 10px 6px; background: var(--panel); border: 1px solid var(--line); border-radius: 6px; text-align: center; }
  .card:hover { border-color: var(--gold-dim); }
  .card.top { border-color: var(--gold); }
  .card > img { width: 58px; height: 58px; border-radius: 6px; }
  .score { font-size: 18px; font-weight: 600; }
  .parts { display: grid; gap: 2px; font-size: 11px; margin-top: 4px; }
  .parts.inline { display: flex; gap: 8px; margin: 0; }
  .parts span { display: flex; align-items: center; gap: 3px; justify-content: center; }
  .parts img { width: 14px; height: 14px; border-radius: 2px; }
  .small { font-size: 11px; }
  p { margin: 0; }
</style>
