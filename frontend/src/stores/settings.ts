import { derived, writable } from "svelte/store";
import { selected_stream_ids } from "./streams";

export let stats_active = writable(true);
export let auto_add = writable(false);
