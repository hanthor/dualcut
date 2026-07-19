# ADR 0003: Distribute via GitHub Releases only (no Flathub)

Date: 2026-07-18 · Status: accepted

## Context
The deliverable is a Flatpak. Flathub's AI policy (as of July 2026) rules
out submission for this project.

## Decision
Every `v*` tag builds `dualcut.flatpak` in CI and attaches it to a GitHub
Release; the README install one-liner tracks `releases/latest`.
`scripts/release.sh` is the only sanctioned way to cut a release (bumps
Cargo + appstream metainfo, tags, pushes).

## Consequences
- No Flathub review pipeline; sandbox holes are our own judgement
  (network for the agent API, home for project files).
- Revisit if Flathub policy changes.
