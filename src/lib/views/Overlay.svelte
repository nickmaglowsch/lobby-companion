<script lang="ts">
  // Overlay da partida (`overlay.rs`): janela pequena, transparente e sem clique por cima do jogo.
  // Mostra só os seus dados, da API oficial do jogo: o próximo item da build e o ouro que falta,
  // o CS por minuto e quanto falta para os inibidores destruídos voltarem.
  import { onMount } from "svelte";
  import { api, icon, type OverlayState } from "$lib/api";
  import { fixed, num, setLocale, t } from "$lib/i18n/index.svelte";

  let st = $state<OverlayState | null>(null);
  let locale = "";

  async function apply(s: OverlayState) {
    if (s.locale && s.locale !== locale) {
      locale = s.locale;
      await setLocale(s.locale);
    }
    st = s;
  }

  onMount(() => {
    document.documentElement.classList.add("overlay-window");
    let un: (() => void) | undefined;
    api.overlay().then(apply);
    api.onOverlay(apply).then((u) => (un = u));
    return () => un?.();
  });

  /** 134 → "2:14". */
  const clock = (s: number) => {
    const x = Math.max(0, Math.ceil(s));
    return `${Math.floor(x / 60)}:${String(x % 60).padStart(2, "0")}`;
  };
  const progress = $derived(
    st?.next && st.next.cost > 0 ? Math.min(1, Math.max(0, (st.next.cost - st.next.missing) / st.next.cost)) : 0,
  );
</script>

{#if st?.visible}
  <div class="ov">
    {#if st.next}
      <div class="next" class:ready={st.next.missing === 0}>
        {#if st.next.icon}<img src={icon(st.next.icon)} alt="" />{/if}
        <div class="txt">
          <div class="label">{t("overlay.next")}</div>
          <div class="name">{st.next.name}</div>
          <div class="gold">{st.next.missing === 0 ? t("overlay.ready") : t("overlay.missing", { gold: st.next.missing })}</div>
          <div class="bar"><span style:width={`${progress * 100}%`}></span></div>
        </div>
      </div>
    {/if}
    <div class="row">
      <span class="label">{t("overlay.csMin")}</span>
      <span><b>{fixed(st.csPerMin, 1)}</b> <span class="muted">({num(st.cs)})</span></span>
    </div>
    {#each st.inhibs as i (`${i.side}:${i.lane}`)}
      <div class="row inhib {i.side}">
        <span>{t("overlay.inhib", { side: t(`overlay.${i.side}`), lane: t(`overlay.${i.lane}`) })}</span>
        <b>{clock(i.respawnIn)}</b>
      </div>
    {/each}
  </div>
{/if}

<style>
  :global(html.overlay-window), :global(html.overlay-window body) { background: transparent !important; overflow: hidden; }
  .ov {
    box-sizing: border-box; width: 240px; margin: 2px; padding: 8px 10px;
    background: rgba(1, 10, 19, 0.82); border: 1px solid var(--gold-dim); border-radius: 6px;
    font-size: 12px; line-height: 1.35; user-select: none;
  }
  .next { display: flex; gap: 8px; align-items: flex-start; padding-bottom: 6px; margin-bottom: 4px; border-bottom: 1px solid var(--line); }
  .next img { width: 36px; height: 36px; border: 1px solid var(--gold-dim); border-radius: 3px; flex: none; }
  .txt { flex: 1; min-width: 0; }
  .label { color: var(--muted); font-size: 11px; }
  .name { color: var(--gold); font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .gold { color: var(--text); }
  .next.ready .gold { color: var(--good); font-weight: 600; }
  .bar { height: 3px; margin-top: 3px; background: var(--line); border-radius: 2px; overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--gold); }
  .next.ready .bar span { background: var(--good); }
  .row { display: flex; justify-content: space-between; gap: 8px; margin-top: 2px; }
  .muted { color: var(--muted); }
  .inhib.enemy b, .inhib.red b { color: var(--bad); }
  .inhib.ally b, .inhib.blue b { color: #4fa3ff; }
</style>
