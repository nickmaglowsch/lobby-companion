<script lang="ts">
  // Ponto de conexão + fase do cliente (à esquerda), o conteúdo do layout no meio e
  // busca + configurações à direita.
  import type { Snippet } from "svelte";
  import { PHASES } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import Search from "./Search.svelte";

  let { children, status = true, searchWidth = 130 }: { children?: Snippet; status?: boolean; searchWidth?: number } = $props();
</script>

<header>
  <span class="dot" class:on={app.snap?.connected} title={app.snap?.connected ? t("topBar.connected") : t("common.clientClosedLong")}></span>
  {#if status}
    <span class="phase">{#if !app.snap?.connected}{t("common.clientClosed")}{:else}{PHASES[app.snap.phase] ?? app.snap.phase}{/if}</span>
  {/if}
  {@render children?.()}
  <span class="grow"></span>
  <Search width={searchWidth} />
  <button class="icon" title={t("parts.settings")} onclick={() => (app.settingsOpen = true)}>⚙</button>
</header>

<style>
  header { display: flex; align-items: center; gap: 8px; padding: 0 12px; background: var(--panel); border-bottom: 1px solid var(--line); min-width: 0; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--bad); flex: none; }
  .dot.on { background: var(--good); }
  .phase { color: var(--muted); font-size: 12px; white-space: nowrap; }
  .grow { flex: 1; }
  .icon { width: 28px; height: 28px; padding: 0; font-size: 15px; flex: none; }
</style>
