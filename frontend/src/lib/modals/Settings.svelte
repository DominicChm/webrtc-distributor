<script>
    import { fade } from "svelte/transition";
    import { settings_active } from "../../stores/modals";
    import { stats_active, auto_add } from "../../stores/settings";

    function close() {
        $settings_active = false;
    }
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<div class="modal modal-open" on:click={close} transition:fade={{ duration: 100 }}>
    <div class="modal-box prose w-11/12 max-w-5xl" on:click|stopPropagation>
        <label for="my-modal-3" class="btn btn-sm btn-circle absolute right-2 top-2" on:click={close}>✕</label>

        <h1>Settings</h1>

        <div class="setting">
            <div>
                <h4>STATS</h4>
                <p >Enable server statistics window</p>
            </div>
            <div>
                <input type="checkbox" class="toggle toggle-secondary" bind:checked={$stats_active}/>
            </div>
        </div>

        <div class="divider" />

        <div class="setting">
            <div>
                <h4>AUTO-ADD (todo)</h4>
                <p>Enable automatic addition of all streams</p>
            </div>
            <div>
                <input type="checkbox" class="toggle toggle-secondary " disabled bind:checked={$auto_add}/>
            </div>
        </div>
    </div>
</div>

<style>
    .setting {
        display: grid;
        grid-template-columns: auto min-content;
        align-items: center;
    }

    .setting p,h4 {
        margin-top: 0;
    }

    .setting p {
        margin-bottom: 0;
    }
</style>