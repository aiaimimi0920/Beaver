# F8: Freeze and resume interactive preview

Date: 2026-09-26. This slice adds an explicit freeze/resume control to the production Godot preview. F8 and F9 remain open.

## Workflow and protocol

The viewer requests a new view revision with `frozen: true`. The engine driver pauses its SceneTree, applies the requested camera and viewport, waits for rendering, and produces one acknowledged PNG carrying the revision and frozen flag. It retains that exact sequence until the next view revision. The driver itself processes while paused, so it can accept resume without restarting Godot.

Core forwards and validates the frozen flag. The UI waits for the matching frame revision, disables camera and resolution changes while frozen, and permits saving the acknowledged original frame through the existing immutable archive. Resume sends a higher revision with `frozen: false`; stale frozen frames do not acknowledge the resumed view. Closing a frozen preview still terminates only its owned process, and the existing viewer lease remains active.

Desktop exposes the optional boolean on `validation.preview.view`. Existing callers omitting it request live mode. Saved legacy frames without the field remain readable as non-frozen frames.

## Scope limits

This is SceneTree pause plus stable image retention. Custom threads, scripts configured to process while paused, and external mutation are not guaranteed to stop. The production UI states this limitation. This slice does not certify a scene as safe for later raycasts, add real hit records, normalize regions, or submit visual feedback. A future picking workflow must enforce or prove the frozen geometry boundary before attributing hits to the image.

The frozen frame is captured after the engine acknowledges the freeze request. It need not be the previously displayed live frame. Original snapshots, saved records and acceptance evidence are not rewritten.

## Evidence

- Real Godot regression `live_preview_real_camera_close_and_lease`: 1 passed, 0 failed in 206.62 seconds. It verifies frozen frame stability, exact-frame archival, resume with sequence advancement, close while frozen, lease cleanup and SQLite reopen. Log: `output/freeze-real-retry.log`; engine PNG/JSON proof: `output/f8-freeze-real/`.
- The first run reached the lease check after passing freeze/resume assertions, then failed its fixed 17-second shutdown expectation. Lease expiration initiates asynchronous owned-process cleanup; the test now waits up to 15 additional seconds for closure after the idle interval. No production lease behavior changed. The first-run log remains at `output/freeze-real.log`.
- Frontend and adjacent regressions: 11 passed, 0 failed. TypeScript typecheck and Desktop Cargo check passed. Targeted Prettier and Rust formatting checks passed. Effective-line check: 1140 sources, 17 unchanged legacy files, 0 violations. Git whitespace check passed. No owned Godot preview processes remained after the run.
- Chromium production-component fixture passed freeze, disabled camera/resolution controls, save and resume: 2 view requests and 1 capture. The [frozen UI screenshot](../output/playwright/f8-freeze/frozen.png) was visually inspected. The fixture uses simulated APIs and an actual engine PNG; it is separate from real engine protocol evidence and does not constitute native release acceptance. Its browser and local server were closed.
