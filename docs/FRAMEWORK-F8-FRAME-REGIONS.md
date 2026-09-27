# F8: Frozen image regions and archive replay

Date: 2026-09-26. The production Godot viewer now supports numbered image regions on an acknowledged frozen frame, optional per-region opinions, an overall opinion, immutable saving and engine-free replay. F8 and F9 remain open.

## Usable workflow

Freeze the preview, enable image selection, and drag up to eight rectangles. The switch defaults off; switching it off preserves existing regions but prevents new ones. Reverse drags and displayed-image scaling map to normalized original-image coordinates. Each region is numbered and may carry an opinion. Saving archives the selection alongside the exact original PNG, camera, viewport dimensions, frozen source and snapshot identity. The saved viewer reproduces numbered regions and opinions without starting Godot.

Resume clears the current unsaved regions as stated in the UI; it does not modify saved archives. This slice does not persist unsaved region drafts. Closing the live viewer before saving also discards those edits.

## Integrity and recovery

The optional selection on validation.preview.capture requires an acknowledged frozen frame with exactly matching sequence and PNG SHA-256. Core rejects out-of-image rectangles, excess regions, oversized UTF-8 opinions, unknown fields and fabricated hit capability. Selections declare normalized-image coordinates and hitCapability: unavailable. They are two-dimensional image regions, without engine hit sets.

The immutable archive digest includes the selection. Reads validate both the PNG and selection before verifying the record digest. Existing archives without selections remain readable. Request receipts and the archive update share the existing transaction; retrying the same request can replay after the live session is gone. The controller retains the original cloned payload after uncertain transport failure and blocks view changes until a retry resolves it. Definitive selection, source, frame and storage-limit failures unlock editing/resume. In-memory pending requests are not durable across viewer unmounts; saved archives can be re-read.

No object checkpoint image, task feedback, validation evidence or approval is manufactured or rewritten. Three-dimensional hit testing, Blender integration, task submission and cross-version relocation remain outstanding.

## Verification

- Core regression preview_selection_archive_survives_reopen_and_rejects_wrong_frames: 1 passed, 0 failed (0.12 seconds). It exercises real PNG validation, SQLite reopen, receipt replay without a live frame, unchanged source/run data, invalid frame identities/regions/capabilities, UTF-8 limits, no writes after rejection and archive-tamper detection. Log: output/regions-core.log.
- Frontend regressions: 9 passed, 0 failed across live-scene-preview.test.ts and preview-selection.test.ts. Coverage includes uncertain-save payload identity, definitive rejection recovery and frozen frame binding. Log: output/regions-ts-tests.log.
- TypeScript typecheck and Desktop Cargo check passed. Desktop emitted three existing warnings. Targeted Prettier and rustfmt checks passed; Git whitespace check passed.
- Effective-line check: 1145 sources, 17 unchanged legacy files, 0 violations. Baseline unchanged. Log: output/regions-structure.log.
- Chromium production-component fixture passed: initially disabled selection, scaled reverse drag, per-region/overall opinions, switch-off preservation, exact-frame capture, resume/new-freeze clearing, close and full-page reload into read-only saved replay. Replay made one saved-read API call and no live-session calls. [Editing screenshot](../output/playwright/f8-regions/edit.png) and [replay screenshot](../output/playwright/f8-regions/replay.png); replay screenshot visually inspected. Result: output/regions-browser-retry.log.
- Initial browser script timed out locating a populated read-only textarea by exact label text. Switching that verification to its textbox role and accessible name passed; no production workaround was needed. Original log retained at output/regions-browser.log.
- Browser APIs were simulated and the image reused a real Godot PNG from the prior verified capture round. This proves production-component interaction, separately from the Core persistence test. No fresh engine protocol or native release acceptance was run in this slice. No Godot process was launched; the owned browser session and fixture server were closed.

