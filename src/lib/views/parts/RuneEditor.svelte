<script lang="ts">
  // Editor da página de runas própria, por cima do layout. Funciona sem o cliente do League:
  // usa a cópia dos dados da última conexão e só grava no arquivo do app (rune-pages.json).
  // O WR de cada runa (naquele lugar da página) vem do build em tela, quando há.
  import { icon, pct, num, perkStat, ownRuneSet, type Style } from "$lib/api";
  import { app } from "$lib/app.svelte";

  const styles = $derived(app.assets?.styles.filter((s) => s.slots.length >= 4 && s.slots[0].length) ?? []);
  const shardSlots = $derived(app.assets?.shardSlots ?? []);
  const perk = (id: number) => app.assets?.perks[id];

  let pri = $state<number | null>(null);
  let primary = $state<(number | null)[]>([null, null, null, null]);
  let sub = $state<number | null>(null);
  /** Secundárias na ordem em que foram escolhidas (a mais antiga sai quando passa de 2). */
  let secondary = $state<number[]>([]);
  let shards = $state<(number | null)[]>([null, null, null]);
  let saving = $state(false);
  let name = $state("");
  /** Página sendo editada (`null` = nova). */
  const editing = $derived(app.editor?.pageId != null ? app.championPages.find((p) => p.id === app.editor?.pageId) : undefined);

  const styleById = (id: number | null) => styles.find((s) => s.id === id);
  const priStyle = $derived(styleById(pri));
  const subStyle = $derived(styleById(sub));
  const rowOf = (s: Style | undefined, id: number) => s?.slots.findIndex((r) => r.includes(id)) ?? -1;

  // Abre com a página editada ou, numa nova, com a página em tela (a sua, a mais jogada ou a de maior WR).
  let wasOpen = false;
  $effect(() => {
    const open = !!app.editor;
    if (open && !wasOpen) {
      const set = editing ? ownRuneSet(editing) : app.runeSet;
      name = editing?.name ?? "";
      const p = set ? styles.find((s) => s.perks.includes(set.primary[0])) : undefined;
      const s2 = set ? styles.find((s) => s.id !== p?.id && set.secondary.every((x) => s.perks.includes(x))) : undefined;
      pri = p?.id ?? styles[0]?.id ?? null;
      primary = p && set ? [...set.primary] : [null, null, null, null];
      sub = s2?.id ?? null;
      secondary = s2 && set ? [...set.secondary] : [];
      shards = set ? [...set.shards] : [null, null, null];
      app.ownError = null;
    }
    wasOpen = open;
  });

  function choosePrimary(id: number) {
    if (id === pri) return;
    pri = id;
    primary = [null, null, null, null];
    if (sub === id) {
      sub = null;
      secondary = [];
    }
  }
  function chooseSub(id: number) {
    if (id === sub || id === pri) return;
    sub = id;
    secondary = [];
  }
  function toggleSecondary(id: number) {
    if (secondary.includes(id)) {
      secondary = secondary.filter((x) => x !== id);
      return;
    }
    const row = rowOf(subStyle, id);
    const rest = secondary.filter((x) => rowOf(subStyle, x) !== row);
    secondary = [...rest, id].slice(-2);
  }

  const complete = $derived(
    !!pri && !!sub && primary.every((x) => x) && secondary.length === 2 && shards.every((x) => x),
  );
  const missing = $derived(
    [
      primary.some((x) => !x) ? "uma runa de cada fileira da árvore principal" : null,
      !sub ? "a árvore secundária" : secondary.length < 2 ? "duas runas da secundária" : null,
      shards.some((x) => !x) ? "os 3 fragmentos" : null,
    ].filter(Boolean).join(", "),
  );

  /** WR da runa no lugar `i` da página (0–3 principal, 4–5 secundária, 6–8 fragmentos). */
  function stat(id: number, i: number): string {
    const s = perkStat(app.build?.runeStats, id, i);
    return s ? pct(s.wr) : "";
  }
  function tip(id: number, i: number): string {
    const s = perkStat(app.build?.runeStats, id, i);
    return `${perk(id)?.name ?? id}${s ? ` · WR ${pct(s.wr)} · PR ${pct(s.pr)} · ${num(s.n)} partidas` : ""}`;
  }

  async function save() {
    if (!complete || !pri || !sub) return;
    saving = true;
    // Secundárias na ordem das fileiras, como o cliente grava.
    const sec = [...secondary].sort((a, b) => rowOf(subStyle, a) - rowOf(subStyle, b));
    const ok = await app.saveOwnRunesPage(app.editor?.pageId ?? null, name, pri, sub, [...primary, ...sec, ...shards] as number[]);
    saving = false;
    if (ok) app.editor = null;
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && app.editor) app.editor = null;
  }
</script>

<svelte:window onkeydown={onKey} />

{#snippet rune(id: number, i: number, on: boolean, pick: () => void, cls = "")}
  <button class="rune {cls}" class:on onclick={pick} title={tip(id, i)}>
    <img src={icon(perk(id)?.icon)} alt={perk(id)?.name ?? ""} />
    <span class="wr">{stat(id, i)}</span>
  </button>
{/snippet}

{#if app.editor}
  <div class="backdrop" role="presentation" onclick={() => (app.editor = null)}></div>
  <div class="dialog" role="dialog" aria-label="Montar a minha página de runas">
    <div class="head">
      <h2>{editing ? "Editar página" : "Nova página"} · {app.champ?.name ?? "campeão"}</h2>
      <input class="name" bind:value={name} maxlength="20" placeholder={editing?.name ?? `Página ${app.championPages.length + 1}`}
        title="Nome da página (até 20 letras); vai no nome da página no cliente: LC: {app.champ?.name ?? ''} <nome>" />
      <button class="close" onclick={() => (app.editor = null)} title="Fechar (Esc)">✕</button>
    </div>
    {#if !styles.length}
      <p class="body muted">Sem os dados das runas: abra o cliente do League uma vez com o app aberto (eles ficam guardados para depois).</p>
    {:else}
      <div class="body cols">
        <section>
          <div class="trees">
            {#each styles as s (s.id)}
              <button class="tree" class:on={s.id === pri} onclick={() => choosePrimary(s.id)} title={s.name}><img src={icon(s.icon)} alt={s.name} /></button>
            {/each}
          </div>
          {#if priStyle}
            {#each priStyle.slots.slice(0, 4) as row, r (r)}
              <div class="row" class:keys={r === 0}>
                {#each row as id (id)}{@render rune(id, r, primary[r] === id, () => (primary[r] = id), r === 0 ? "key" : "")}{/each}
              </div>
            {/each}
          {/if}
        </section>
        <section>
          <div class="trees">
            {#each styles as s (s.id)}
              <button class="tree" class:on={s.id === sub} disabled={s.id === pri} onclick={() => chooseSub(s.id)} title={s.name}><img src={icon(s.icon)} alt={s.name} /></button>
            {/each}
          </div>
          {#if subStyle}
            {#each subStyle.slots.slice(1, 4) as row, r (r)}
              <div class="row">{#each row as id (id)}{@render rune(id, 4, secondary.includes(id), () => toggleSecondary(id))}{/each}</div>
            {/each}
          {:else}
            <p class="muted small">Escolha a árvore secundária.</p>
          {/if}
          <div class="shards">
            {#each shardSlots.slice(0, 3) as row, r (r)}
              <div class="row">{#each row as id (id)}{@render rune(id, 6 + r, shards[r] === id, () => (shards[r] = id), "shard")}{/each}</div>
            {/each}
          </div>
        </section>
      </div>
      <div class="foot">
        <span class="muted small">
          {#if app.ownError}<span class="bad">{app.ownError}</span>
          {:else if !complete}Falta: {missing}.
          {:else}Fica salva só neste computador (funciona sem o cliente aberto).{/if}
          {#if app.build?.runeStats}WR de cada runa naquele lugar da página, no build em tela.{/if}
        </span>
        <button onclick={() => (app.editor = null)}>Cancelar</button>
        <button class="primary" disabled={!complete || saving} onclick={save}>{saving ? "Salvando…" : "Salvar como minha"}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; background: rgb(0 0 0 / 0.55); z-index: 20; }
  .dialog { position: fixed; z-index: 21; top: 16px; left: 50%; transform: translateX(-50%); width: min(660px, calc(100vw - 16px));
    max-height: calc(100vh - 32px); display: grid; grid-template-rows: auto 1fr auto; background: var(--panel); border: 1px solid var(--gold-dim); border-radius: 8px; }
  .head { display: flex; gap: 10px; align-items: center; padding: 8px 14px; border-bottom: 1px solid var(--line); }
  .head .name { flex: 1; min-width: 0; max-width: 220px; margin-left: auto; font-size: 13px; }
  h2 { margin: 0; font-size: 14px; color: var(--gold); }
  .close { border: none; background: none; color: var(--muted); font-size: 16px; padding: 0 4px; }
  .body { padding: 10px 14px; overflow: auto; }
  .cols { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
  @media (max-width: 460px) { .cols { grid-template-columns: 1fr; } }
  section { display: grid; gap: 6px; align-content: start; }
  .trees { display: flex; gap: 4px; margin-bottom: 4px; }
  .tree { padding: 3px; border-radius: 50%; line-height: 0; opacity: 0.55; }
  .tree.on { opacity: 1; border-color: var(--gold); }
  .tree img { width: 22px; height: 22px; }
  .row { display: flex; gap: 6px; padding: 3px 0; border-top: 1px solid var(--line); }
  .rune { display: grid; justify-items: center; gap: 1px; padding: 2px; border-color: transparent; background: none; width: 52px; }
  .rune img { width: 30px; height: 30px; border-radius: 50%; opacity: 0.45; filter: grayscale(0.7); background: var(--panel-2); }
  .rune.key img { width: 38px; height: 38px; }
  .rune.shard img { width: 20px; height: 20px; }
  .rune.on img { opacity: 1; filter: none; box-shadow: 0 0 0 2px var(--gold); }
  .rune:hover img { opacity: 0.9; }
  .wr { font-size: 10px; color: var(--muted); font-variant-numeric: tabular-nums; min-height: 13px; }
  .rune.on .wr { color: var(--text); }
  .shards { margin-top: 4px; }
  .foot { display: flex; gap: 8px; align-items: center; padding: 8px 14px; border-top: 1px solid var(--line); }
  .foot > span { flex: 1; }
  .muted { color: var(--muted); }
  .small { font-size: 11px; }
  .bad { color: var(--bad); }
</style>
