<script>
    import Stats from "./Stats.svelte";
    import { ChartBar } from "phosphor-svelte";
    import { Cpu, Question, Broadcast } from "phosphor-svelte";
    import { stats_active } from "./stores/stores";
</script>

<div class="navbar bg-base-100 shadow-md">
    <div class="flex-1">
        <button class="btn btn-ghost normal-case text-xl prose">CHANGEME</button>
    </div>
    <div class="flex-none">
        <ul class="menu menu-horizontal px-1 gap-2">
            <div class="btn-group">
                <!-- <button class="btn">Button</button> -->

                <button class="btn tooltip tooltip-bottom btn-active" data-tip="Stats" class:btn-active={$stats_active} on:click={() => ($stats_active = !$stats_active)}>
                    <ChartBar size="25" />
                </button>
            </div>
            <div class="btn-group">
                <button class="btn tooltip tooltip-bottom" data-tip="Help">
                    <Question size="25" />
                </button>
                <button class="btn tooltip tooltip-bottom" data-tip="Streams">
                    <Broadcast size="25" />
                </button>
            </div>
        </ul>
    </div>
</div>
