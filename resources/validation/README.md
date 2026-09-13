# Validation runtime dependencies

GUT 9.4.0 is pinned from https://github.com/bitwes/Gut/tree/v9.4.0
(tag archive root `bitwes-Gut-ca6cf20`). `gut-9.4.0.zip` contains the unchanged
`addons` directory, including `addons/gut/LICENSE.md` (MIT) and font licenses.
The archive SHA-256 is
`c54428c250f55a2945282ce462f1aecffac0133a7af2c459ac2ed286ddd0b83e`.
Beaver verifies this hash before extracting into a disposable validation copy.
It never installs GUT in the user's personal Codex environment.

The adapter targets Godot 4.4 or later. A real Godot renderer is required for
visual flows. Video records the complete run with Godot Movie Maker, then uses
FFmpeg (resolved from project validation settings or PATH) to encode WebM.
Missing tools are reported as run failures, never as passing evidence.

All scripts here are Beaver source. The compressed, immutable upstream archive
is a distributed dependency, not handwritten source or an exemption for modified
GUT files in game projects.
