Download ffmpeg.exe and ffprobe.exe and put in root project dir

## Note: Not sure about the final name yet. Any ideas? :)

# Quickstart

# Why?
... same as old archive

# Interacting with CHANGEME
You have two options for interacting with CHANGEME. For quick projects that don't need more than basic stream viewing, using the internal webserver is the fastest way to get started. For more complicated projects, or for embedding into an existing web UI, a JSON api is provided through STDIN and STDOUT.

# Configuration
... JSON config file.

# random commands
`./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libvpx -cpu-used 5 -deadline 1 -g 3 -error-resilient 1 -auto-alt-ref 1 -f rtp rtp://127.0.0.1:5000?pkt_size=1200`
