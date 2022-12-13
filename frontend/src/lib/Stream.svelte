<script lang="ts">
    import { onMount } from "svelte";

    export let stream: MediaStream;

    let video;
    onMount(() => {
        video.srcObject = stream;
    });
</script>

<div class="aspect-video bg-black relative overflow-hidden rounded-lg justify-items-center video">
    <header class="absolute z-10 p-2 backdrop-blur-md">{stream.id}</header>

    <!-- svelte-ignore a11y-media-has-caption -->
    <video autoplay muted class="w-full h-full z-0" bind:this={video} />
</div>
