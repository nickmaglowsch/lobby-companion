<script lang="ts">
  // Configurações por cima do layout (Grade, Abas e Por fase não têm rolagem para o formulário).
  import { app } from "$lib/app.svelte";
  import { t } from "$lib/i18n/index.svelte";
  import SettingsForm from "./SettingsForm.svelte";

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") app.settingsOpen = false;
  }
</script>

<svelte:window onkeydown={onKey} />

{#if app.settingsOpen}
  <div class="backdrop" role="presentation" onclick={() => (app.settingsOpen = false)}></div>
  <div class="dialog" role="dialog" aria-label={t("parts.settings")}>
    <div class="head">
      <h2>{t("parts.settings")}</h2>
      <button class="close" onclick={() => (app.settingsOpen = false)} title={t("parts.closeEsc")}>✕</button>
    </div>
    <div class="body"><SettingsForm /></div>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; background: rgb(0 0 0 / 0.55); z-index: 20; }
  .dialog { position: fixed; z-index: 21; top: 24px; left: 50%; transform: translateX(-50%); width: min(520px, calc(100vw - 24px));
    max-height: calc(100vh - 48px); display: grid; grid-template-rows: auto 1fr; background: var(--panel); border: 1px solid var(--gold-dim); border-radius: 8px; }
  .head { display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; border-bottom: 1px solid var(--line); }
  h2 { margin: 0; font-size: 14px; color: var(--gold); }
  .close { border: none; background: none; color: var(--muted); font-size: 16px; padding: 0 4px; }
  .body { padding: 4px 14px 14px; overflow: auto; }
</style>
