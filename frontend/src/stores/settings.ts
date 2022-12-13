import { writable } from "svelte/store";

export let stats_active = writable(true);
export let auto_add = writable(false);
export let stream_ids = writable([]);