# Recipe: auto-captions with local STT (#37)

Transcribe a clip's audio with any local speech-to-text tool and land
the segments as styled text clips on a `subtitles` overlay track. No ML
ships in dualcut — the agent surface is the integration point.

## 1. Extract the audio

```sh
# from the project's media file (or export just the audio):
dualcut-render project.json /tmp/voice.wav wav
```

## 2. Transcribe with timestamps

Any tool emitting timed segments works; whisper.cpp example:

```sh
whisper-cli -m ggml-base.en.bin -f /tmp/voice.wav --output-json /tmp/voice.json
```

## 3. Convert segments into subtitle clips

```python
import json, urllib.request

segs = json.load(open("/tmp/voice.json"))["transcription"]
project = json.load(urllib.request.urlopen("http://localhost:7357/project"))

track = next((t for t in project["overlays"] if t["id"] == "subtitles"), None)
if track is None:
    track = {"id": "subtitles", "name": "Subtitles", "clips": []}
    project["overlays"].append(track)

for i, seg in enumerate(segs):
    t0 = seg["offsets"]["from"] / 1000
    t1 = seg["offsets"]["to"] / 1000
    track["clips"].append({
        "id": f"sub-{i}",
        "start": round(t0, 2),
        "duration": round(t1 - t0, 2),
        "type": "text",
        "text": seg["text"].strip(),
        "font": "Sans Semi-Bold 22",
        "color": "#ffffff",
        "outline": "#000000",
        "align": "center",
    })

req = urllib.request.Request(
    "http://localhost:7357/project",
    data=json.dumps(project).encode(),
    headers={"Content-Type": "application/json"},
    method="POST",
)
urllib.request.urlopen(req)
```

The running editor hot-reloads with the subtitles in place — review
them on the timeline, retime by dragging, restyle via a caption def.

## Notes

- Outline + center alignment (v0.20.0 text styling) keeps captions
  readable over any footage.
- For burned-in styling shared across captions, make a `caption` def
  and emit `compref` clips instead of raw text clips.
- Editing the project file on disk works identically if the HTTP API
  is off; the app watches the file.
