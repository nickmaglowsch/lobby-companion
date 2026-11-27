<script lang="ts">
  import { pct, pts, type Trend } from "./api";
  import { i18n, t } from "./i18n/index.svelte";

  // Tendência do WR nos últimos ~30 dias (linha `all` do lolalytics = todos os elos).
  let { trend }: { trend: Trend | null } = $props();

  const W = 120;
  const H = 28;
  const delta = $derived(trend ? trend.wrLast7 - trend.wrPeriod : 0);

  // Datas do lolalytics vêm como "AAAA-MM-DD"; outro formato aparece como veio.
  const dateFmt = $derived(new Intl.DateTimeFormat(i18n.tag, { day: "numeric", month: "short", timeZone: "UTC" }));
  function date(s: string | undefined): string {
    if (!s) return "";
    const d = new Date(`${s}T00:00:00Z`);
    return Number.isNaN(d.getTime()) ? s : dateFmt.format(d);
  }

  function line(values: number[], lo: number, hi: number): string {
    const span = hi - lo || 1;
    const step = values.length > 1 ? W / (values.length - 1) : 0;
    return values.map((v, i) => `${(i * step).toFixed(1)},${(H - 2 - ((v - lo) / span) * (H - 4)).toFixed(1)}`).join(" ");
  }

  const paths = $derived.by(() => {
    if (!trend || trend.wr.length < 2) return null;
    // Dias com poucas partidas (o de hoje, incompleto) puxam a escala: usamos o suavizado para os limites.
    const all = trend.wrs.length ? trend.wrs : trend.wr;
    const lo = Math.min(...all) - 0.5;
    const hi = Math.max(...all) + 0.5;
    const clamp = (v: number) => Math.min(hi, Math.max(lo, v));
    return { raw: line(trend.wr.map(clamp), lo, hi), smooth: line(all, lo, hi) };
  });
</script>

{#if trend && paths}
  <div class="trend" title={t("trend.title", { from: date(trend.dates[0]), to: date(trend.dates[trend.dates.length - 1]) })}>
    <svg width={W} height={H} viewBox="0 0 {W} {H}" aria-hidden="true">
      <polyline class="raw" points={paths.raw} />
      <polyline class="smooth" points={paths.smooth} />
    </svg>
    <span>
      WR <b class:up={delta >= 0.5} class:down={delta <= -0.5}>{t("lib.points", { value: pts(delta, 1) })}</b>
      <span class="muted">{t("trend.compare", { last7: pct(trend.wrLast7), days: trend.dates.length, period: pct(trend.wrPeriod) })}</span>
    </span>
  </div>
{/if}

<style>
  .trend { display: flex; align-items: center; gap: 8px; margin-top: 6px; font-size: 12px; flex-wrap: wrap; }
  svg { flex: none; background: var(--panel); border-radius: 3px; }
  polyline { fill: none; stroke-linejoin: round; stroke-linecap: round; }
  .raw { stroke: var(--muted); stroke-width: 1; opacity: 0.5; }
  .smooth { stroke: var(--gold); stroke-width: 1.6; }
</style>
