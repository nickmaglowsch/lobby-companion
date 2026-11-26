<script lang="ts">
  // Botão "Importar" + o que foi importado (✓/✗ com o motivo no tooltip).
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";

  let { big = false, labels = true }: { big?: boolean; labels?: boolean } = $props();
  const parts = $derived(
    app.snap?.import
      ? ([[t("common.runes"), app.snap.import.runes], [t("common.items"), app.snap.import.items], [t("common.spells"), app.snap.import.spells]] as const)
          .filter(([, o]) => !!o)
          .map(([label, o]) => ({ label, ok: o!.ok, msg: o!.msg }))
      : [],
  );
</script>

<div class="imp" class:big>
  <button class="primary" onclick={() => app.importNow()} disabled={app.importing || !app.snap?.connected || !app.build}>
    {app.importing ? t("importButton.importing") : t("importButton.import")}
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
