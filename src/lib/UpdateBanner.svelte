<script lang="ts">
  // Aviso de versão nova: aparece no topo quando `update.rs` acha uma release mais nova no GitHub.
  // Só instala no clique. "Depois" esconde esta versão por 24 h (guardado no navegador do app).
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type UpdateInfo, type UpdateProgress } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";

  // `floating`: nos layouts de uma tela, o aviso fica por cima, no canto de baixo.
  let { inChampSelect = false, floating = false }: { inChampSelect?: boolean; floating?: boolean } = $props();

  const SNOOZE_KEY = "lc-update-later";
  const SNOOZE_MS = 24 * 3600 * 1000;

  let info = $state<UpdateInfo | null>(null);
  let hidden = $state(false);
  let installing = $state(false);
  let progress = $state<UpdateProgress | null>(null);
  let error = $state<string | null>(null);

  const percent = $derived(
    progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null,
  );

  function snoozed(version: string): boolean {
    try {
      const s = JSON.parse(localStorage.getItem(SNOOZE_KEY) ?? "null");
      return s?.version === version && Date.now() < s.until;
    } catch {
      return false;
    }
  }

  function show(u: UpdateInfo | null) {
    info = u;
    hidden = !u || snoozed(u.version);
  }

  function later() {
    if (!info) return;
    try {
      localStorage.setItem(SNOOZE_KEY, JSON.stringify({ version: info.version, until: Date.now() + SNOOZE_MS }));
    } catch {
      /* sem armazenamento: esconde só nesta janela */
    }
    hidden = true;
  }

  async function install() {
    installing = true;
    error = null;
    try {
      await api.installUpdate();
    } catch (e) {
      error = String(e);
      installing = false;
      progress = null;
    }
  }

  onMount(() => {
    const unlisten: (() => void)[] = [];
    (async () => {
      show(await api.update());
      unlisten.push(await api.onUpdate(show));
      unlisten.push(await api.onUpdateProgress((p) => (progress = p)));
    })();
    return () => unlisten.forEach((u) => u());
  });
</script>

{#if info && !hidden}
  <section class="update" class:floating role="status">
    <p>
      <strong>{t("updateBanner.available", { version: info.version })}</strong>
      <span class="muted">{t("updateBanner.current", { current: info.current })}</span>
    </p>
    {#if installing}
      <p class="muted small">
        {percent !== null ? t("updateBanner.downloadingPct", { percent }) : t("updateBanner.downloading")}
        {t("updateBanner.restartNote")}
      </p>
      {#if percent !== null}<div class="bar"><span style="width: {percent}%"></span></div>{/if}
    {:else}
      <div class="row">
        <button class="primary" onclick={install} disabled={inChampSelect}
          title={inChampSelect ? t("updateBanner.champSelect") : ""}>{t("updateBanner.install")}</button>
        <button class="link" onclick={() => openUrl(info!.url)}>{t("updateBanner.notes")}</button>
        <button class="link later" onclick={later}>{t("updateBanner.later")}</button>
      </div>
      {#if inChampSelect && !floating}
        <p class="muted small">{t("updateBanner.champSelect")}</p>
      {/if}
    {/if}
    {#if error}<p class="bad small">{error}</p>{/if}
  </section>
{/if}

<style>
  .update { border: 1px solid var(--gold); border-radius: 6px; background: var(--panel); padding: 8px 10px; display: grid; gap: 6px; }
  .update.floating { position: fixed; inset-inline-end: 12px; bottom: 52px; z-index: 10; width: 300px; padding: 6px 10px; gap: 4px; font-size: 13px; box-shadow: 0 6px 24px rgb(0 0 0 / 0.6); }
  p { margin: 0; }
  .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  button { background: var(--panel); color: var(--text); border: 1px solid var(--gold-dim); border-radius: 4px; padding: 4px 10px; font: inherit; cursor: pointer; }
  button:hover:not(:disabled) { border-color: var(--gold); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: #1e282d; border-color: var(--gold); color: var(--gold); font-weight: 600; }
  button.link { border: none; background: none; color: var(--gold); padding: 4px 6px; }
  .later { margin-inline-start: auto; color: var(--muted); }
  button.later { color: var(--muted); }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  .bad { color: var(--bad); }
  .bar { height: 4px; background: var(--line); border-radius: 2px; overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--gold); transition: width 0.2s; }
</style>
