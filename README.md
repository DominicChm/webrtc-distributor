# EasyStreamer
Download ffmpeg.exe and ffprobe.exe and put in root project dir

# Quickstart

# Why?
... same as old archive

# Interacting with EasyStreamer
You have two options for interacting with EasyStreamer. For quick projects that don't need more than basic stream viewing, using the internal webserver is the fastest way to get started. For more complicated projects, or for embedding into an existing web UI, a JSON api is provided through STDIN and STDOUT.

# Configuration
... JSON config file.

# Troubleshooting
| Behavior                   | Cause         | Solution                                         |
| -------------------------- | ------------- | ------------------------------------------------ |
| No video is coming through | Wrong port/IP | Look at [linking FFMPEG and Easystreamer](ADDME) |
| H.264 (libx264) stream is freezing/choppy | BFrames not disabled | Add `-bf 0` to your ffmpeg command after `-c:v libx264` |
| Quality is terrible | Too-Low bitrate | Specify a higher bitrate using the `-b:v` option. You might try `-b:v 1M` to increase the bitrate to 1 megabit/s | 

# Helpful Commands
`./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libvpx -cpu-used 5 -deadline 1 -g 3 -error-resilient 1 -auto-alt-ref 1 -f rtp rtp://127.0.0.1:5000?pkt_size=1200`


# Todo:
- [ ] Proper error handling
  - Gotta get rid of all the .expect()'s and figure out how to do things with results.
- [ ] Define the API
- [ ] Create a demo JS library
- [ ] Document document document!

# Flags
-c: Config file. Defines FFMPEG stream inputs accessible by clients.

# Event API
### Connection events
I: client offer. All client calls must include a client-specific ID. It's up to the caller to generate and track these unique IDs.

### Client Stream API
I: add client stream: Adds a stream to a client. Must specify stream ID
I: delete client stream: Deletes a stream from a client. Must specify stream ID

### Client stream replacement API
- Note: Idea is to be usable for things like VMS, where a live stream sometimes needs to be temporarily replaced with a historical stream
?? I: temp replace client stream: Replaces a stream in-place. Does NOT trigger a renegotiation. 
?? I: restore client stream: Restores the original stream of a replaced stream. Does NOT trigger a renegotiation.

### Connection events
O: client closed: Triggered when the webrtc connection is closed.
O: client disconnection: Triggered when the webrtc connection is disconnected. Note that a client might recover from this state, and become connected again.
O: client connection: Triggered when the webrtc connection is opened.

### Stream API
I: Add Stream: Adds a new RTP stream
I: Delete Stream: Deletes an RTP stream. May trigger RENEGOTIATION.


# Testing Commands
Low-framerate video with frequent GOPs.\
`./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libx264 -b:v 2M -g 3 -tune zerolatency -pkt_size 1200 -f rtp rtp://239.7.69.7:5002`

GOP burst at start of encoding stream. Makes it so the fast-forward buffer is immediately populated, allowing semi-instant connections to get instant video. Note: Requires EasyStreamer to be started BEFORE ffmpeg\
`./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libx264 -force_key_frames "expr:gte(3,n)" -b:v 2M -g 100 -tune zerolatency -bf 0 -pkt_size 1200 -f rtp rtp://239.7.69.7:5002`

High framerate version of the previous command\
`./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=30 -vcodec libx264 -force_key_frames "expr:gte(3,n)" -b:v 2M -g 100 -tune zerolatency -bf 0 -pkt_size 1200 -f rtp rtp://239.7.69.7:5002`

Find webcams \
`./ffmpeg -list_devices true -f dshow -i dummy` \
`./ffmpeg -f dshow -list_options true -i video="USB 2.0 Camera"` \

Stream from webcam \
`./ffmpeg -re -f dshow -i video="USB 2.0 Camera" -vcodec libx264 -force_key_frames "expr:gte(3,n)" -b:v 2M -g 1000 -preset ultrafast -tune zerolatency -bf 0 -pkt_size 1200 -f rtp rtp://239.7.69.7:5002`
## Pre-0.0.1-alpha1 checklist
- [ ] Client stream addition/removal
- [ ] Dynamic configuration (config.json)
  - [ ] Option for persistance
- [ ] Managed, Un-managed streams
- [ ] API for dynamic addition, removal of streams
- [ ] JSON api
- [ ] Documentation!

## Priority TODOs
- [x] Stream add, remove
  - [ ] FIXME: find a way to resume streams for connected clients without a constant time delay. It might not work for all circumstances.
- [ ] Config serialization
- [ ] Streams add option - persistent?: bool
  - If persistent, changes are written to passed config.
- [ ] Managed/Unmanaged streams (managed, internal - unmanaged, external)
  - Managed streams can be started/stopped as clients are added/dropped, saving on encoding power.
  - Managed streams can have their settings simplified
  - LOOK INTO if rawvideo format can take input over UDP / non-stdin
    - Use case: opencv python -> EasyStreamer ffmpeg
  - https://ffmpeg-user.ffmpeg.narkive.com/eeg4eddb/detecting-frames-on-raw-video-stream
- [ ] Config
  - option to allow config mutation by clients (--allow-config ?)
  - Maybe general option for mutation, then secondary for web clients specifically?
- [ ] Web server port option

## TODO ideas
- Datachannel to notify when buffering complete, so flash of fast-start/fast-forward isn't seen
- stdout/stdin API (JSON/CSV/etc)
- bframe detection and HOW TO DISABLE hint for h264
- Automatic codec detection
- ffmpeg command generation help/automation