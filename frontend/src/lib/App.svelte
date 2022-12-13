<script lang="ts">
  import Navbar from "./Navbar.svelte";
  import Stats from "./Stats.svelte";
  import StreamContainer from "./StreamContainer.svelte";
  import { stats_active } from "../stores/settings";
  import { settings_active, streams_active } from "../stores/modals";
  import Settings from "./modals/Settings.svelte";
  import Streams from "./modals/Streams.svelte";
</script>

<main class="preview h-full max-w-full flex-1 relative overflow-hidden">
  <Navbar />

  <!-- Handle visibility HERE so that, if not visible, relevant APIs aren't being polled -->
  {#if $settings_active}
    <Settings />
  {:else if $streams_active}
    <Streams />
  {/if}

  {#if $stats_active}
    <Stats />
  {/if}
  <div class="z-1 overflow-y-auto min-h-0 max-h-full">
    <StreamContainer />
  </div>
</main>

<style>
</style>
