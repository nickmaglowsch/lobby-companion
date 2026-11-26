<script lang="ts">
  import { championIcon, icon, pct, type DamageType, type TeamMix } from "./api";
  import { app } from "./app.svelte";
  import { t } from "./i18n/index.svelte";

  // Composição de dano dos dois times (tipo de dano do próprio cliente) e, quando o inimigo
  // concentra um tipo, os itens de armadura/resistência mágica do seu build. Só exibição.
  // Os dados vêm de `app.composition` (recalculado quando mudam os campeões ou o build).
  const comp = $derived(app.composition);
  const allies = $derived(app.snap?.allyPicks ?? []);
  const enemies = $derived(app.snap?.enemyPicks ?? []);
  const assets = $derived(app.assets);

  const typeName = (k: DamageType) => t(`composition.${k}`);
  const champName = (id: number) => assets?.champions[id]?.name ?? `#${id}`;

  function describe(m: TeamMix): string {
    const parts = (["magic", "physical", "mixed"] as DamageType[])
      .map((k) => [k, m[k]] as const)
      .filter(([, n]) => n > 0)
      .sort((x, y) => y[1] - x[1])
      .map(([k, n]) => t(`composition.${k}Count`, { count: n }));
    return parts.length ? parts.join(t("lib.listSep")) : "—";
  }

  const teams = $derived(
    comp
      ? ([
          [t("composition.allies"), allies, comp.allies],
          [t("composition.enemies"), enemies, comp.enemies],
        ] as [string, number[], TeamMix][]).filter(([, ids]) => ids.length > 0)
      : [],
  );
</script>

{#if comp && teams.length}
  <section class="card">
    <h2>{t("composition.title")}</h2>
    {#each teams as [label, ids, mix] (label)}
      <div class="team">
        <span class="label">{label}</span>
        <span class="icons">
          {#each ids as id (id)}
            <img class={comp.types[id] ?? "unknown"} src={championIcon(id)} alt={champName(id)}
              title="{champName(id)} · {comp.types[id] ? typeName(comp.types[id]) : t('composition.unknownType')}" />
          {/each}
        </span>
        <span>{describe(mix)}</span>
        {#if mix.physical + mix.magic + mix.mixed > 0}
          <span class="bar" title={t("composition.barTitle", { physical: Math.round(mix.physicalShare * 100), magic: Math.round(mix.magicShare * 100) })}>
            <span class="physical" style="width: {mix.physicalShare * 100}%"></span>
            <span class="magic" style="width: {mix.magicShare * 100}%"></span>
          </span>
        {/if}
      </div>
    {/each}
    {#if comp.need && comp.items.length}
      <div class="def">
        <span class="muted small">{comp.need === "armor" ? t("composition.armorInBuild") : t("composition.mrInBuild")}</span>
        {#each comp.items as it (it.id)}
          {@const item = assets?.items[it.id]}
          <span class="item" title="{item?.name} · {it.slot} · {t('common.games', { count: it.n })}">
            <img src={icon(item?.icon)} alt={item?.name} />
            <span>{pct(it.wr)}</span>
            <span class="muted tiny">{it.slot}</span>
          </span>
        {/each}
      </div>
    {/if}
    {#if comp.note}<p class="small note">{comp.note}</p>{/if}
    <p class="muted small">{t("composition.footnote")}</p>
  </section>
{/if}

<style>
  .card { background: var(--panel); border: 1px solid var(--line); border-radius: 6px; padding: 10px; }
  h2 { margin: 0 0 6px; font-size: 14px; color: var(--gold); }
  .team { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; padding: 3px 0; font-size: 13px; }
  .label { color: var(--muted); min-width: 84px; }
  .icons { display: flex; gap: 2px; }
  .icons img { width: 22px; height: 22px; border-radius: 3px; border: 2px solid var(--line); }
  .icons img.physical { border-color: #e07a3f; }
  .icons img.magic { border-color: #4a9fe8; }
  .icons img.mixed { border-color: #b07ae0; }
  .bar { display: flex; width: 80px; height: 6px; border-radius: 3px; overflow: hidden; background: var(--line); }
  .bar .physical { background: #e07a3f; }
  .bar .magic { background: #4a9fe8; }
  .def { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-top: 6px; }
  .item { display: grid; justify-items: center; font-size: 12px; font-variant-numeric: tabular-nums; }
  .item img { width: 30px; height: 30px; border-radius: 4px; border: 1px solid var(--good); }
  .note { color: var(--good); margin: 6px 0 0; }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  .tiny { font-size: 10px; }
</style>
