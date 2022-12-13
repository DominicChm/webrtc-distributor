import { readable } from "svelte/store";
import type { stream_def_t } from "./net";

const uid = uuidv4();

let pc = new RTCPeerConnection({
    iceServers: [
        {
            urls: 'stun:stun.l.google.com:19302'
        }
    ]
})

pc.oniceconnectionstatechange = console.log;
pc.onicecandidate = e => console.log;

pc.addTransceiver('video', { 'direction': 'recvonly' })

export async function signal() {
    if (!pc.localDescription) {
        const lo = await pc.createOffer()
        await pc.setLocalDescription(lo);
    }

    console.log(pc.localDescription);
    let res = await fetch(`/api/signal/${uid}`, {
        method: "POST",
        body: JSON.stringify(pc.localDescription)
    });

    let sd = await res.json();
    console.log(sd);
    if (sd === '') {
        return alert('Session Description must not be empty')
    }

    try {
        pc.setRemoteDescription(new RTCSessionDescription(sd))
    } catch (e) {
        alert(e)
    }
}

export const media_streams = readable({}, (set) => {
    let s = {};
    function on_track(ev: RTCTrackEvent) {
        console.log("NEW TRACK")
        let stream = ev.streams[0]
        let id = stream.id;

        if (s[id]) { return }

        // Init this new media stream
        stream.onremovetrack = on_remove_track;
        s[id] = stream;
        set(s);
    }

    function on_remove_track(ev: MediaStreamTrackEvent) {
        // Removed the video. The mediastream is useless now.
        // Delete it.
        let id = this.id;
        if (ev.track.kind == 'video') {
            delete s[id];
        }
    }

    pc.ontrack = on_track
});


export async function update_peer() {
    let streams = await (await fetch("/api/streams")).json() as stream_def_t;

}

export async function add_stream(id: string) {

}

function uuidv4() {
    return ([1e7] as any + -1e3 + -4e3 + -8e3 + -1e11).replace(/[018]/g, c =>
        (c ^ crypto.getRandomValues(new Uint8Array(1))[0] & 15 >> c / 4).toString(16)
    );
}