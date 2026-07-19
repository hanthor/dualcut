# ADR 0004: vello:// URI handler bridges live vector rendering into GES

Date: 2026-07-18 · Status: accepted

## Context
Live per-frame vector rendering needs a GStreamer source GES can use, but
the GES Rust bindings only expose Formatter subclassing — no
GESSource subclasses.

## Decision
`vellosrc` is a `GstPushSrc` subclass registered with a `vello://` URI
handler. uridecodebin — and therefore `GESUriClip` — instantiates it from
URIs like `vello://star?fill=%23ffd700&w=220&h=220&spin=1`. Static
`shape` clips keep the cached-PNG raster path (zero per-frame cost).

## Consequences
- Any document `video` clip can point at a live vector source today.
- Shape parameters live in the URI; animating them per-frame means
  extending vellosrc properties, not the document mapping.
- One shared wgpu device + Vello renderer (see `vector.rs`) serves both
  the PNG path and vellosrc; renderer reuse is what makes per-frame
  rendering affordable.
