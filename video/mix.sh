#!/usr/bin/env bash
# Mixes the narration over the rendered film: each line 0.4 s after its scene's
# cut, music ducked under the voice, loudness at -14 LUFS.
# Usage: video/mix.sh <rendered.mp4> <out.mp4>
set -euo pipefail
cd "$(dirname "$0")"
IN="$1"; OUT="$2"
# scene start times in seconds, from `npm run timeline -- --film blindfetch`
declare_starts="hook:0 leak:4 reveal:18 index:24 fetch:42 wallet:54 browser:68 ticks:82 outro:98"
inputs=(); filters=""; labels=""; i=1
for pair in $declare_starts; do
  name=${pair%%:*}; start=${pair##*:}
  ms=$(python3 -c "print(int(($start + 0.4) * 1000))")
  inputs+=(-i "vo2/$name.mp3")
  filters+="[$i:a]aresample=48000,adelay=${ms}|${ms},apad[v$i];"
  labels+="[v$i]"
  i=$((i + 1))
done
n=$((i - 1))
filters+="${labels}amix=inputs=$n:normalize=0:duration=longest,volume=1.0[voice];"
filters+="[voice]asplit=2[vo][sc];"
filters+="[0:a]aresample=48000,volume=0.55[music];"
filters+="[music][sc]sidechaincompress=threshold=0.03:ratio=8:attack=20:release=350[ducked];"
filters+="[ducked][vo]amix=inputs=2:normalize=0:duration=first,loudnorm=I=-14:TP=-1:LRA=11[a]"
ffmpeg -y -v error -i "$IN" "${inputs[@]}" -filter_complex "$filters" -map 0:v -map "[a]" -c:v copy -c:a aac -b:a 192k -ar 48000 -movflags +faststart "$OUT"
echo "mixed: $OUT $(ffprobe -v error -show_entries format=duration -of csv=p=0 "$OUT")s"
