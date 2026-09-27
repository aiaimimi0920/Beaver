# F8 Live preview resolution switching

Date: 2026-09-26

## Delivered workflow

The production Godot live viewer switches between 540p (960 x 540), 720p
(1280 x 720) and 1080p (1920 x 1080) without opening another session.
Camera orbit, pan and zoom remain unchanged. The selector shows the desired
resolution; the image caption reports actual decoded engine-frame dimensions.
While a change is pending, the previous image remains visible with a pending
message. Original saved captures and durable validation runs are unchanged.

Core applies camera and dimensions as one revision. Identical retries are
idempotent; changing dimensions under the same revision is rejected. Only the
three presets may be selected; an existing non-preset capture size can remain
unchanged for camera-only commands. The engine resizes its viewport before
capture. Core accepts frames only for the current revision and dimensions,
checks PNG dimensions, and calculates the image hash. The UI also rejects
outdated revision/size responses and coalesces queued view changes.

## Verification

- Real Godot regression: 1 passed, 0 failed, 88.17 seconds. Within one session,
  540p -> camera movement -> 720p -> 1080p -> 540p produced actual PNGs with
  matching dimensions and unchanged camera transforms across resizes.
  Duplicate commands, invalid dimensions, revision conflicts, close/replacement,
  idle expiry and unchanged durable run were checked.
  Engine metadata: `4.5.1-rc (custom_build)`.
- Frontend regressions: 4 passed. Includes atomic camera/resolution coalescing,
  late wrong-size/revision frames, failed-command retry and session isolation.
- `cargo check --locked -p beaver-desktop`, `npm run typecheck`: passed.
- Targeted rustfmt, Prettier, gdformat and gdlint: passed.
- `npm run check:effective-lines`: 1137 sources, 0 violations.
  Session ownership module: 251 effective lines (cohesive); worker: 175;
  client controller: 217; production component: 173. Baseline unchanged.
- `git diff --check`: passed.
- Chromium production-component smoke with a mocked API and real engine PNGs:
  selected 720p, 1080p and 540p, verified natural image dimensions and no decode
  alerts, then closed the viewer. Screenshot visually inspected.
  This is UI evidence separate from the real Core engine regression.
- Browser and fixture server closed; owned live-preview Godot processes: 0.

The initial combined browser shell waited for its open command to exit.
The smoke was therefore executed separately and passed; closing that browser
released the earlier shell, whose redundant smoke reported session-not-open.
Use `output/live-resolution-browser-final.log` for the completed browser check.

## Evidence

- [720p PNG](../output/f8-live-resolution-real/720p.png)
- [1080p PNG](../output/f8-live-resolution-real/1080p.png)
- [540p PNG](../output/f8-live-resolution-real/540p.png)
- [1080p engine metadata](../output/f8-live-resolution-real/1080p.json)
- [Production selector screenshot](../output/playwright/f8-resolution/1080p.png)
- [Browser fixture](../output/playwright/f8-resolution/harness.tsx)
- [Browser workflow](../output/playwright/f8-resolution/smoke.js)

Logs: `output/live-resolution-{real,desktop,ts,types,format,lines,diff,browser-final}.log`.

## Remaining boundaries

This closes the live resolution portion of F8.2. Blender bindings, actual
engine picking, persistent frozen interactive frames for feedback, topology-aware
relocation and the remaining F8/F9 integration work remain open. No native
release acceptance, package, publication or version bump was performed.
The full Beaver development goal remains active.
