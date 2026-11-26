// Só no navegador (`npm run dev`, fora do Tauri): completa os dados sintéticos com nomes e ícones
// do CommunityDragon, que espelha os mesmos arquivos do cliente (`/lol-game-data/assets/...`).
// No app de verdade os ícones vêm do próprio cliente pelo protocolo `lcu`.
import type { Assets } from "./api";

const CD = "https://raw.communitydragon.org/latest/plugins/rcp-be-lol-game-data/global/";

/** Caminho do cliente → URL do CommunityDragon (lá tudo é minúsculo). */
export function cdragonUrl(path: string): string {
  return CD + "default/" + path.replace(/^\/lol-game-data\/assets\//i, "").toLowerCase();
}

interface Raw { id: number; name: string; iconPath: string }

async function load<T>(file: string): Promise<T | null> {
  try {
    const r = await fetch(`${CD}pt_br/v1/${file}`);
    return r.ok ? ((await r.json()) as T) : null;
  } catch {
    return null;
  }
}

/** Troca nome e ícone dos itens, runas, árvores e feitiços que o mock já tem. */
export async function withIcons(a: Assets): Promise<Assets> {
  const [items, perks, styles, spells] = await Promise.all([
    load<Raw[]>("items.json"),
    load<Raw[]>("perks.json"),
    load<{ styles: Raw[] }>("perkstyles.json"),
    load<Raw[]>("summoner-spells.json"),
  ]);
  const fill = (target: Record<string, { name: string; icon: string }>, list: Raw[] | null | undefined) => {
    for (const r of list ?? []) {
      const t = target[String(r.id)];
      if (t) {
        t.name = r.name;
        t.icon = r.iconPath;
      }
    }
  };
  fill(a.items, items);
  fill(a.perks, perks);
  fill(a.spells, spells);
  for (const s of a.styles) {
    const r = styles?.styles.find((x) => x.id === s.id);
    if (r) s.icon = r.iconPath;
  }
  return a;
}
