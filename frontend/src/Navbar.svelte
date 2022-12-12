<script>
    import Stats from "./Stats.svelte";
    import {ChartBar} from "phosphor-svelte"
    import {Cpu} from "phosphor-svelte"

</script>

<div class="navbar bg-base-100">
    <div class="flex-1">
        <a class="btn btn-ghost normal-case text-xl">daisyUI</a>
    </div>
    <div class="flex-none">
        <ul class="menu menu-horizontal px-1">
            <div class="btn-group">
                <button class="btn btn-active">Button</button>
                <button class="btn"><ChartBar size="25"/></button>
                <button class="btn"><Cpu size="25"/></button>
                <button class="btn">Button</button>
            </div>
            <li><button>i frew up</button></li>
        </ul>
    </div>
</div>
