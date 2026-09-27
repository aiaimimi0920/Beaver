# F8 Frozen preview resolution

Date: 2026-09-26

## Delivered workflow

Version and attempt checkpoint scene previews accept 540p (960 x 540),
720p (1280 x 720), and 1080p (1920 x 1080). The production preview exposes
a next-render resolution selector and displays the actual saved dimensions.
Active renders and unconfirmed requests lock the selector. Retrying a lost
response preserves both request ID and resolution.

Core stores dimensions in the immutable capture flow. Latest-per-target lookup
retains its existing persistent key. Omitted resolution preserves the old
default and request shape. An active render with another size returns
PREVIEW_RESOLUTION_ACTIVE_CONFLICT. After a terminal run, a new render may use
another size without modifying previous evidence or request receipts.

## Verification

- Core scene_preview: 4 passed, 2 opt-in engine tests ignored.
- Real attempt_scene_preview_real_hd_capture: 1 passed in 52.68 seconds.
  The fixture freezes attempt output, deletes its workspace scene, runs the
  existing visual service, and checks completed status, evidence integrity,
  1280 x 720 dimensions, and the expected green center pixel.
- Desktop scene_preview_dispatch: 2 passed, including 1080p through the public
  contract and project-local persistent lookup.
- Frontend object-scene-preview: 4 passed, including lost-response resolution
  locking and exact request retry.
- TypeScript typecheck passed. Targeted Rustfmt and Prettier checks passed.
- Effective-line gate: 1126 sources, 0 violations; existing legacy debt retained.
- git diff --check passed. Independent narrow source review found no actionable
  replay, resolution-conflict, or UI retry issue.
- Chromium production component with a mocked API: selected 720p, verified API
  input and active selector lock, cancelled, reloaded, decoded the real
  1280 x 720 PNG, and displayed the saved dimensions.

Logs: output/resolution-{core,desktop,ts,types,lines,real,diff}.log.

Engine proof: [result](../output/f8-resolution-real/result.json),
[PNG](../output/f8-resolution-real/frozen-scene.png).
Browser proof: [screenshot](../output/playwright/f8-resolution/completed.png),
[fixture](../output/playwright/f8-resolution/harness.tsx),
[workflow](../output/playwright/f8-resolution/smoke.js).

## Boundaries and remaining work

The real capture is a Core engine fixture, not native game-creation or release
acceptance. Browser API responses are mocked. No compiled Beaver host was
launched. Trusted scene scripts execute; workspace isolation is not an OS
sandbox. Real engine capture was verified at 720p; 540p has earlier engine
evidence and 1080p is covered by persisted Desktop/Core flow configuration.

This closes frozen-preview resolution selection only. F8.1-F8.5 remain open:
live camera controls, bounded live sessions/frame transfer, Blender preview,
true engine picking and topology-aware feedback still require implementation.
F9 and the wider development-plan audit remain pending.

