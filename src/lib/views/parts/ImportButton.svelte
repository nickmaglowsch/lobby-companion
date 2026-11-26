<script lang="ts">
  // Botão "Importar" + o que foi importado (✓/✗ com o motivo no tooltip).
  import { app } from "$lib/app.svelte";

  let { big = false, labels = true }: { big?: boolean; labels?: boolean } = $props();
  const parts = $derived(
    app.snap?.import
      ? ([["Runas", app.snap.import.runes], ["Itens", app.snap.import.items], ["Feitiços", app.snap.import.spells]] as const)
          .filter(([, o]) => !!o)
          .map(([label, o]) => ({ label, ok: o!.ok, msg: o!.msg }))
      : [],
  );
</script>

<div class="imp" class:big>
  <button class="primary" onclick={() => app.importNow()} disabled={app.importing || !app.snap?.connected || !app.build}>
    {app.importing ? "Importando…" : "Importar"}
  </button>
  <span class="marks">
    {#each parts as p (p.label)}
      <span class:ok={p.ok} class:bad={!p.ok} title="{p.label}: {p.msg}">{p.ok ? "✓" : "✗"}{labels ? ` ${p.label}` : p.label[0]}</span>
    {/each}
  </span>
</div>

<style>
  .imp { display: flex; align-items: center; gap: 8px; }
  .imp.big { display: grid; gap: 4px; }
  .imp.big button { padding: 8px; font-size: 15px; }
  .marks { display: flex; gap: 8px; font-size: 11px; white-space: nowrap; }
  .ok { color: var(--good); }
  .bad { color: var(--bad); }
</style>
