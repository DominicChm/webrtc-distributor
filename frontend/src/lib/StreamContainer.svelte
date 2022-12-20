<script>
    import { onMount } from "svelte";
    import { signal, media_streams, add_stream } from "../stores/streams";
    import Stream from "./Stream.svelte";

    onMount(async () => {
        //add_stream("test_stream");
        //add_stream("test_stream_2");
        //signal(["test_stream"]);
        //signal();
    });
</script>

<div class="grid gap-3 p-3 xl:grid-cols-2 grid-cols-1 justify-center flex-1">
    {#each Object.entries($media_streams) as [id, stream] (id)}
        <Stream {stream} />
    {/each}
</div>
