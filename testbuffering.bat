start "" ./ffmpeg -re -f lavfi -i testsrc=size=640x480:rate=1 -vcodec libx264 -b:v 200k -cpu-used 5 -g 3 -f rtp -sdp_file ./extstream rtp://239.7.69.7:5002
timeout /t 1
start "" ./ffmpeg -protocol_whitelist rtp,udp,file -i extstream -vcodec copy -f rtp -sdp_file intstream rtp://127.0.0.1:5001?pkt_size=1316

timeout /t 1
start "" ./ffplay extstream -fflags nobuffer -flags low_delay -framedrop -protocol_whitelist rtp,udp,file
pause
start "" ./ffplay intstream -fflags nobuffer -flags low_delay -framedrop -protocol_whitelist rtp,udp,file


