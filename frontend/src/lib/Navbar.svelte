<script>
    import Logo1 from "../assets/logo1-dark.svg";
    import { Question, Broadcast, Gear } from "phosphor-svelte";
    import { stats_active } from "../stores/settings";
    import { settings_active, streams_active} from "../stores/modals";

    function open_settings() {
        $settings_active = true;
    }

    function open_streams() {
        $streams_active = true;
    }
</script>

<div class="navbar bg-base-100 shadow-md z-40 relative">
    <div class="flex-1">
        <button class="btn btn-ghost normal-case text-xl prose">
            <img src={Logo1} alt="Logo" class="inline-block h-full m-0" />

            <h1 class="inline-block m-0 text-3xl ml-2 mb-1">EasyStreamer</h1>
        </button>
    </div>
    <div class="flex-none">
        <ul class="menu menu-horizontal px-1 gap-2">
            <div class="btn-group">
                <button class="btn tooltip tooltip-bottom" data-tip="Settings" class:btn-active={$settings_active} on:click={open_settings}>
                    <Gear size="25" />
                </button>
                <button class="btn tooltip tooltip-bottom" data-tip="Help">
                    <Question size="25" />
                </button>
                <button class="btn btn-primary tooltip tooltip-bottom" data-tip="Streams" on:click={open_streams}>
                    <Broadcast size="25" />
                </button>
            </div>
        </ul>
    </div>
</div>
