# F8: Frozen attempt scene previews

Date: 2026-09-26

## Usable workflow

Open an attempt checkpoint file, choose a Godot .tscn or .scn member, and select
Godot scene preview. Rendering uses the entire immutable input or output
checkpoint, with project/run/attempt/checkpoint/path/hash identity. The production
view supports durable latest-result lookup, queued/running cancellation, and
reopening the same target. Provenance shows the attempt, run, checkpoint, scene
hash, capture snapshot, engine and image hash.

Core verifies checkpoint membership and every frozen blob before scheduling.
Deleting the live scene does not affect scheduling. Corrupt dependencies fail
before a new preview is created. The dedicated Desktop methods are
object.attemptScenePreview.run and object.attemptScenePreview.get. Mutations
notify the existing managed visual queue. Preview captures remain excluded from
acceptance evidence.

Version and attempt previews share durable scheduling but retain typed identity
keys. Existing version preview storage keys and request method names are
preserved. Input and output checkpoints cannot reuse one another's result.
The UI remounts its preview session whenever the selected frozen file changes.

## Fresh verification

- Core: cargo test --locked -p beaver-core --lib scene_preview:
  3 passed, 1 real-engine test deliberately ignored. Includes the unchanged
  version regressions and a new attempt output/deletion/replay/input separation/
  foreign run/corrupt dependency regression.
- Desktop: cargo test --locked -p beaver-desktop scene_preview_dispatch:
  2 passed, including version durability and attempt contract/routing rejection.
- Frontend: npx tsx --test tests/object-scene-preview.test.ts: 4 passed.
  Covers lost-response request reuse, stale reads, mutation refresh, and
  cross-checkpoint response rejection through the dedicated API.
- npm run typecheck passed.
- Targeted Rustfmt and Prettier completed. git diff --check passed.
- npm run check:effective-lines: 1126 sources, 0 violations.
- Chromium production ObjectAttemptFilePreview component with a mocked API:
  open explicit scene preview, render, reload queued state, cancel, reload
  completed state, and decode the prior real capture at 960 x 540.
  Harness and screenshot: [browser artifacts](../output/playwright/f8-attempt-scene/).
- Independent read-only source review found no actionable integration issue.

Logs: output/attempt-scene-core.log, output/attempt-scene-desktop.log,
output/attempt-scene-ts.log, output/attempt-scene-types.log,
output/attempt-scene-lines.log. Initial compile exposed test-only imports removed
by extraction; imports were corrected before the successful Core run. An initial
Desktop command used an unsupported --lib target; the corrected binary test
command above passed.

## Evidence boundaries and remaining work

The actual Godot capture worker is unchanged; its real-engine evidence is retained
in [the preceding version-preview slice](FRAMEWORK-F8-FROZEN-GODOT-PREVIEW.md).
This slice did not repeat a real-engine capture from an attempt or native release
acceptance. The browser API is mocked and its PNG comes from that preceding
real-engine run; it does not prove a new attempt produced the displayed image.

The scene's trusted scripts execute; workspace isolation is not an OS sandbox.
Camera interaction, Blender preview, live frame/session controls, real picking,
and topology-aware feedback remain open. F8.1-F8.5 and the overall development
goal remain unfinished.

