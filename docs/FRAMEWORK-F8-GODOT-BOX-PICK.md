# F8 Godot frozen-frame box selection

2026-09-26. This slice completes the Godot static-mesh hit-set portion of F8.4.
The overall F8/F9 plan remains open; no release or native acceptance is claimed.

## Usable workflow

Freeze a Godot preview, enable image regions, and drag a rectangle with point
selection disabled. Beaver queries the frozen mesh snapshot and displays matching
node paths beside the numbered region. Comments, context thumbnails,
delete/renumber, saved-frame replay and existing feedback-task delivery preserve
the receipt and original frame identity.

The engine clips triangles against the rectangle camera frustum, including
near/far planes. Through-selection includes occluded supported meshes; it does
not claim visible-pixel selection. Existing unsupported-geometry exclusions and
the 100,000-triangle snapshot budget remain. Results contain at most 32 unique
paths, each at most 4,096 UTF-8 bytes and 32,768 bytes in aggregate; omitted
matching paths set truncated. The UI reports skipped nodes and truncation.
Historical topology still requires relocation against current content.

The Desktop pick API accepts an optional normalized rectangle. Core validates
bounds, exact region geometry, receipt capability, frozen frame identity, path
limits and trusted engine receipt. Idempotent retries handle engine numeric JSON
round trips. Legacy ray receipts and image-only regions remain supported.
No database migration, version change, commit or push was performed.

## Verification

- npm run typecheck: passed.
- Frontend preview-picking, preview-selection and publication-frame-picker tests: 8 passed.
- Core pick_receipts: 1 passed.
- Core publication_frame: 3 passed, including project reopen and worker/RPC delivery of Box/Occluded paths; forged paths are rejected.
- cargo check --locked -p beaver-desktop: passed.
- Real graphical live_preview_real_camera_close_and_lease (ignored test explicitly enabled): 1 passed in 105.84 seconds. Center rectangle returns Box and Occluded; full-frame selection truncates at 32; outside, behind-camera and beyond-far nodes are excluded; empty rectangle has no hits. Conflicting retries, forged paths and changed region geometry are rejected. Existing camera, resolution, archive, reopen and lease checks also passed.
- Separate runtime projection probe: near depth -1, far depth +1. The review suggestion to change to 0-to-1 clipping was rejected on runtime evidence. Installed Godot reports 4.5.2.rc.custom_build.2890667c8 despite its 4-4-1 directory name.
- Chromium production PreviewFrameRegions with simulated API and recorded real hit sets: scaled reverse drag, two-node set, 32-node truncation, comments, delete/renumber, exact local-storage reload and read-only replay passed. This fixture does not prove a live browser-to-engine connection. Initial harness failures were a stopped temporary server, missing scroll adjustment and non-unique SVG selectors; final proof uses a live isolated server and named region SVG.
- Relevant Prettier, Rustfmt and gdformat checks passed. npm run check:effective-lines: 1,171 source files, no violations; baseline unchanged. Box clipping has 62 effective lines, Core pick 213 and region editor 308. git diff --check passed.

Cargo reported three existing unused-mut warnings in object_metadata_tests.rs.
No full application suite, new Beaver.exe build, external model call or formal
native acceptance was run in this development slice.

## Evidence and remaining work

Logs are under output/box-*.log, including typecheck, frontend, core-pick,
core-archive, desktop, engine, projection, structure, prettier, rustfmt,
gdformat, diff-check and browser-proof. Real PNGs/receipts are under
output/box-engine-proof/. The test-only simulated browser fixture and screenshot
are under output/playwright/box-pick/.

Blender interactive camera/selection, stronger topology relocation and remaining
F8/F9 integration/native-delivery conditions are outstanding. F8.1-F8.5 and F9
checkboxes remain unchecked. The whole Beaver development goal remains active.
