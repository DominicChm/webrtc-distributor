import { derived, get, readable, writable } from "svelte/store";
import { API_STREAMS, type stream_def_t } from "./net";

const uid = uuidv4();

export let selected_stream_ids = writable([]);
let transceivers = {};

// Contains result of API_STREAMS, polled at 1hz
export let stream_defs = readable<null | stream_def_t[]>(null, (set) => {
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

// ========= WEBRTC ========== //

let pc = new RTCPeerConnection({
    iceServers: [
        {
            urls: 'stun:stun.l.google.com:19302'
        }
    ]
})

pc.oniceconnectionstatechange = console.log;
pc.onicecandidate = e => console.log;

let batched_signal_timeout = null;
export async function signal(streams: string[]) {
    const lo = await pc.createOffer()
    await pc.setLocalDescription(lo);

    console.log(pc.localDescription);

    let res = await fetch(`/api/signal`, {
        method: "POST",
        body: JSON.stringify({
            uid,
            stream_ids: streams,
            offer: pc.localDescription
        })
    });

    let sd = await res.json();
    console.log(sd);

    try {
        pc.setRemoteDescription(new RTCSessionDescription(sd))
    } catch (e) {
        console.error(e)
    }
}

export const media_streams = readable({}, (set) => {
    let s = {};

    function on_track_unmute(id, stream) {
        s[id] = stream;
        set(s);
    }

    function on_track_mute(id, stream) {
        delete s[id];
        set(s);
    }

    function on_track(ev: RTCTrackEvent) {
        console.log("NEW TRACK")
        console.log(ev);

        let stream = ev.streams[0]

        if (!stream) return;

        let id = stream.id;

        if (s[id]) { return }

        // Init this new media stream
        //stream.onremovetrack = on_remove_track;
        ev.track.onunmute = () => on_track_unmute(id, stream);
        ev.track.onmute = () => on_track_mute(id, stream);

        // Starts the stream
        fetch(`/api/resync`, {
            method: "POST",
            body: JSON.stringify({
                uid,
                stream_ids: [stream.id],
            })
        });
    }

    function on_remove_track(ev: MediaStreamTrackEvent) {
        // Removed the video. The parent mediastream is useless now.
        // Delete it.
        console.log(`REMOVE TRACK ${this.id}`)
        let id = this.id;
        if (ev.track.kind == 'video') {
            delete s[id];
        }
        set(s);
    }

    pc.ontrack = on_track;
});

selected_stream_ids.subscribe(ids => {
    if (batched_signal_timeout) {
        clearTimeout(batched_signal_timeout);
    }
    signal(ids);
});


export async function add_stream(id: string) {
    if (pc.getTransceivers().length <= get(selected_stream_ids).length) {
        pc.addTransceiver('video');
    }
    selected_stream_ids.update(i => [...i, id]);
}

export async function remove_stream(id: string) {
    let t = transceivers[id]
    if (t) {
        t.stop();
    }
    selected_stream_ids.update(i => i.filter(d => d != id));
}

export async function add_streams(ids: string[]) {
    for (let i of ids) {
        transceivers[i] = pc.addTransceiver('video', { 'direction': 'recvonly' });
    }
    selected_stream_ids.update(i => [...i, ...ids]);
}

function uuidv4() {
    return ([1e7] as any + -1e3 + -4e3 + -8e3 + -1e11).replace(/[018]/g, c =>
        (c ^ crypto.getRandomValues(new Uint8Array(1))[0] & 15 >> c / 4).toString(16)
    );
}