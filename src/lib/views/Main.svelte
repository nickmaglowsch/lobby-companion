<script lang="ts">
  // Janela principal: escolhe o layout pela configuração (Por fase, Grade, Abas ou Clássico).
  // O estado e as ações ficam em `$lib/app.svelte.ts`, compartilhados pelos layouts.
  import { onMount } from "svelte";
  import { app } from "$lib/app.svelte";
  import UpdateBanner from "$lib/UpdateBanner.svelte";
  import Classic from "./Classic.svelte";
  import Grid from "./Grid.svelte";
  import Tabs from "./Tabs.svelte";
  import Phase from "./Phase.svelte";
  import SettingsDialog from "./parts/SettingsDialog.svelte";
  import RuneEditor from "./parts/RuneEditor.svelte";

  onMount(() => {
    let un: (() => void) | undefined;
    let gone = false;
    // Desmontada antes de o `init` terminar: para de ouvir assim que ele devolver.
    app.init().then((u) => (gone ? u() : (un = u)));
    return () => {
      gone = true;
      un?.();
    };
  });
</script>

{#if app.settings}
  {#if app.layout === "classic"}
    <div class="classic-banner"><UpdateBanner inChampSelect={app.snap?.inChampSelect ?? false} /></div>
    <Classic />
  {:else}
    {#if app.layout === "grid"}<Grid />{:else if app.layout === "tabs"}<Tabs />{:else}<Phase />{/if}
    <UpdateBanner inChampSelect={app.snap?.inChampSelect ?? false} floating />
    <SettingsDialog />
  {/if}
  <RuneEditor />
{/if}

<style>
  .classic-banner { padding: 10px 12px 0; }
  .classic-banner:empty { display: none; }
</style>
