<script lang="ts">
  // Aviso de versão nova: aparece no topo quando `update.rs` acha uma release mais nova no GitHub.
  // Só instala no clique. "Depois" esconde esta versão por 24 h (guardado no navegador do app).
  import { onMount } from "svelte";
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
    if (installing) return;
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
    // Ouve antes de perguntar: o aviso que chega no meio vale mais que a resposta (e não se perde).
    let heard = false;
    const listening = [
      api.onUpdate((u) => {
        heard = true;
        show(u);
      }),
      api.onUpdateProgress((p) => (progress = p)),
    ];
    Promise.all(listening)
      .then(() => api.update())
      .then((u) => { if (!heard) show(u); }, () => {});
    // Desmontado antes de os ouvintes ficarem prontos: eles saem assim que ficarem.
    return () => listening.forEach((l) => l.then((un) => un()));
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
        <button class="link" onclick={() => api.openUrl(info!.url)}>{t("updateBanner.notes")}</button>
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
  /* O resto dos botões vem do estilo global (`+page.svelte`). */
  .link { padding: 4px 6px; }
  .later { margin-inline-start: auto; color: var(--muted); }
  .small { font-size: 12px; }
  .bad { color: var(--bad); }
  .bar { height: 4px; background: var(--line); border-radius: 2px; overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--gold); transition: width 0.2s; }
</style>
