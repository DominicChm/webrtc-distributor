<script>
    import { Cpu, MonitorPlay, Stack } from "phosphor-svelte";
    import { stats_active } from "./stores/stores";
    //https://www.xicons.org/#/
</script>

<div
    class="shadow absolute bottom-3 md:right-3 md:left-auto left-3 right-3 transition-all duration-100 opacity-0 flex justify-center"
    class:opacity-100={$stats_active}
>
    <div class="stats">
        <div class="stat relative">
            <div class="stat-figure text-secondary">
                <MonitorPlay size="40" />
            </div>
            <div class="stat-title">Watchers</div>
            <div class="stat-value">1</div>
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
            <div class="stat-value">10%</div>
            <div class="stat-desc">(5%)</div>

            <div class="absolute top-0 left-0 right-0 bottom-0 opacity-0 hover:opacity-100 bg-base-100 transition-all duration-100">
                <div class="stat">More CPU stats here...</div>
            </div>
        </div>

        <div class="stat relative">
            <div class="stat-figure text-secondary">
                <Stack size="40" />
            </div>
            <div class="stat-title">RAM Usage</div>
            <div class="stat-value">1.1GB</div>
            <div class="stat-desc">(14%)</div>

            <div class="absolute top-0 left-0 right-0 bottom-0 opacity-0 hover:opacity-100 bg-base-100 transition-all duration-100">
                <div class="stat">More memory stats here...</div>
            </div>
        </div>
    </div>
</div>
