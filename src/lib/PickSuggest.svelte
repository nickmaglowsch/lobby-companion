<script lang="ts">
  // Cartão "Sugestão de pick": só sugestão (texto e ícones). Clicar mostra o build do campeão
  // na janela (`lookup`), sem mudar nada no cliente: a janela só importa sozinha o build do
  // campeão que você mesmo escolheu ou declarou (`may_apply`).
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    championIcon, pct, pts, LANES,
    type Assets, type PickAdvice, type PickPart, type SynergyAdvice,
  } from "./api";
  import Synergy from "./Synergy.svelte";
  import { t } from "./i18n/index.svelte";

  let { advice, synergy, assets, locked, onPick }: {
    advice: PickAdvice | null;
    synergy: SynergyAdvice | null;
    assets: Assets | null;
    locked: boolean;
    onPick: (championId: number, lane: string | null) => void;
  } = $props();

  const name = (id: number) => assets?.champions[id]?.name ?? `#${id}`;
  const partText = (p: PickPart) =>
    t(p.kind === "counter" ? "pickSuggest.partCounter" : "pickSuggest.partSynergy", {
      name: name(p.championId),
      lane: LANES[p.lane] ?? p.lane,
    });
  // Depois de travar, a lista fica recolhida (o que importa passa a ser a sinergia).
  let open = $state(false);
  const showList = $derived(!locked || open);
</script>

<section class="card">
  <div class="card-head">
    <h2>{t("pickSuggest.title")}</h2>
    {#if advice?.loading}<span class="muted small">{t("common.calculating")}</span>{/if}
    {#if locked && advice?.suggestions.length}
      <button class="link small" onclick={() => (open = !open)}>{open ? t("pickSuggest.hide") : t("pickSuggest.show")}</button>
    {/if}
  </div>
  {#if advice?.lane}
    <div class="muted small">
      {LANES[advice.lane] ?? advice.lane} · {advice.pool === "tier" ? t("pickSuggest.poolTier") : t("pickSuggest.poolMastery")}
      {#if advice.enemies.length || advice.allies.length}
        · {t("pickSuggest.against")}
        {#each advice.enemies as id (id)}<img class="mini" src={championIcon(id)} alt={name(id)} title={name(id)} />{/each}
        {#if advice.allies.length}
          · {t("pickSuggest.with")}
          {#each advice.allies as a (a.championId)}<img class="mini" src={championIcon(a.championId)} alt={name(a.championId)} title={name(a.championId)} />{/each}
        {/if}
      {/if}
    </div>
  {/if}
  {#if showList}
    {#each advice?.suggestions ?? [] as s (s.championId)}
      <button class="pick" onclick={() => onPick(s.championId, advice?.lane ?? null)} title={t("pickSuggest.viewBuild", { name: name(s.championId) })}>
        <img class="pick-icon" src={championIcon(s.championId)} alt={name(s.championId)} />
        <div class="body">
          <div>
            <b>{name(s.championId)}</b>
            <span class="total" class:up={s.total >= 0} class:down={s.total < 0}>{t("lib.points", { value: pts(s.total) })}</span>
            {#if s.mastery !== null}<span class="muted small">· {t("pickSuggest.mastery", { points: s.mastery })}</span>{/if}
          </div>
          <div class="part muted small">{t("pickSuggest.base", { base: pts(s.base), wr: pct(s.wr), count: s.games })}</div>
          {#each s.parts.slice(0, 4) as p (p.kind + p.championId)}
            <div class="part muted small">
              <span class:up={p.delta >= 0.5} class:down={p.delta <= -0.5}>{pts(p.delta)}</span>
              {partText(p)} · {pct(p.wr)} · {t("common.games", { count: p.n })}
            </div>
          {/each}
        </div>
      </button>
    {/each}
  {/if}
  {#if advice?.note}<p class="muted small">{advice.note}</p>{/if}
  {#if showList && advice?.suggestions.length}
    <p class="muted small">
      {t("pickSuggest.footnote")}
      {#if advice.pageUrl}<button class="link small" onclick={() => openUrl(advice!.pageUrl!)}>{t("pickSuggest.tierListLink")}</button>{/if}
    </p>
  {/if}
  {#if synergy}<Synergy {synergy} {assets} />{/if}
</section>

<style>
  .card { background: var(--panel); border: 1px solid var(--line); border-radius: 6px; padding: 10px; }
  .card-head { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
  h2 { margin: 0 0 6px; font-size: 14px; color: var(--gold); }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  p { margin: 6px 0 0; }
  .mini { width: 18px; height: 18px; border-radius: 3px; vertical-align: middle; border: 1px solid var(--gold-dim); }
  .pick { display: grid; grid-template-columns: 36px 1fr; gap: 8px; align-items: start; width: 100%; text-align: start; padding: 5px 0; margin: 0;
    border: none; border-top: 1px solid var(--line); border-radius: 0; background: none; color: inherit; font: inherit; cursor: pointer; }
  .pick:hover .pick-icon { border-color: var(--gold); }
  .pick-icon { width: 36px; height: 36px; border-radius: 4px; border: 1px solid var(--gold-dim); }
  .body { display: grid; gap: 1px; }
  .total { font-size: 12px; font-variant-numeric: tabular-nums; }
  .part { font-variant-numeric: tabular-nums; }
  .up { color: var(--good); }
  .down { color: var(--bad); }
  button.link { border: none; background: none; color: var(--gold); cursor: pointer; padding: 0 4px; font: inherit; }
</style>
