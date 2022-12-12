<script>
    import { Cpu, MonitorPlay, Stack } from "phosphor-svelte";
    import { cpu_percent, proc_cpu_percent, mem_percent, proc_mem_pretty, stats_active } from "./stores/stats";
    import { fly } from "svelte/transition";
    //https://www.xicons.org/#/
</script>

<div class="shadow fixed bottom-3 md:right-3 z-40 md:left-auto left-3 right-3 transition-all duration-100 flex justify-center" transition:fly={{ y: 10, duration: 100 }}>
    <div class="stats">
        <div class="stat relative">
            <div class="stat-figure text-secondary">
                <MonitorPlay size="40" />
            </div>
            <div class="stat-title">Watchers</div>
            <div class="stat-value">???</div>
            <div class="stat-desc" />

            <div class="absolute top-0 left-0 right-0 bottom-0 opacity-0 hover:opacity-100 bg-base-100 transition-all duration-100">
                <div class="stat">More watcher stats here...</div>
            </div>
        </div>

        <div class="stat relative">
            <div class="stat-figure text-secondary">
                <Cpu size="40" />
            </div>
            <div class="stat-title">CPU Usage</div>
            <div class="stat-value">{$cpu_percent}%</div>
            <div class="stat-desc">({$proc_cpu_percent}%)</div>
            <div class="absolute top-0 left-0 right-0 bottom-0 opacity-0 hover:opacity-100 bg-base-100 transition-all duration-100">
                <div class="stat">More CPU stats here...</div>
            </div>
        </div>

        <div class="stat relative">
            <div class="stat-figure text-secondary">
                <Stack size="40" />
            </div>
            <div class="stat-title">RAM Usage</div>
            <div class="stat-value">{$mem_percent}%</div>
            <div class="stat-desc">({$proc_mem_pretty})</div>
            <div class="absolute top-0 left-0 right-0 bottom-0 opacity-0 hover:opacity-100 bg-base-100 transition-all duration-100">
                <div class="stat">More memory stats here...</div>
            </div>
        </div>
    </div>
</div>
