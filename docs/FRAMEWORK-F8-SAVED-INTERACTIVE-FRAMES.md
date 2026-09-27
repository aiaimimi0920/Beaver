# F8: Saved interactive Godot frames

Date: 2026-09-26. Status: bounded capture and replay workflow complete; F8 and F9 remain open.

## Usable outcome

The production object scene preview can capture and save a real Godot frame, close the live session, and display the saved PNG without opening Godot. Saved frames include the source target, preview run and snapshot identity, image hash, dimensions, frame sequence, view revision, and camera transform/projection. The current completed preview run exposes its saved frames in the object preview.

Core persists immutable records in the project SQLite store under `objectPreviewFrames`. Each run allows at most 8 frames and a 16 MiB serialized archive. `objectScenePreviewSource` retains the source per run, so replacing the latest preview source does not change an existing run binding. Legacy runs fall back to their matching existing source record.

`validation.preview.capture` saves the latest acknowledged Core frame for the requested ready view revision. Stable request receipts make a lost-response retry return the original record, including after the session closes. `validation.preview.saved` reads without resolving the engine and verifies source identity, record digest, PNG hash and decoded dimensions. Capture does not change validation evidence, approval, or the original validation run.

## Focused verification

- Real Godot Core test `live_preview_real_camera_close_and_lease`: 1 passed, 0 failed, 149.20 seconds. Covered real capture, idempotent replay, project isolation, retry after close, unchanged validation run, SQLite reopen with engine resolution forbidden, and rejection of a modified saved camera. Existing camera/resolution/close/lease assertions remained in the same test. Log: `output/saved-frame-real.log`. Real PNG and camera proofs: `output/f8-saved-frame-real/`.
- Frontend live-preview and adjacent scene-preview tests: 10 passed, 0 failed. Includes lost-response capture retry, pending-view rejection and mismatched source identity rejection. Log: `output/saved-frame-ts.log`.
- `cargo check --locked -p beaver-desktop` passed. `npm run typecheck` passed. Targeted Rust formatting and Prettier checks passed. Logs: `output/saved-frame-desktop.log`, `output/saved-frame-types.log`, `output/saved-frame-format-check.log`.
- `npm run check:effective-lines` passed: 1140 sources, 17 unchanged legacy files, 0 violations. No baseline change. Log: `output/saved-frame-lines.log`. Targeted `git diff --check` passed.
- Chromium production-component fixture completed capture, image decoding, close, reload and saved-image replay at 960 x 540, with no preview-open call on reload and no UI alert. Screenshot inspected: [saved frame replay](../output/playwright/f8-saved-frame/replay.png). Script: `output/playwright/f8-saved-frame/smoke.js`; execution log: `output/saved-frame-browser.log`. The fixture uses a real engine PNG with simulated APIs and browser-local fixture storage. Durable SQLite replay is established separately by the Core test; this browser check is not native end-to-end acceptance.

The owned browser session and fixture HTTP server were closed after verification.

## Remaining boundaries

Saving a frame does not pause the running engine or guarantee the saved sequence is the last image presented on screen. No live-frame point selection, normalized regions, real raycast hits, or feedback submission is implemented by this slice. A later selection workflow must freeze the engine scene consistently with the original frame, or retain matching per-frame geometry; raycasting a later scene against an old PNG is invalid.

The UI lists frames for the current preview run. Older run records remain stored, but history navigation and deletion are not added. Blender preview, full F8 selection/feedback integration, F9 production audit and native delivery remain outstanding. F8.3 and the overall framework must remain unchecked.
