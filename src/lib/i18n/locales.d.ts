// Módulo gerado pelo `compactLocales` de `vite.config.js` a partir de `locales/*.json`.
declare module "virtual:locales" {
  /** Chaves do `en.json`, na ordem. */
  export const keys: string[];
  /** Textos do `en.json`, na ordem de `keys`. */
  export const en: string[];
  /** Por arquivo ("pt_BR", "ja"…): textos na ordem de `keys` (buraco = o do inglês) + chaves só dele. */
  export const loaders: Record<string, () => Promise<{ default: [(string | undefined)[], Record<string, string>] }>>;
}
