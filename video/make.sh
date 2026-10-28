#!/usr/bin/env bash
# Assembles the demo video from frames/*.png and vo/*.mp3.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p out/clips
dur() { ffprobe -v error -show_entries format=duration -of csv=p=0 "$1"; }

clip() { # name image seconds [audio]
  local name=$1 img=$2 secs=$3 audio=${4:-}
  local frames; frames=$(python3 -c "print(int(round($secs*30)))")
  local zoom="scale=2016:1134,zoompan=z='min(zoom+0.00018,1.035)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=$frames:s=1920x1080:fps=30,format=yuv420p"
  if [ -n "$audio" ]; then
    ffmpeg -y -v error -loop 1 -i "$img" -i "$audio" -filter_complex "[0:v]$zoom[v];[1:a]apad,atrim=0:$secs,afade=t=in:d=0.05[a]" \
      -map "[v]" -map "[a]" -t "$secs" -c:v libx264 -preset medium -crf 18 -c:a aac -b:a 192k -ar 48000 "out/clips/$name.mp4"
  else
    ffmpeg -y -v error -loop 1 -i "$img" -f lavfi -i anullsrc=r=48000:cl=stereo -filter_complex "[0:v]$zoom[v]" \
      -map "[v]" -map 1:a -t "$secs" -c:v libx264 -preset medium -crf 18 -c:a aac -b:a 192k -ar 48000 "out/clips/$name.mp4"
  fi
  echo "clip $name ${secs}s"
}

clip title frames/title.png 3
for s in s1 s2 s3 s4 s5; do
  img=frames/$s.png; [ -f "$img" ] || img=frames/s2.png
  clip "$s" "$img" "$(python3 -c "print($(dur vo/$s.mp3)+0.7)")" "vo/$s.mp3"
done
clip s6 frames/s6.png "$(python3 -c "print($(dur vo/s6.mp3)+2.0)")" vo/s6.mp3

printf "file 'clips/%s.mp4'\n" title s1 s2 s3 s4 s5 s6 > out/list.txt
ffmpeg -y -v error -f concat -safe 0 -i out/list.txt -c copy out/blindfetch-demo.mp4
echo "done: out/blindfetch-demo.mp4 $(dur out/blindfetch-demo.mp4)s"
