# R19 character handoff and recorded merge recovery

Completed on 2026-09-11 (Asia/Shanghai), continuing conversation
`01a08d8e-3333-72c1-803e-d3c7b50f7ce8`. The retained straw-hat character revision,
reusable production prompt changes and NPR laboratory EXE handoff are complete.
All four child tasks and the parent are completed and accepted, with no remaining
conflicts or active tasks.

## Deliveries

- Beaver: `release/Beaver-native-0.1.19-preview-r19-win32-x64/Beaver.exe`.
- Character and NPR laboratory: `release/Beaver-game-DlYQUM/Game.exe`.
- [Head comparison][head-comparison], [full-body comparison][full-comparison] and
  [four-view comparison][four-views].
- [Actual exported EXE window][exe-window] and [face view][exe-face].
- [Structured completion proof](../output/validation/npr-r19-completion-proof.json).

Keep each complete release directory, including licenses for Beaver and
`spoutlibrary.dll` for the game. Earlier releases and evidence were retained.

## Native package and fresh startup

The native EXE is 13,405,696 bytes, SHA-256
`9ae9348057dab6ef879cc29874aedc2bbd08e05163515ba3af3f5dee5e368fe7`.
The complete package contains 606 files and 17,017,941 bytes. The official
`verify-native-release.ts` check passed after packaged-host testing.

`scripts/start-fresh-native-test.ps1 -Build` compiled the native UI and release
host, then established a new blank project. A separate invocation without
`-Build` started the packaged R19 in another new blank round. Each round began
with zero projects and tasks and created exactly `export_presets.cfg`,
`main.tscn` and `project.godot`. Live packaged-host state confirmed one project
and zero tasks, and capability discovery exposed `task.retryMerge`.

- [Compiled-host startup proof][built-start].
- [Packaged-host startup proof][packaged-start].

The old host was closed before each launch. Process checks verified the selected
EXE path and singleton Beaver process. The packaged host was then restarted with
the original R16 data and WebView directories to continue the retained character
round. This recovery reused its two registered projects and five task records.

Both complete production/workflow skill files occur byte-for-byte in the R19
EXE. The production skill retains R17/R18 guidance on jaw-to-neck form, hair
roots, face-shadow diagnosis, bounds after topology edits and exported normals.
Its SHA-256 is
`875669747a8b138e43a793e946c2052fb22b33807cfc70dc5e7228e473284cb3`.
These embedding checks establish delivery of the prompts; their effectiveness
across new tasks still requires broader observation.

## Merge correction and regression proof

The final task had 174 recorded changes and 11 conflicts. Direct API reads and
hash comparison showed that each conflicted project file already matched the
task's recorded output. The previous conflict test only compared the project
against the original baseline.

R19 excludes such matching files from the pending merge and task rollback
ownership. Genuine differences still block the entire merge. The new
`task.retryMerge` API retries recorded, already-finished output without running
AI or recapturing later workspace edits. It preserves the original ID, report,
thread, baseline and approval policy. There is no force-overwrite option or new
UI button. Unfinished execution, review violations, pending questions, shutdown
and incomplete journal recovery remain blocked.

The regression was reproduced before the fix. Checks passed afterward:

- `cargo test --locked -p beaver-core task_ -- --nocapture`: 19 passed.
- `cargo test --locked -p beaver-desktop business_catalog::tests -- --nocapture`:
  2 passed.
- `cargo fmt --all -- --check`: passed.
- Native UI build and `cargo build --release --locked -p beaver-desktop`: passed.

Tests cover matching additions, modifications and deletions; preservation during
rollback; atomic rejection of genuine conflicts; retry of recorded output;
unfinished/review/shutdown rejection; and prevention of replay after completion.

On the retained task, one `task.retryMerge` call completed the merge with 163
owned changes and zero conflicts. The 11 matching imports remained outside its
rollback ownership. Recorded ID, report, thread, turn, baseline and workspace
matched before and after. The new events were `mergeRetry`, `merge` and
`status: completed`. The final child was accepted, the resumed parent reconciled
its four accepted children, and the parent was accepted through Beaver APIs.

## Character, laboratory and exported program

The retained project is
`output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/Fresh game`.
Its parent task is `98b362b6-e6a5-4634-9044-dfd82f4a72cd`; the final child is
`8985991d-f082-4738-b0ab-19b20be5d83a`.

The managed character work completed face/shadow refinement, white-hair grouping
and root cleanup, and clothing/hat structure and material refinement. Matched
comparison images and final laboratory resources are integrated into the main
project. The operator verified those report files through `asset.text` after
the R19 merge.

The retained final-task evidence records:

- Standard `validate` and `preview`: `ok=true`, `initialized=true`, `errors=[]`,
  `engineErrors=false`.
- Accepted source preservation, packed-image consistency, topology checks and
  matched capture settings: passed.
- Project laboratory: 67 passing checks.
- EXE embedded pack loaded through the bound editor: 67 passing checks. All
  three mesh arrays, 13 decoded NPR textures, axes and material profile match
  the project resources.
- Separate ordinary `Game.exe` window/control round: 18.42 seconds, seven
  captures/input steps, exit code 0, no forced termination or runtime errors.

The embedded-pack checks ran through the editor. The actual EXE window and
targeted control input have separate evidence. Earlier unsupported external
script/path probes are retained as failed probes and excluded from passing
runtime evidence.

The standard export already contains the final accepted assets, so it was
retained. The EXE is 87,008,800 bytes, SHA-256
`3ab50b763bc09ab4ca4444b826265e8e2b76ec241231716d9c0ef20cf303db9b`.
A fresh R19 `game.verifyExport` call passed with 3 inventory files and
87,378,682 bytes. This operator-side export and verification complete the
standard-export handoff described in the child's delivery documentation.

## Scope and remaining limits

All game production and game acceptance operations were conducted through Beaver
APIs and its managed tasks. The operator's source edits were limited to Beaver
and its reusable prompts; no game code, assets or game design documents were
authored outside that workflow to satisfy acceptance.

The character remains simplified and does not reach the Silver Wolf reference's
finish. One retained character round does not establish reliable first-pass
artistic production. The Beaver preview is unsigned and depends on its documented
external Windows runtimes. Native and game manifests retain
`runtimeVerified: false`; independent startup, content and interaction evidence
is recorded separately. Full product/migration acceptance is outside this record.

Earlier R16/R17/R18 records and intermediate progress JSON remain historical
evidence. This R19 record and its structured proof describe the final state.

[built-start]: ../output/validation/fresh-round-20260911-125519-79266e52ff324da5b9a0215c47600d35/start-proof.json
[packaged-start]: ../output/validation/fresh-round-20260911-131412-28fcb1884f4c46bea63cb1338c65afe2/start-proof.json
[head-comparison]: ../output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/Fresh%20game/artifacts/straw-hat-refinement/final-handoff-head-comparison.png
[full-comparison]: ../output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/Fresh%20game/artifacts/straw-hat-refinement/final-handoff-full-comparison.png
[four-views]: ../output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/Fresh%20game/artifacts/straw-hat-refinement/final-handoff-full-four-views.png
[exe-window]: ../output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/Fresh%20game/artifacts/straw-hat-refinement/final-handoff/existing-package-controls/lab-reset-full.png
[exe-face]: ../output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/Fresh%20game/artifacts/straw-hat-refinement/final-handoff/existing-package-controls/lab-face.png
