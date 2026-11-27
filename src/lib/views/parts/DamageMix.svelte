<script lang="ts">
  // Composição de dano compacta: barra físico × mágico de cada time e a dica de defesa.
  import { championIcon, icon, pct, type TeamMix } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";

  // mode: "full" = os dois times; "enemy" = só o inimigo e a defesa; "def" = só a dica de defesa.
  let { columns = false, mode = "full" }: { columns?: boolean; mode?: "full" | "enemy" | "def" } = $props();
  const c = $derived(app.composition);
  const teams = $derived(
    c
      ? ([["allies", app.snap?.allyPicks ?? [], c.allies], ["enemy", app.snap?.enemyPicks ?? [], c.enemies]] as [string, number[], TeamMix][])
          .filter(([side, ids]) => ids.length && (mode === "full" || (mode === "enemy" && side === "enemy")))
      : [],
  );
  const share = (m: TeamMix) =>
    t("damageMix.share", { phys: Math.round(m.physicalShare * 100), mag: Math.round(m.magicShare * 100) });
</script>

{#if c && (teams.length || (mode === "def" && c.need && c.items.length))}
  <div class="mix" class:columns>
    {#each teams as [side, ids, m] (side)}
      <div class="team">
        <div class="lbl">
          <span>{t(`damageMix.${side}`)}</span>
          <span class="icons">{#each ids as id (id)}<img class={c.types[id] ?? "unknown"} src={championIcon(id)} alt={app.champName(id)} title={app.champName(id)} />{/each}</span>
        </div>
        <div class="bar" title={share(m)}>
          <i class="phys" style="width: {m.physicalShare * 100}%"></i><i class="mag" style="width: {m.magicShare * 100}%"></i>
        </div>
        <span class="muted small">{share(m)}</span>
      </div>
    {/each}
    {#if c.need && c.items.length}
      <div class="def">
        <span class="muted small">{c.need === "armor" ? t("damageMix.armor") : t("damageMix.magicResist")}</span>
        {#each c.items.slice(0, 3) as it (it.id)}
          <span class="d" title="{app.assets?.items[it.id]?.name} · {it.slot}">
            <img src={icon(app.assets?.items[it.id]?.icon)} alt="" />{pct(it.wr)}
          </span>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .mix { display: grid; gap: 6px; }
  .mix.columns { grid-template-columns: 1fr 1fr 1.2fr; gap: 14px; align-items: start; }
  .team { display: grid; gap: 3px; }
  .lbl { display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: var(--muted); }
  .icons { display: flex; gap: 2px; }
  .icons img { width: 16px; height: 16px; border-radius: 3px; border: 1px solid var(--line); }
  .icons img.physical { border-color: #e07a3f; }
  .icons img.magic { border-color: #4a9fe8; }
  .icons img.mixed { border-color: #b07ae0; }
  .bar { display: flex; height: 7px; border-radius: 4px; overflow: hidden; background: var(--line); }
  .bar i { display: block; height: 100%; }
  .phys { background: #e07a3f; }
  .mag { background: #4a9fe8; }
  .def { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .d { display: flex; align-items: center; gap: 3px; font-size: 11px; color: var(--good); }
  .d img { width: 20px; height: 20px; border-radius: 3px; border: 1px solid var(--good); }
  .small { font-size: 11px; }
</style>
