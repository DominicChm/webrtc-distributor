<script>
    import { fade } from "svelte/transition";
    import { streams_active } from "../../stores/modals";
    import { stream_defs } from "../../stores/streams";
    import { Pencil, Plus } from "phosphor-svelte";
    function close() {
        $streams_active = false;
    }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="modal modal-open" on:click={close} transition:fade={{ duration: 100 }}>
    <div class="modal-box prose w-11/12 max-w-5xl" on:click|stopPropagation>
        <label for="my-modal-3" class="btn btn-sm btn-circle absolute right-2 top-2" on:click={close}>✕</label>

        <h1>Streams</h1>
        <div class="overflow-x-auto w-full text-lg">
            {#if $stream_defs}
                <table class="table w-full mb-1">
                    <!-- head -->
                    <thead>
                        <tr>
                            <th class="w-0" />
                            <th class="w-1/3">Id</th>
                            <th class="w-1/3">video</th>
                            <th class="w-1/3">audio</th>
                            <th class="w-0" />
                        </tr>
                    </thead>
                    <tbody>
                        {#each $stream_defs as s}
                            <tr>
                                <th>
                                    <label>
                                        <input type="checkbox" class="checkbox" />
                                    </label>
                                </th>

                                <td>
                                    {s.id}
                                </td>

                                <td>
                                    {#if s.video}
                                        {s.video.ip} : {s.video.port}
                                    {:else}
                                        <span class="badge badge-ghost badge-sm">None</span>
                                    {/if}
                                </td>

                                <td>
                                    {#if s.audio}
                                        {s.audio.ip} : {s.audio.port}
                                    {:else}
                                        <span class="badge badge-ghost badge-sm">None</span>
                                    {/if}
                                </td>
                                <th>
                                    <button disabled class="btn btn-square btn-md">
                                        <Pencil size="25"/>
                                    </button>
                                </th>
                                
                            </tr>
                        {/each}
                    </tbody>
                </table>
                <div class="p-1 flex justify-center overflow-hidden">
                    <button class="btn btn-ghost btn-square">
                        <Plus size="25"/>
                    </button>
                </div>
            {:else}
                <div class="flex flex-col justify-center items-center">
                    <progress class="progress w-85 progress-primary	" />
                    <div class="badge badge-primary absolute">Loading...</div>
                </div>
            {/if}
        </div>
    </div>
</div>
