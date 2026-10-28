# -*- coding: utf-8 -*-
"""Generates the narration clips with the ElevenLabs voice. The API key is read
from ~/arkiv/.env and never printed."""
import json, os, pathlib, re, sys, urllib.request

VOICE = "J60wsh32nnaUrr8BIYwZ"
env = pathlib.Path.home().joinpath("arkiv/.env").read_text()
key = re.search(r"^ELEVENLABS_API_KEY=(.+)$", env, re.M).group(1).strip()
lines = json.load(open("narration.json"))
only = set(sys.argv[1:])
for name, text in lines.items():
    out = pathlib.Path(f"vo/{name}.mp3")
    if only and name not in only:
        continue
    req = urllib.request.Request(
        f"https://api.elevenlabs.io/v1/text-to-speech/{VOICE}",
        data=json.dumps({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {"stability": 0.55, "similarity_boost": 0.75},
        }).encode(),
        headers={"xi-api-key": key, "content-type": "application/json", "accept": "audio/mpeg"},
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        out.write_bytes(r.read())
    print(name, out.stat().st_size, "bytes")
