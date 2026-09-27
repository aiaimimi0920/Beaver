# F8 Live Godot camera preview

Date: 2026-09-26

## Delivered workflow

From a completed, integrity-checked frozen scene preview, the production object
preview opens an independent Godot session. The user can orbit with left drag,
pan with Shift-drag, zoom with the wheel, reset the camera, and close the viewer.
Version and attempt scene previews share this entry point. The original saved
capture and validation run remain unchanged; interactive frames are not acceptance
evidence and do not cancel production tasks.

Core restores the original immutable snapshot into a temporary sandbox. It checks
project/run ownership, completed capture integrity, camera bounds and revisions.
The Desktop catalog and dispatch expose `validation.preview.open/read/view/close`.
Repeated open with the same request reuses its session; only one live session is
allowed globally. A stale close cannot close a replacement session.

The engine emits actual PNGs with frame sequence, camera transform/projection,
resolution and engine version. Core checks dimensions, limits PNG size to 16 MiB,
and calculates SHA-256 before transferring the latest frame. UI verifies source
and session identity, rejects stale frames, coalesces camera updates, and retries
failed latest updates. Closing during an outstanding open releases the late session.

Visible viewers poll every 500 ms. Capture pauses after 2 seconds without reads;
the session expires after 15 seconds. Hidden viewers do not renew the lease.
Reopening after expiry starts a new session. Explicit close and application
shutdown stop the owned process and clean its temporary files.

## Verification

- Real engine regression `live_preview_real_camera_close_and_lease`: 1 passed,
  0 failed. Created a fresh 3D scene, captured it, deleted its workspace scene,
  and opened the frozen snapshot. Actual 960 x 540 images and camera matrices
  changed after the camera command. Checked request reuse, session limit,
  wrong-project rejection, stale revision rejection, explicit close, replacement
  protection, idle expiry, and unchanged durable run. Runtime reported
  `4.5.1-rc (custom_build)`.
- Frontend regressions: 3 passed, 0 failed; cover lost-open retry, close during
  open, source/session/stale-frame isolation, coalescing and failed-write retry.
- `npm run typecheck` and `cargo check --locked -p beaver-desktop`: passed.
  Existing unrelated Rust warnings remain.
- Targeted Prettier, rustfmt, gdformat and gdlint: passed.
- `npm run check:effective-lines`: 1137 sources, 0 violations; baseline unchanged.
- `git diff --check`: passed.
- Chromium smoke with the production live component and mocked API: decoded the
  real engine PNG, opened the viewer, dragged its camera, reset and closed it.
  This is separate UI evidence, not native end-to-end acceptance. Browser session
  and fixture server were closed. No owned live-preview Godot processes remained.

Proof: [initial PNG](../output/f8-live-preview-real/initial.png),
[moved PNG](../output/f8-live-preview-real/moved.png),
[initial metadata](../output/f8-live-preview-real/initial.json),
[moved metadata](../output/f8-live-preview-real/moved.json),
[production component screenshot](../output/playwright/f8-live/camera.png),
[browser fixture](../output/playwright/f8-live/harness.tsx),
[browser workflow](../output/playwright/f8-live/smoke.js).
Logs: `output/live-preview-{real,ts-tests,types,desktop,format,rustfmt,gd-format,gd-lint,lines,diff,browser-final}.log`.

## Remaining boundaries

Scenes require an active Camera3D; 2D and camera-less scenes report unsupported
instead of fabricating a view. Orbit uses a fixed pivot 10 units along the initial
camera direction, without object framing or picking. The session uses the saved
capture resolution; changing live resolution is not implemented. Frame transfer
uses polling and base64, not streaming video. Temporary frames are not persisted
for feedback or recovery after host restart.

Blender binding, real engine selection, frozen interactive-frame feedback,
topology-aware relocation, remaining F8 requirements and F9 integration/release
evidence remain open. No native host release acceptance, packaging or publishing
was performed. The complete Beaver development goal remains active.
