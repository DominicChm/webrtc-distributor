<script lang="ts">
    import { onMount } from "svelte";

    export let stream: MediaStream;

    let video;
    onMount(() => {
        video.srcObject = stream;
    });
</script>

<div class="aspect-video bg-black relative overflow-hidden rounded-lg justify-items-center video min-h-0 min-w-0">
    <header class="absolute z-10 p-2 backdrop-blur-md">{stream.id}</header>

    <!-- svelte-ignore a11y-media-has-caption -->
    <video autoplay muted class="w-full z-0 min-w-0 min-h-0 width: 100%;" bind:this={video} />
</div>
