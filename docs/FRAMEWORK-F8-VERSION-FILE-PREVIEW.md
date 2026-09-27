# F8: frozen object-version file preview

## Delivered workflow

The production object-details panel now lists files from the selected frozen
version manifest. Selecting a member reads that exact version and displays text,
an image or audio controls. Changing the project, object, version or selected
file clears the old result; late responses cannot replace the current selection.
Closing the preview cancels its presentation. Blob URLs are revoked on cleanup.

Old versions remain readable after a newer capture, deletion of live files,
registration metadata edits and reopening the project runtime. No task is needed.
The existing catalog thumbnail remains a live asset reference; this delivery
does not establish frozen thumbnail provenance.

## Source and API contract

The read-only Desktop command `object.versionFile` requires `projectId`,
`objectId`, `versionId`, `path` and `sha256`. It routes to the requested project
runtime, checks the catalog version against the persisted version tuple, validates
the frozen manifest and requires an exact manifest member and hash. There is no
host-store or live-workspace fallback.

Core checks blob metadata, rejects linked/non-file blobs and checks byte length.
For content at or below 8 MiB, bounded reading and SHA-256 verification precede
delivery. Larger content returns `tooLarge` after metadata/length checks; its
content digest is not verified. Unsupported binary content returns `unsupported`.
Both limits are explained in the production panel.

PNG, JPEG, GIF and WebP use image presentation; WAV, MP3, Ogg and FLAC use audio
controls without autoplay. Other valid text is escaped, including SVG and HTML.
Media decoding errors are visible. The client validates the response schema,
media MIME allowlist, bounded base64 and every echoed request identity field.

`read_frozen_version` and `readFrozenObjectVersion` validate historical frozen
metadata independently of current registration metadata. Existing `read_version`
and `readObjectVersion` retain their stricter metadata equality checks, preserving
the existing import and acceptance consumers.

## Focused verification (2026-09-25)

- `npm run typecheck`: completed without diagnostics.
- `npx tsx --test tests/object-version-file.test.ts tests/object-catalog.test.ts tests/object-import-contract.test.ts`: 14 passed, 0 failed.
- `cargo test --locked -p beaver-core --lib object_version -- --nocapture`: 14 passed, 0 failed.
- `cargo test --locked -p beaver-desktop object_catalog_dispatch -- --nocapture`: 3 passed, 0 failed.
- `cargo fmt --all` and Prettier on the changed TypeScript/TSX files completed.
- `npm run check:effective-lines`: 1094 sources, 17 unchanged legacy files, 0 violations; baseline unchanged.
- `git diff --check`: passed.

Logs are in `output/f8-preview-{format,types,ui,core,desktop,lines}-final.log`.
The resumed tool process handle was unavailable, so final results were recovered
from the completed logs and the structural checker JSON (`ok: true`).

Regression coverage includes persisted/catalog version divergence, foreign
identities, stale hashes, missing manifest members, size/hash corruption, old
content after live deletion and metadata edits, response races, escaped active
text, media transport and explicit limits. Desktop tests exercise business-tool
validation and actual dispatch. Production component rendering verifies that
buttons come from frozen members rather than current registration files.

## Remaining acceptance and plan status

Media test bytes prove transport and classification, not real codec decoding.
No native WebView interaction, real media decoding, Godot/Blender rendering,
engine picking, camera/frame session, thumbnail provenance or feedback routing
acceptance was performed. No release build, publication or version bump occurred.

This completes the frozen-version file-viewing subflow only. F8.1 remains open
for engine-backed objects, attempts and subcontent; F8.2-F8.5 remain open for
sessions, selection provenance, annotations and actual feedback integration.
The overall Beaver development goal remains active.
