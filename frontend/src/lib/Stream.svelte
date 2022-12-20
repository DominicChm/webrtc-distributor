<script lang="ts">
    import { onMount } from "svelte";

    export let stream: MediaStream;

    let video;
    onMount(() => {
        video.srcObject = stream;
    });
</script>

<div class="aspect-video bg-black relative overflow-hidden rounded-2xl justify-items-center video min-h-0 min-w-0">
    <header class="absolute z-10 p-2 backdrop-blur-md backdrop-brightness-90 rounded-br-2xl">{stream.id}</header>

    <!-- svelte-ignore a11y-media-has-caption -->
    <video autoplay muted class="w-full h-full 
    z-0 min-w-0 min-h-0 object-contain;" bind:this={video} />
</div>
