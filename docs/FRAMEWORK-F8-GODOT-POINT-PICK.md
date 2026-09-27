# F8 Godot frozen static mesh point picking

Date: 2026-09-26. Status: bounded workflow delivered; F8 and F9 remain open.

## Usable workflow

Freeze a Godot preview, enable image selection and static-mesh point picking, then click the image. The production editor shows the engine-reported node path and world position, or an explicit miss, beside the numbered region and its context thumbnails. Region comments and deletion preserve the associated receipt; deletion renumbers the remaining associations. Saving the frame retains the selection in the immutable archive. Existing archive feedback delivery carries the full selection and receipt in image provenance.

The capability is named `frozen-static-mesh-ray`. The driver copies world-space triangles and the camera projection at the frozen render boundary; subsequent picks query that snapshot. Receipts include session, revision, sequence, PNG digest, request ID and normalized point. Core accepts only matching owned-engine receipts, supports idempotent request replay, and rejects conflicting IDs, stale frame identities and forged saved selections. Desktop exposes `validation.preview.pick`; the UI polls a correlated request and rejects late results after the frame changes.

## Verification

- TypeScript: 7 tests passed across `preview-picking`, `preview-selection` and `publication-frame-picker`; `npm run typecheck` passed. Logs: `output/pick-ts-tests.log`, `output/pick-types.log`.
- Core receipt regression: 1 passed, including integer/float point round-trip and foreign-frame rejection. Log: `output/pick-core-tests.log`.
- Publication frame regressions: 3 passed. A real worker/RPC fixture verifies selection persistence after reopening, feedback image provenance, and rejection of a tampered node path. Log: `output/pick-feedback-tests.log`.
- `cargo check -p beaver-desktop` passed. Log: `output/pick-desktop.log`.
- Real graphical Godot regression: `cargo test -p beaver-core --lib live_preview_real_camera_close_and_lease -- --ignored --nocapture` passed, 1 test in 123.36 seconds. The center ray hits Box at world Z = 0.5 with normal Z = 1; a corner ray misses. It also covers request replay/conflict, stale selection rejection, capture, Store reopen, resolution/camera changes, resume and session close. Log: `output/pick-engine-final.log`; PNG/JSON artifacts: `output/pick-engine-proof/`.
- Engine executable: `Z:\project\godot-4-4-1\bin\godot.windows.editor.x86_64.console.exe`. The executable reports `4.5.2.rc.custom_build.2890667c8`; the directory name does not identify its actual version.
- Chromium production-component replay: enabled point mode, selected the recorded center hit, edited its comment, selected the recorded corner miss, deleted the first region, checked receipt renumbering, saved, reloaded and checked identical selection state, then opened read-only replay. Screenshot: `output/playwright/point-pick/replayed.png`. This fixture replays prior real-engine receipts and uses localStorage; it does not establish native browser-to-Desktop API acceptance. The only console resource error was the fixture's missing favicon. The isolated browser session and HTTP server were closed.
- GDScript, targeted Rust and TypeScript formatting checks passed. Effective-line check: 1167 sources, 17 unchanged historical files, 0 violations; baseline unchanged. `git diff --check` passed.

## Failures resolved during development

Godot parses command JSON numeric fields as floats. The driver now writes integral revision and sequence fields so Core identity matching accepts genuine receipts. PrimitiveMesh does not expose the ArrayMesh blend-shape API, so that check is limited to ArrayMesh. An isolated property-list probe confirmed the BaseMaterial3D property is `grow`; the initial `grow_enabled` access prevented geometry collection and was corrected. Triangle normals now follow Godot's clockwise winding, with a real-engine assertion. One intermediate rerun omitted `BEAVER_TEST_GODOT`; the final passing run supplied it and `BEAVER_PREVIEW_PROOF`.

## Scope and remaining work

The ray queries supported static mesh triangles, not exact rendered-pixel identity. Skinning, blend shapes, ShaderMaterial, transparency, billboards, grow, overlay and next-pass materials are excluded. Unsupported objects may occlude a supported mesh, and renderer LOD, visibility-range and other custom rendering effects are not reproduced. UI and receipts identify this restricted capability and expose skipped-node counts. Limits are 100000 triangles, 32 requests per frozen frame and 8 numbered regions.

There is no complete box-selection hit set, cross-version topology remapping or Blender picking. Historic node paths/triangle indices require relocation before use against changed topology. A timed-out pending request can require resuming and freezing again. No full native release acceptance, external model-provider acceptance or new release packaging was performed. Existing unrelated compiler unused-mut warnings remain. F8.1-F8.5 and F9 remain unchecked pending their full acceptance conditions.
