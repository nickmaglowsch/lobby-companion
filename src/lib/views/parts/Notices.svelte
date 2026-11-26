<script lang="ts">
  // Erro, aviso, "buscando" e erro de importação numa linha (texto completo no tooltip).
  import { app } from "$lib/app.svelte";

  const items = $derived(
    [
      app.snap?.error ? { kind: "bad", text: app.snap.error } : null,
      app.importError ? { kind: "bad", text: app.importError } : null,
      app.banError ? { kind: "bad", text: app.banError } : null,
      app.snap?.canBan ? { kind: "turn", text: "Sua vez de banir: clique numa sugestão de ban para banir no cliente." } : null,
      app.snap?.warning ? { kind: "warn", text: app.snap.warning } : null,
      app.snap?.loading ? { kind: "info", text: "Buscando no lolalytics…" } : null,
    ].filter((x): x is { kind: string; text: string } => !!x),
  );
</script>

<!-- Sempre presente (mesmo vazio): os layouts reservam uma linha da grade para ele. -->
<div class="notices">
  {#each items as n, i (i)}<p class={n.kind} title={n.text}>{n.text}</p>{/each}
</div>

<style>
  .notices { display: grid; gap: 2px; }
  p { margin: 0; padding: 3px 12px; font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; background: var(--panel-2); }
  .bad { color: var(--bad); }
  .warn { color: var(--warn); }
  .info { color: var(--muted); }
  .turn { color: var(--gold); }
</style>
