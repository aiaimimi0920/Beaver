# F8 frozen attempt media preview

Date: 2026-09-25

## Delivered workflow

Production execution records now preview images and audio selected from an
attempt's frozen input or output checkpoint. The existing checkpoint list and
Desktop `objectTask.attemptFile` route remain the entry points. PNG, JPEG, GIF
and WebP use an image element; WAV, MP3, Ogg and FLAC use audio controls.
The attempt limit remains 1 MiB; the object-version limit remains 8 MiB.

Core checks project, run, attempt, checkpoint membership and hash before
returning bounded media bytes. It never falls back to the workspace. Regression
coverage reads both persisted checkpoint selections after live modification and
deletion, and rejects tampered blobs. Oversized files remain explicitly
unverified. Unsupported binary content has an explicit placeholder; SVG and
HTML remain escaped text rather than executable media.

Versions and attempts share a passive-format classifier, bounded MIME/base64
schemas and the media component. Full request identity is checked on responses;
selection and close invalidate late requests. The media component is keyed to
the selected request, revokes its Blob URL on cleanup and shows decoding errors.
The Desktop tool description now documents supported media formats.

## Verification

Commands ran from the repository root in the normal PowerShell environment.

- `cargo test --locked -p beaver-core --lib object_attempt_file`: 3 passed.
- `cargo test --locked -p beaver-core object_version_file`: 4 passed.
- `cargo test --locked -p beaver-desktop object_attempt_runtime::tests`: 3 passed.
- `npx tsx --test tests/object-attempt-file.test.ts tests/object-version-file.test.ts`: 8 passed.
- `npm run typecheck`: passed.
- Rustfmt on changed Rust files and Prettier on changed TS/TSX files: passed.
- `npm run check:effective-lines`: 1097 sources, 0 violations; baseline unchanged.
- `git -c core.safecrlf=false diff --check`: passed.

Logs: `output/f8-attempt-media-{core,version,desktop,ui,types,format,lines}.log`.
The initial compile found a test-only base64 import inherited through the old
version module; it is now explicit. Initial filters using test file names matched
zero tests; the commands and passing counts above use actual module names.

## Remaining scope

This closes the attempt checkpoint media presentation increment of F8.1.
F8.1-F8.5 remain unchecked. Native WebView decoding and playback were not
exercised: Core tests verify transport bytes and integrity, and UI tests verify
schemas, request lifecycle and server-rendered integration. Media header fixtures
do not prove decoding. No native executable was launched or delivered.

Godot/Blender object-scoped rendering, provenance-backed thumbnails, camera
interaction, frozen selection and feedback-to-rework integration remain open.
No release, full acceptance, version increment, commit or push was performed.
