Download ffmpeg.exe and ffprobe.exe and put in root project dir

# random commands
ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=30 -vcodec libvpx -cpu-used 5 -deadline 1 -g 10 -error-resilient 1 -auto-alt-ref 1 -f rtp rtp://127.0.0.1:5004?pkt_size=1200
ffprobe -show_frames -f rtp rtp://127.0.0.1:3333?pkt_size=1200