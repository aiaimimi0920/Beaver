# F8 frozen Godot scene preview

Date: 2026-09-26. Development slice; no release or full native acceptance.

## Usable workflow

Object version details now offer a Godot scene preview for frozen .tscn/.scn
members. The user explicitly renders, sees queued/running/terminal status,
cancels active work, and reopens the same target to retrieve its latest durable
result. Terminal results can be rendered again; active work is deduplicated.
Lost run responses retry the same request identity.

The Core selects the exact historical version and pinned dependency closure,
checks independently stored version records and blob integrity, and restores
those frozen files into the existing visual worker workspace. No current
project scene or project configuration is substituted. If project.godot is not
in the frozen closure, the preview uses a declared minimal config_version=5
configuration. Scenes depending on project settings/autoloads must include
their needed frozen configuration; failures are visible.

The fixed 960 x 540 capture uses the scene's own camera after 30 settling frames.
The UI records version, scene SHA-256, snapshot, engine/runner, capture time and
image SHA-256. Trusted scripts execute: workspace isolation is not an OS sandbox.

## Ownership and persistence

- object.scenePreview.run/get are registered Desktop contracts and route through
  the open project runtime. Mismatched project identities fail.
- objectScenePreview stores target-to-latest-run mapping atomically with the
  run and idempotent receipt. Previous run evidence remains in validation storage.
- objectPreview reuses the durable visual worker lane, cancellation, process
  cleanup and startup interruption recovery without registering a validation flow.
- Preview runs are excluded from validation listings and acceptance confirmation.
  Ordinary validation rerun rejects them before live snapshot/import side effects.
- Completed media uses existing evidence integrity validation and asset serving.
- Production state suppresses late results after mutation/close, retries a fresh
  query after an older in-flight query, and polls active runs only while visible.

Primary implementation: native/core/src/object_scene_preview.rs,
native/core/src/object_import_snapshot.rs, native/core/src/validation/,
native/desktop/src/data_dispatch_object_catalog.rs and
src/ui/object-preview/ObjectScenePreview.tsx.

## Fresh verification

All commands ran from the workspace with the normal PowerShell toolchain.

- cargo test --locked -p beaver-core --lib object_scene_preview:
  2 passed, 1 intentionally ignored engine test. Log:
  ../output/f8-scene-core-final.log.
- The ignored engine test was explicitly run with BEAVER_TEST_GODOT and
  BEAVER_PREVIEW_PROOF: 1 passed in 57.82 seconds. It deletes the live scene
  before enqueue, runs the actual managed service, checks completed evidence
  integrity and dimensions, and verifies the center pixel is green.
  [Actual image](../output/f8-scene-real/frozen-scene.png),
  [recorded provenance](../output/f8-scene-real/result.json),
  [engine log](../output/f8-scene-real.log).
  Recorded engine version: 4.5.2.rc.custom_build.2890667c8.
- cargo test --locked -p beaver-core --lib object_import: 28 passed.
  This covers the shared dependency selection/import boundary.
- cargo check --locked -p beaver-desktop passed.
- cargo test --locked -p beaver-desktop object_scene_preview_dispatch:
  1 passed; registered contracts, deleted live source, request replay,
  cross-project rejection and project-local writes.
- npx tsx --test tests/object-scene-preview.test.ts
  tests/object-version-file.test.ts: 7 passed. Includes response-loss retry,
  close/late-read isolation and the successful-mutation refresh race.
- npm run typecheck passed. Targeted Prettier and Rustfmt checks passed.
- npm run check:effective-lines: 1123 sources, 0 violations; exit code 0.
  Existing historical baseline was not changed.
- Chromium exercised the production scene component with a simulated API:
  render, queued page reload, cancel, completed page reload, and successful
  decoding of the actual engine-produced 960 x 540 PNG.
  [Browser proof](../output/playwright/f8-scene/completed.png),
  [harness](../output/playwright/f8-scene/harness.tsx),
  [interaction script](../output/playwright/f8-scene/smoke.js).
  This proves browser interaction separately from real Core engine execution;
  it is not a live Desktop API/WebView end-to-end test.

## Remaining work

F8.1-F8.5 remain open. This slice closes frozen object-version Godot capture,
not the entire preview framework. Interactive camera/session/frame transport,
resolution controls, Blender rendering, attempt/child scene binding, real
engine selection, and feedback relocation across old topology remain.
The browser proof does not establish native media/WebView integration or
formal release acceptance. No full game-creation acceptance was performed.

