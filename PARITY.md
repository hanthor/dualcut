# Feature parity

Where Dualcut stands against mainstream editors (#28). Columns split
the **GUI** (what a human can do in the app) from the **Backend** (what
the document/engine/agent surface supports) because they intentionally
lead-lag each other.

Legend: ✅ solid · 🟡 partial/basic · ❌ absent · — not applicable

## Media & library

| Feature | CapCut Desktop | iMovie | Kdenlive | Dualcut GUI | Dualcut Backend |
|---|---|---|---|---|---|
| Import media library | ✅ | ✅ | ✅ | ✅ | ✅ (`library`) |
| Drag-and-drop import | ✅ | ✅ | ✅ | ✅ | — |
| Thumbnails / waveforms | ✅ | ✅ | ✅ | ✅ | ✅ (cached) |
| Proxy media for smooth preview | ✅ | ✅ | ✅ | ✅ | ✅ (cached, preview-only) |
| Stock/cloud asset store | ✅ | 🟡 | ❌ | ❌ | ❌ |

## Timeline

| Feature | CapCut Desktop | iMovie | Kdenlive | Dualcut GUI | Dualcut Backend |
|---|---|---|---|---|---|
| Multi-track layers | ✅ | 🟡 (main + overlays) | ✅ | ✅ (scene lanes + overlays) | ✅ |
| Scene/section grouping | ❌ | ❌ | ❌ | ✅ (ruler) | ✅ |
| Drag to move / retime | ✅ | ✅ | ✅ | ✅ | ✅ |
| Edge trim | ✅ | ✅ | ✅ | ✅ (right edge) | ✅ |
| Split at playhead | ✅ | ✅ | ✅ | ✅ (S key) | ✅ |
| Snapping | ✅ | ✅ | ✅ | ✅ | ✅ |
| Zoom | ✅ | ✅ | ✅ | ✅ | — |
| Playhead indicator | ✅ | ✅ | ✅ | ✅ | — |
| Ripple/roll/slip edits | ✅ | 🟡 | ✅ | ✅ (ripple delete) | ✅ |
| Track mute/hide toggles | ✅ | ✅ | ✅ | ✅ (overlay tracks) | ✅ |

## Editing & compositing

| Feature | CapCut Desktop | iMovie | Kdenlive | Dualcut GUI | Dualcut Backend |
|---|---|---|---|---|---|
| Transform (position/scale) | ✅ | 🟡 | ✅ | ✅ (preview handles) | ✅ |
| Opacity | ✅ | ✅ | ✅ | ✅ | ✅ |
| Keyframe animation | ✅ | ❌ | ✅ | ✅ | ✅ (tween + keyframes) |
| Animation presets (in/out) | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Scene transitions (wipes etc.) | ✅ | ✅ | ✅ | ✅ (6 kinds) | ✅ |
| Effects (blur, color) | ✅ | 🟡 | ✅ | ✅ | ✅ |
| Full color grading | ✅ | 🟡 | ✅ | ❌ | 🟡 (videobalance) |
| Masks / chroma key | ✅ | ✅ | ✅ | ✅ (chroma key + crop) | ✅ |
| Speed ramping | ✅ | ✅ | ✅ | 🟡 (constant rate; ramping later) | ✅ |
| Vector shapes | 🟡 (stickers) | ❌ | 🟡 | ✅ (7 shapes, live GPU) | ✅ (`vello://`) |

## Text & templates

| Feature | CapCut Desktop | iMovie | Kdenlive | Dualcut GUI | Dualcut Backend |
|---|---|---|---|---|---|
| Text clips | ✅ | ✅ | ✅ | ✅ | ✅ |
| Rich text styling | ✅ | 🟡 | ✅ | ✅ (align/outline/shadow) | ✅ |
| Title templates | ✅ | ✅ | ✅ | ✅ (defs + thumbnails) | ✅ |
| Parameterised/nested templates | 🟡 | ❌ | ❌ | ✅ | ✅ (defs nest) |
| Save selection as template | 🟡 | ❌ | 🟡 | ✅ | ✅ |
| Auto-captions (STT) | ✅ | ❌ | 🟡 | 🟡 (needs local whisper.cpp + model) | 🟡 (recipe via agent surface) |

## Audio

| Feature | CapCut Desktop | iMovie | Kdenlive | Dualcut GUI | Dualcut Backend |
|---|---|---|---|---|---|
| Audio tracks | ✅ | ✅ | ✅ | ✅ (overlays) | ✅ |
| Waveform display | ✅ | ✅ | ✅ | ✅ | — |
| Volume keyframes/fades | ✅ | ✅ | ✅ | ✅ (presets + keyframes) | ✅ |
| Detach audio from video | ✅ | ✅ | ✅ | ✅ | ✅ |
| Auto-crossfade at cuts | ✅ | ✅ | 🟡 | ✅ | ✅ |
| Audio effects (EQ, denoise) | ✅ | 🟡 | ✅ | ✅ (EQ + compressor) | ✅ |

## Export

| Feature | CapCut Desktop | iMovie | Kdenlive | Dualcut GUI | Dualcut Backend |
|---|---|---|---|---|---|
| H.264 / H.265 | ✅ | ✅ | ✅ | ✅ | ✅ |
| VP8/VP9/AV1 | 🟡 | ❌ | ✅ | ✅ | ✅ |
| ProRes / lossless (FFV1) | ❌ | ✅ (ProRes) | ✅ | ✅ | ✅ |
| Audio-only export | ✅ | ✅ | ✅ | ✅ (5 formats) | ✅ |
| Overwrite guard, dir picker | ✅ | ✅ | ✅ | ✅ | — |
| Render progress | ✅ | ✅ | ✅ | ✅ (live bar) | ✅ (callback) |
| Background render queue | ✅ | ❌ | ✅ | ✅ (sequential queue) | 🟡 (HTTP `/render`) |

## Automation (Dualcut's home turf)

| Feature | CapCut Desktop | iMovie | Kdenlive | Dualcut GUI | Dualcut Backend |
|---|---|---|---|---|---|
| Human-readable project format | ❌ | ❌ | 🟡 (XML) | ✅ (Code tab) | ✅ (JSON + schema) |
| Hot-reload on external edit | ❌ | ❌ | ❌ | ✅ | ✅ |
| Scripting | ❌ | ❌ | 🟡 (Python, limited) | ✅ (TypeScript) | ✅ |
| HTTP API | ❌ | ❌ | ❌ | — | ✅ (port 7357) |
| Agent skill / docs for AI edits | ❌ | ❌ | ❌ | ✅ (installer) | ✅ |
| Headless render CLI | ❌ | ❌ | ✅ (melt) | — | ✅ |

## Remaining gaps

1. **Speed ramping** — constant rate shipped; keyframed curves are a
   follow-up.
2. **Auto-captions bundling** — *Generate Captions…* in the menu and
   the agent recipe both drive a local whisper.cpp install (#37); the
   remaining gap is bundling STT so it works out of the box.
3. **Bezier masks** — chroma key + rectangular crop ship; freeform
   masks need a compositor story.
4. **Denoise** — EQ + compressor ship; needs a flatpak-viable plugin.
5. **Full color grading** — basic balance only; curves/wheels are a
   bigger project.
