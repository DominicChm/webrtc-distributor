import { derived, readable, writable } from "svelte/store";
import type { API_STATS, stats_t } from "./net";

export let stats = readable<null | stats_t>(null, (set) => {
    async function update() {
        try {
            let s = await (await fetch("/api/stats")).json();
            set(s);
        } catch (e) {
            set(null);
        }
    }
    update();
    let i = setInterval(update, 1000);

    return () => {
        clearInterval(i);
    }
})

export let cpu_percent = derived(stats, (v) => {
    if (v)
        return Math.round(v.system_status.cpu_used).toString().padStart(2, "0");
    else
        return "??";
});

export let proc_cpu_percent = derived(stats, (v) => {
    if (v)
        return Math.round(v.system_status.proc_cpu).toString().padStart(2, "0");
    else
        return "??";
});

export let mem_percent = derived(stats, (v) => {
    if (v)
        return Math.round(v.system_status.mem_used / v.system_status.mem_total * 100).toString().padStart(2, "0");
    else
        return "??";
});

export let proc_mem_pretty = derived(stats, (v) => {
    if (v)
        return fmtBytes(v.system_status.proc_mem, 1);
    else
        return "??";
});

export let num_clients = derived(stats, (v) => {
    if (v)
        return v.clients;
    else
        return "??";
});

export function fmtBytes(bytes, decimals = 2) {
    if (!+bytes) return '0 Bytes'

    const k = 1024
    const dm = decimals < 0 ? 0 : decimals
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']

    const i = Math.floor(Math.log(bytes) / Math.log(k))

    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`
}