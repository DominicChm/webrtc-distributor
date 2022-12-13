import { readable } from "svelte/store";

type track_def_t = {
    port: number,
    ip: string,
    codec: string
};

type stream_def_t = {
    id: string,
    default: boolean,
    video?: track_def_t,
    audio?: track_def_t
}[];

export let stream_defs = readable<null | stream_def_t>(null, (set) => {
    async function update() {
        try {
            let s = await (await fetch("/api/streams")).json();
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
