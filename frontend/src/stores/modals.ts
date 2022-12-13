import { writable } from "svelte/store";

export let settings_active = writable(false);
export let streams_active = writable(false);