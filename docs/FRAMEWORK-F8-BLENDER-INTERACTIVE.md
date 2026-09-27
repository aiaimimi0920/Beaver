# F8 Blender interactive camera and archived annotations

Completed on 2026-09-26 as a focused development slice. F8 and F9 remain open.

## Observable workflow

A completed frozen Blender scene now exposes the production interactive preview. Users can orbit, pan, zoom, reset the observer camera, and choose 540p, 720p or 1080p. Freezing locks camera and resolution controls and preserves the confirmed PNG bytes. Users can draw numbered image regions, enter comments, save an immutable frame, close the session, and replay the frame and annotations without starting Blender. Existing archived-feedback consumers remain available.

The Core restores the validated snapshot into a temporary project and runs Blender with factory startup, disabled script auto-execution and isolated user directories. The shared preview monitor owns revisions, frame validation, session identity, cancellation and lease expiry. Blender uses a copied observer camera and command-driven Workbench rendering. It renders a static scene only when an active view changes. Existing frozen dependency checks still reject missing or out-of-snapshot resources.

## Verification

- Real Blender 5.2.1 LTS: `object_blender_live_camera_archive_and_lease` passed, 1 test in 38.10 seconds. Covers deleted mutable source, snapshot rendering, session reuse/exclusion, camera changes with visible geometry, resolutions, reset pixels, exact frozen PNG identity, annotation archive, close/reopen, lease expiry, and restart replay with engine launch forbidden. Log: `output/blender-live-core-final.log`.
- Existing real frozen Blender render/dependency regression passed earlier in this slice; its valid evidence was reused.
- Shared-monitor adjacent real Godot regression `live_preview_real_camera_close_and_lease` passed, 1 test in 210.54 seconds. Log: `output/blender-live-godot.log`.
- Frontend focused tests passed: 13 tests, 0 failures. Log: `output/blender-live-ui-tests.log`. Desktop Cargo check passed: `output/blender-live-desktop.log`.
- Final TypeScript check, Prettier check, targeted Rustfmt check, Python AST parsing, and `git diff --check` passed. Effective-line check: 1175 sources, 0 violations; baseline unchanged. Logs: `output/blender-live-typecheck-final.log`, `output/blender-live-format-final.log`, `output/blender-live-structure-final.log`.
- Chromium exercised production ObjectScenePreview, LiveScenePreview and SavedPreviewFrames with simulated transport and real archived pixels: camera drag/reset, resolution changes, frozen control locks, numbered region/comment save, page reload, read-only replay and no preview-open API on replay. Screenshot: `output/playwright/blender-live/replayed.png`; execution records: `replay-proof.log` and `readonly-proof.log` in that directory. Browser and test server were closed afterward. This UI fixture does not prove a browser-to-engine transport integration.

## Corrections made during verification

Blender embeds render-time PNG metadata, so identical rerenders need not have identical file hashes. Reset checks compare decoded pixels; freeze/unfreeze of an unchanged view reuses the previous PNG bytes. A fixed orbit depth could turn a small model out of view; orbit depth now follows the frozen mesh bounds and a visible-pixel assertion protects the regression. Live and saved viewer siblings now use distinct React keys.

## Remaining boundaries

This slice supports image-region annotations only for Blender. Blender mesh picking, topology relocation, remaining F8 integration and F9 delivery work are still outstanding. Perspective and orthographic observer cameras are supported; panorama cameras fail explicitly. Animation, final shader/light fidelity and precise visible-surface selection are outside this implementation. No full native Beaver.exe acceptance, external release, version bump, commit or push was performed. The overall development plan is not complete.
