<script lang="ts">
  // Formulário de configurações (no Clássico fica no fim da página; nos outros, numa janela por cima).
  import { TIERS, REGIONS, PATCHES, championIcon } from "$lib/api";
  import { app, LAYOUTS } from "$lib/app.svelte";

  async function toggleUpdates() {
    await app.save();
    if (app.settings?.checkUpdates) app.checkUpdateNow();
  }
  /** "campeão:lane" da página própria esperando a confirmação para apagar. */
  let confirmDelete = $state<string | null>(null);
</script>

{#if app.settings}
  {@const settings = app.settings}
  <div class="grid">
    <label class="wide">Layout da janela
      <select bind:value={settings.layout} onchange={() => app.save()}>
        {#each LAYOUTS as [v, l] (v)}<option value={v}>{l}</option>{/each}
      </select>
    </label>
    <label>Elo <select bind:value={settings.tier} onchange={() => app.save()}>{#each TIERS as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
    <label>Região <select bind:value={settings.region} onchange={() => app.save()}>{#each REGIONS as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
    <label>Período <select bind:value={settings.patch} onchange={() => app.save()}>{#each PATCHES as [v, l] (v)}<option value={v}>{l}</option>{/each}</select></label>
    <label title="Para campeões sem página sua escolhida">Runas <select bind:value={settings.runeChoice} onchange={() => app.save()}><option value="pick">Mais jogada</option><option value="win">Maior WR</option></select></label>
    <label>Amostra mínima <input type="number" min="0" step="50" bind:value={settings.minGames} onchange={() => app.save()} /></label>
    <label>Flash na tecla <select bind:value={settings.flashSlot} onchange={() => app.save()}><option value={1}>D</option><option value={2}>F</option></select></label>
  </div>
  <div class="checks">
    <label><input type="checkbox" bind:checked={settings.autoImport} onchange={() => app.save()} /> Importar sozinho (ao travar e ao mudar runa, lane ou filtro)</label>
    <label><input type="checkbox" bind:checked={settings.importRunes} onchange={() => app.save()} /> Página de runas</label>
    <label><input type="checkbox" bind:checked={settings.importItems} onchange={() => app.save()} /> Conjunto de itens (WR no título dos blocos)</label>
    <label><input type="checkbox" bind:checked={settings.importSpells} onchange={() => app.save()} /> Feitiços</label>
    <label><input type="checkbox" bind:checked={settings.replaceCurrentPage} onchange={() => app.save()} /> Sem espaço para página nova: substituir a página atual</label>
    <label><input type="checkbox" bind:checked={settings.closeInGame} onchange={() => app.save()} /> Fechar a janela quando a partida começar</label>
    <label><input type="checkbox" bind:checked={settings.suggestBans} onchange={() => app.save()} /> Sugerir bans pelos campeões do meu time</label>
    <label><input type="checkbox" bind:checked={settings.suggestPicks} onchange={() => app.save()} /> Sugerir pick para a minha posição (counter + sinergia)</label>
    {#if settings.suggestPicks}
      <label class="sub">Candidatos <select bind:value={settings.pickPool} onchange={() => app.save()}><option value="mastery">Meu pool (maestria)</option><option value="tier">Tier list da lane</option></select></label>
    {/if}
    <label><input type="checkbox" bind:checked={settings.checkUpdates} onchange={toggleUpdates} /> Procurar atualizações no GitHub (só avisa; instala quando você clicar)</label>
  </div>
  <div class="own">
    <span class="muted small">Minhas páginas de runas</span>
    {#each [...app.ownPages].sort((a, b) => app.champName(a.championId).localeCompare(app.champName(b.championId), "pt-BR") || a.id - b.id) as p (`${p.championId}:${p.id}`)}
      {@const key = `${p.championId}:${p.id}`}
      <div class="own-row">
        <img src={championIcon(p.championId)} alt="" />
        <span>{app.champName(p.championId)} <span class="muted">· "{p.name}"{p.active ? " · em uso" : ""}</span></span>
        {#if confirmDelete === key}
          <button class="link bad" onclick={() => { confirmDelete = null; app.deleteOwnRunes(p.championId, p.id); }}>Apagar mesmo</button>
          <button class="link" onclick={() => (confirmDelete = null)}>Não</button>
        {:else}
          <button class="link" onclick={() => (confirmDelete = key)}>Apagar</button>
        {/if}
      </div>
    {:else}
      <span class="muted small">Nenhuma. Busque o campeão e use "+ Nova página" no cartão de runas (funciona sem o cliente aberto) ou "Copiar do cliente": cada página aparece pelo nome ao lado de Mais jogada e Maior WR.</span>
    {/each}
  </div>
  <div class="version">
    <span class="muted small">Versão {app.version}</span>
    <button onclick={() => app.checkUpdateNow()}>Procurar agora</button>
    {#if app.updateCheck}<span class="muted small">{app.updateCheck}</span>{/if}
  </div>
  <p class="muted small foot">Dados: lolalytics.com (ranqueada solo). Não afiliado à Riot Games nem ao lolalytics.</p>
{/if}

<style>
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; margin: 10px 0; }
  .grid label { display: grid; gap: 2px; font-size: 12px; color: var(--muted); }
  .grid .wide { grid-column: 1 / -1; }
  .checks { display: grid; gap: 4px; font-size: 13px; }
  .checks .sub { padding-left: 22px; color: var(--muted); }
  .own { display: grid; gap: 4px; margin-top: 10px; font-size: 13px; }
  .own-row { display: flex; align-items: center; gap: 6px; }
  .own-row img { width: 20px; height: 20px; border-radius: 3px; }
  .own-row button { margin-left: auto; }
  .own-row button + button { margin-left: 0; }
  button.link { border: none; background: none; color: var(--gold); padding: 0 2px; cursor: pointer; font: inherit; font-size: 12px; }
  .bad { color: var(--bad) !important; }
  .version { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; margin-top: 10px; }
  .muted { color: var(--muted); }
  .small { font-size: 12px; }
  .foot { margin: 10px 0 0; }
</style>
