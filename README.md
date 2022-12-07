Download ffmpeg.exe and ffprobe.exe and put in root project dir

## Note: Not sure about the final name yet. Any ideas? :)

# Quickstart

# Why?
... same as old archive

# Interacting with CHANGEME
You have two options for interacting with CHANGEME. For quick projects that don't need more than basic stream viewing, using the internal webserver is the fastest way to get started. For more complicated projects, or for embedding into an existing web UI, a JSON api is provided through STDIN and STDOUT.

# Configuration
... JSON config file.

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