<script lang="ts">
  
  import Navbar from "./Navbar.svelte";
  import Stats from "./Stats.svelte";
  import StreamContainer from "./StreamContainer.svelte";
  import {stats_active} from "../stores/stats"
</script>

<main class="preview h-full flex-1 min-w-max relative overflow-hidden">
  <Navbar />
  {#if $stats_active}
    <Stats />
  {/if}
  <div class="z-1 overflow-y-auto min-h-0 max-h-full">
    <StreamContainer />
  </div>
</main>

<style>
</style>
