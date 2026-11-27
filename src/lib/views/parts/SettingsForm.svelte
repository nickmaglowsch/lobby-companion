<script lang="ts">
  // Formulário de configurações (no Clássico fica no fim da página; nos outros, numa janela por cima).
  import { api, tiers, regions, patches, championIcon } from "$lib/api";
  import { app, layouts } from "$lib/app.svelte";
  import { compare, LOCALES, t } from "$lib/i18n/index.svelte";

  /** Toda mudança no formulário grava (o `change` de cada campo sobe até aqui); ligar a procura
   *  de atualizações já procura uma. */
  async function changed(e: Event) {
    await app.save();
    if ((e.target as HTMLInputElement).name === "checkUpdates" && app.settings?.checkUpdates) app.checkUpdateNow();
  }
  /** "campeão:lane" da página própria esperando a confirmação para apagar. */
  let confirmDelete = $state<string | null>(null);
  /** Nome nativo de um locale do cliente ("pt_BR" → "Português (Brasil)"). */
  const localeName = (l: string) => LOCALES.find(([v]) => v === l)?.[1] ?? l;
</script>

{#if app.settings}
  {@const settings = app.settings}
  <div class="form" onchange={changed}>
  <div class="grid">
    <label class="wide">{t("settings.layout")}
      <select bind:value={settings.layout}>
        {#each layouts() as [v, l] (v)}<option value={v}>{l}</option>{/each}
      </select>
    </label>
    <label class="wide">{t("settings.language")}
      <span class="lang">
        <select bind:value={settings.language}>
          <option value="">{t("settings.languageAuto")}</option>
          {#each LOCALES as [v, l] (v)}<option value={v}>{l}</option>{/each}
        </select>
        {#if !settings.language}
          <span class="muted small">
            {settings.clientLocale ? t("settings.languageDetected", { name: localeName(settings.clientLocale) }) : t("settings.languageNoClient")}
          </span>
        {/if}
      </span>
    </label>
    <label>{t("settings.tier")} <select bind:value={settings.tier}>{#each tiers() as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
    <label>{t("settings.region")} <select bind:value={settings.region}>{#each regions() as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
    <label>{t("settings.period")} <select bind:value={settings.patch}>{#each patches() as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
    <label title={t("settings.runesTip")}>{t("common.runes")} <select bind:value={settings.runeChoice}><option value="pick">{t("parts.mostPlayed")}</option><option value="win">{t("parts.highestWr")}</option></select></label>
    <label>{t("settings.minGames")} <input type="number" min="0" step="50" bind:value={settings.minGames} /></label>
    <label class="wide" title={t("metric.settingTip")}>{t("metric.setting")}
      <select bind:value={settings.metric}>
        <option value="wpa">{t("metric.optWpa")}</option>
        <option value="wr">{t("metric.optWr")}</option>
      </select>
    </label>
    <label>{t("settings.flashKey")} <select bind:value={settings.flashSlot}><option value={1}>D</option><option value={2}>F</option></select></label>
  </div>
  <div class="checks">
    <label><input type="checkbox" bind:checked={settings.autoImport} /> {t("settings.autoImport")}</label>
    <label><input type="checkbox" bind:checked={settings.importRunes} /> {t("settings.importRunes")}</label>
    <label><input type="checkbox" bind:checked={settings.importItems} /> {t("settings.importItems")}</label>
    <label><input type="checkbox" bind:checked={settings.importSpells} /> {t("common.spells")}</label>
    <label><input type="checkbox" bind:checked={settings.replaceCurrentPage} /> {t("settings.replaceCurrentPage")}</label>
    <label><input type="checkbox" bind:checked={settings.closeInGame} /> {t("settings.closeInGame")}</label>
    <label><input type="checkbox" bind:checked={settings.suggestBans} /> {t("settings.suggestBans")}</label>
    <label><input type="checkbox" bind:checked={settings.suggestPicks} /> {t("settings.suggestPicks")}</label>
    {#if settings.suggestPicks}
      <label class="sub">{t("settings.pickPool")} <select bind:value={settings.pickPool}><option value="mastery">{t("settings.poolMastery")}</option><option value="tier">{t("settings.poolTier")}</option></select></label>
    {/if}
    <label><input type="checkbox" name="checkUpdates" bind:checked={settings.checkUpdates} /> {t("settings.checkUpdates")}</label>
  </div>
  <div class="checks overlay">
    <span class="muted small">{t("settings.overlay")}</span>
    <label><input type="checkbox" bind:checked={settings.overlay.enabled} /> {t("settings.overlayEnabled")}</label>
    {#if settings.overlay.enabled}
      <span class="sub muted small">{t("settings.overlayBorderless")}</span>
      <label class="sub"><input type="checkbox" bind:checked={settings.overlay.inhibitors} /> {t("settings.overlayInhibitors")}</label>
      <label class="sub"><input type="checkbox" bind:checked={settings.overlay.nextItem} /> {t("settings.overlayNextItem")}</label>
      <label class="sub"><input type="checkbox" bind:checked={settings.overlay.csPerMin} /> {t("settings.overlayCs")}</label>
      <label class="sub"><input type="checkbox" bind:checked={settings.overlay.goldDiff} /> {t("settings.overlayGoldDiff")}</label>
      <label class="sub"><input type="checkbox" bind:checked={settings.overlay.goldGraph} /> {t("settings.overlayGraph")}</label>
      <label class="sub">{t("settings.overlayKey")}
        <select bind:value={settings.overlay.scoreboardKey}>
          <option value={9}>Tab</option>
          <option value={192}>`</option>
        </select>
      </label>
      <span class="sub"><button onclick={() => api.overlayEdit(true)}>{t("settings.overlayAdjust")}</button></span>
    {/if}
  </div>
  <div class="own">
    <span class="muted small">{t("settings.ownPages")}</span>
    {#each [...app.ownPages].sort((a, b) => compare(app.champName(a.championId), app.champName(b.championId)) || a.id - b.id) as p (`${p.championId}:${p.id}`)}
      {@const key = `${p.championId}:${p.id}`}
      <div class="own-row">
        <img src={championIcon(p.championId)} alt="" />
        <span>{app.champName(p.championId)} <span class="muted">· "{p.name}"{p.active ? ` · ${t("settings.ownActive")}` : ""}</span></span>
        {#if confirmDelete === key}
          <button class="link bad" onclick={() => { confirmDelete = null; app.deleteOwnRunes(p.championId, p.id); }}>{t("settings.deleteConfirm")}</button>
          <button class="link" onclick={() => (confirmDelete = null)}>{t("parts.no")}</button>
        {:else}
          <button class="link" onclick={() => (confirmDelete = key)}>{t("parts.delete")}</button>
        {/if}
      </div>
    {:else}
      <span class="muted small">{t("settings.ownEmptyNamed", { new: t("parts.newPage"), copy: t("parts.copyFromClient"), mostPlayed: t("parts.mostPlayed"), highestWr: t("parts.highestWr") })}</span>
    {/each}
  </div>
  <div class="version">
    <span class="muted small">{t("settings.version", { version: app.version })}</span>
    <button onclick={() => app.checkUpdateNow()}>{t("settings.checkNow")}</button>
    {#if app.updateCheck}<span class="muted small">{app.updateCheck}</span>{/if}
  </div>
  <p class="muted small foot">{t("settings.footer")}</p>
  </div>
{/if}

<style>
  .form { display: contents; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin: 10px 0; }
  .grid label { display: grid; gap: 2px; font-size: 12px; color: var(--muted); }
  .grid .wide { grid-column: 1 / -1; }
  .lang { display: flex; gap: 8px; align-items: center; min-width: 0; }
  .lang span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .checks { display: grid; gap: 4px; font-size: 13px; }
  .checks .sub { padding-inline-start: 22px; color: var(--muted); }
  .checks.overlay { margin-top: 10px; }
  .own { display: grid; gap: 4px; margin-top: 10px; font-size: 13px; }
  .own-row { display: flex; align-items: center; gap: 6px; }
  .own-row img { width: 20px; height: 20px; border-radius: 3px; }
  .own-row button { margin-inline-start: auto; }
  .own-row button + button { margin-inline-start: 0; }
  button.link { border: none; background: none; color: var(--gold); padding: 0 2px; cursor: pointer; font: inherit; font-size: 12px; }
  .bad { color: var(--bad) !important; }
  .version { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-top: 10px; }
  .small { font-size: 12px; }
  .foot { margin: 10px 0 0; }
</style>
