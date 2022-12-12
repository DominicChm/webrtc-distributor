import { writable } from "svelte/store";

export let stats_active = writable(true);

export let stats_cpu = writable(0);
export let stats_proc_cpu = writable(0);

export let stats_memory = writable(0);
export let stats_proc_memory = writable(0);

export let stats_clients = writable(0);

export function fmtBytes(bytes, decimals = 2) {
    if (!+bytes) return '0 Bytes'

    const k = 1024
    const dm = decimals < 0 ? 0 : decimals
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB']

    const i = Math.floor(Math.log(bytes) / Math.log(k))

    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`
}