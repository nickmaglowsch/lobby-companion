<script lang="ts">
  import { num, type Damage } from "./api";

  // Perfil de dano do campeão (dano médio por partida no lolalytics).
  let { damage }: { damage: Damage | null } = $props();

  const parts = $derived.by(() => {
    if (!damage) return [];
    const total = damage.physical + damage.magic + damage.true;
    if (total <= 0) return [];
    return [
      { key: "phys", label: "Físico", value: damage.physical },
      { key: "magic", label: "Mágico", value: damage.magic },
      { key: "true", label: "Verdadeiro", value: damage.true },
    ].map((p) => ({ ...p, share: (p.value / total) * 100 }));
  });
</script>

{#if parts.length}
  <div class="dmg" title={parts.map((p) => `${p.label}: ${num(Math.round(p.value))} por partida`).join(" · ")}>
    <div class="bar">
      {#each parts as p (p.key)}
        {#if p.share > 0}<span class={p.key} style="width: {p.share}%"></span>{/if}
      {/each}
    </div>
    <div class="legend">
      {#each parts as p (p.key)}
        <span><i class={p.key}></i>{p.label} {Math.round(p.share)}%</span>
      {/each}
    </div>
  </div>
{/if}

<style>
  .dmg { margin-top: 6px; display: grid; gap: 3px; max-width: 280px; }
  .bar { display: flex; height: 6px; border-radius: 3px; overflow: hidden; background: var(--line); }
  .legend { display: flex; gap: 10px; font-size: 11px; color: var(--muted); }
  .legend span { display: flex; align-items: center; gap: 4px; }
  i { width: 8px; height: 8px; border-radius: 2px; display: inline-block; }
  .phys { background: #e07a3f; }
  .magic { background: #4a9fe8; }
  .true { background: #e8e8e8; }
</style>
