import { derived, readable, writable } from "svelte/store";
import { API_STREAMS, type stream_def_t } from "./net";

export let stream_ids = writable([]);


export let stream_defs = readable<null | stream_def_t>(null, (set) => {
    async function update() {
        try {
            let s = await (await fetch(API_STREAMS)).json();
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
});

