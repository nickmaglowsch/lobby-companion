// Formatos curtos usados nos layouts compactos.
import { pts } from "../../i18n/index.svelte";

/** Pontos com sinal e duas casas, no formato do idioma ("+1,20" / "−0,35"). */
export const signed = (x: number) => pts(x, 2);
