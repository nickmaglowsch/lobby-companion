import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
// @ts-expect-error type error without @types/node package
import { readdirSync, readFileSync } from "node:fs";
const host = process.env.TAURI_DEV_HOST;

/**
 * Traduções da janela sem as chaves repetidas (`virtual:locales`, usado por `src/lib/i18n`).
 * Cada idioma vira a lista dos textos na ordem das chaves do `en.json` (buraco = texto igual ao
 * inglês ou ausente, que já caem no inglês) + as chaves que só ele tem (formas de plural); o
 * módulo principal leva as chaves uma vez só. Os arquivos JSON continuam iguais (os testes do
 * `i18n.rs` leem eles).
 */
function compactLocales() {
  // Caminho absoluto: no `vite dev`, `addWatchFile` com caminho relativo vira um import a resolver.
  const DIR = `${process.cwd().replace(/\\/g, "/")}/src/lib/i18n/locales/`;
  const ID = "virtual:locales";
  /** @param {string} file */
  const read = (file) => /** @type {Record<string, string>} */ (JSON.parse(readFileSync(DIR + file + ".json", "utf8")));
  return {
    name: "compact-locales",
    /** @param {string} id */
    resolveId: (id) => (id === ID || id.startsWith(ID + "/") ? "\0" + id : undefined),
    /** @param {string} id @this {{ addWatchFile(f: string): void }} */
    load(id) {
      if (!id.startsWith("\0" + ID)) return;
      this.addWatchFile(DIR + "en.json");
      const en = read("en");
      const keys = Object.keys(en);
      const file = id.slice(ID.length + 2);
      if (!file) {
        const files = readdirSync(DIR).filter((/** @type {string} */ f) => f.endsWith(".json") && f !== "en.json");
        const loaders = files.map((/** @type {string} */ f) => `${JSON.stringify(f.slice(0, -5))}: () => import("${ID}/${f.slice(0, -5)}")`);
        return `export const keys = ${JSON.stringify(keys)};\nexport const en = ${JSON.stringify(Object.values(en))};\nexport const loaders = {${loaders.join(",")}};`;
      }
      this.addWatchFile(DIR + file + ".json");
      const dict = read(file);
      const values = keys.map((k) => (dict[k] === undefined || dict[k] === en[k] ? "" : JSON.stringify(dict[k])));
      const extra = Object.fromEntries(Object.entries(dict).filter(([k]) => !(k in en)));
      return `export default [[${values.join(",")}], ${JSON.stringify(extra)}];`;
    },
  };
}

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [compactLocales(), sveltekit()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
