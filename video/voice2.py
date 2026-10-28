# -*- coding: utf-8 -*-
"""Narration, one clip per line, in the cloned voice with the playbook's
settings. The key is read from ~/arkiv/.env and never printed."""
import json, pathlib, re, sys, urllib.request

VOICE = "J60wsh32nnaUrr8BIYwZ"
key = re.search(r"^ELEVENLABS_API_KEY=(.+)$", pathlib.Path.home().joinpath("arkiv/.env").read_text(), re.M).group(1).strip()
lines = json.load(open("narration2.json"))
only = set(sys.argv[1:])
for name, text in lines.items():
    if only and name not in only:
        continue
    req = urllib.request.Request(
        f"https://api.elevenlabs.io/v1/text-to-speech/{VOICE}",
        data=json.dumps({"text": text, "model_id": "eleven_v4",
                         "voice_settings": {"stability": 1.0, "similarity_boost": 0.75},
                         "language_code": "en"}).encode(),
        headers={"xi-api-key": key, "content-type": "application/json", "accept": "audio/mpeg"},
    )
    with urllib.request.urlopen(req, timeout=180) as r:
        pathlib.Path(f"vo2/{name}.mp3").write_bytes(r.read())
    print(name)
