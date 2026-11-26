// Formatos curtos usados nos layouts compactos.
export const signed = (x: number) => `${x >= 0 ? "+" : "−"}${Math.abs(x).toFixed(2).replace(".", ",")}`;
