<script>
    import { onMount } from "svelte";
    import { signal, media_streams } from "../stores/rtc";
    import Stream from "./Stream.svelte";

    onMount(signal);
</script>

<div class="grid gap-3 p-3 xl:grid-cols-2 grid-cols-1 justify-center flex-1">
    {#each Object.entries($media_streams) as [id, stream] (id)}
        <Stream {stream}></Stream>
    {/each}
</div>

<style>
    .video {
        max-height: calc(100vh - 5rem);
        object-fit: cover;
    }
</style>
