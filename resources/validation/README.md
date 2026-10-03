# Validation runtime dependencies

GUT 9.4.0 is pinned from https://github.com/bitwes/Gut/tree/v9.4.0
(tag archive root `bitwes-Gut-ca6cf20`). `gut-9.4.0.zip` contains the unchanged
`addons` directory, including `addons/gut/LICENSE.md` (MIT) and font licenses.
The archive SHA-256 is
`c54428c250f55a2945282ce462f1aecffac0133a7af2c459ac2ed286ddd0b83e`.
Beaver verifies this hash before extracting into a disposable validation copy.
It never installs GUT in the user's personal Codex environment.

After verification and extraction, Beaver overlays only `addons/gut/gut_loader.gd`
with the tracked `gut_loader.gd` in this directory. Godot 4.8 migrates and removes
`debug/gdscript/warnings/exclude_addons`; the overlay selects the existing
`directory_rules` dictionary or the legacy boolean, suppresses addon warnings
while loading GUT, and restores the complete original value before tests run.
GUT's warnings-manager snapshot also receives the original value. Missing or
malformed settings fail explicitly. No project settings, fixed archive bytes,
test requirements or engine-error gates are changed. All other extracted GUT
files remain byte-identical to the pinned archive.

The focused Rust sandbox tests cover isolation, upstream preservation and
zero-exit engine errors. The opt-in real-engine regression also exercises both
legacy boolean values through a settings double, dictionary restoration and
the actual loader/warnings-manager lifecycle. Run it with `BEAVER_TEST_GODOT`
set to the configured editor:

```powershell
cargo test --locked -p beaver-core validation::sandbox_tests -- --include-ignored --nocapture
```

The adapter targets Godot 4.4 or later. A real Godot renderer is required for
visual flows. Video records the complete run with Godot Movie Maker, then uses
FFmpeg (resolved from project validation settings or PATH) to encode WebM.
Missing tools are reported as run failures, never as passing evidence.

Runner `beaver-validation-2` preserves the game's logical canvas, stretch mode
and aspect policy when setting the capture window size. Control clicks use the
canvas-to-window transform. PNGs contain the rendered viewport fitted into its
actual window rectangle, including black bars; they do not crop the game canvas.
Movie Maker output is decoded and checked against the captured viewport aspect
before FFmpeg fits it into the same rectangle. A mismatched movie aspect fails
explicitly because missing pixels cannot be recovered by padding. Presentation
changes during a flow also fail rather than producing inconsistent evidence.

The opt-in viewport regression covers `canvas_items` and `viewport` at 960x540,
960x720 and 1280x720, button input, corner markers, black bars, source preservation
and decoded WebM frames. Set `BEAVER_TEST_GODOT` and `BEAVER_TEST_FFMPEG`, then run:

```powershell
cargo test --locked -p beaver-core visual_preserves_canvas_layout_and_clicks_at_capture_resolutions -- --ignored --nocapture
```

All scripts here are Beaver source. The compressed, immutable upstream archive
is a distributed dependency. Projects may retain its unchanged `addons/gut`
contents, including the MIT and font licenses, as a development dependency so
test scripts resolve when opened in the editor. The code-structure checker derives
its trusted paths and SHA-256 identities only from this compiled-in archive after
verifying the fixed archive hash above. New, moved or modified project files are
not exempt, and project-owned provenance cannot change this trust list. The
compatibility overlay remains sandbox-only and receives no new exemption.

Focused dependency and gate regressions:

```powershell
cargo test --locked -p beaver-core --lib code_structure
cargo test --locked -p beaver-core --test code_structure_contract
```
