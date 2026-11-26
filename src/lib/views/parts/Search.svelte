<script lang="ts">
  // Busca de campeão (mostra o build na janela; na seleção, só importa o do seu campeão).
  import { app } from "$lib/app.svelte";

  let { width = 160, placeholder = "Buscar campeão…" }: { width?: number; placeholder?: string } = $props();
  let text = $state("");
  const id = `champs-${Math.random().toString(36).slice(2, 8)}`;
</script>

<form onsubmit={(e) => { e.preventDefault(); if (app.search(text)) text = ""; }}>
  <input list={id} {placeholder} bind:value={text} disabled={!app.assets} style="width: {width}px" />
  <datalist {id}>{#each app.champions as c (c.id)}<option value={c.name}></option>{/each}</datalist>
</form>

<style>
  form { display: contents; }
  input { background: var(--panel-2); color: var(--text); border: 1px solid var(--gold-dim); border-radius: 4px; padding: 3px 6px; font: inherit; font-size: 13px; }
</style>
