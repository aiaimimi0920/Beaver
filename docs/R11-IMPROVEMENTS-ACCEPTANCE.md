# R11 improvements and fresh acceptance

Date: 2026-09-09. Status: R11 export passed; first-play fix passed fresh R12 verification. See [R12-ACCEPTANCE.md](R12-ACCEPTANCE.md) for the final release and results.

## Implemented

- Native standard export imports the same frozen snapshot before export, checks
  engine import capability, preserves import/export logs, and rejects nonzero
  exit codes and engine errors. Windows namespace paths are normalized only at
  the engine boundary.
- First play now runs the same import check on the merged project and retains a
  uniquely named `play-import-*.log` in the Beaver data directory. This fixes missing
  imported font resources in a project whose `.godot` cache was never generated.
- Task merge completion, user acceptance and export-snapshot verification have
  separate UI meanings. Project delivery status is persisted on both successful
  and failed export/verification calls. Runtime verification remains false.
- Changed recognized text files are normalized to UTF-8 without BOM before merge.
  Resource reading strictly supports UTF-8 and BOM-marked UTF-16. Unknown encoding
  is not silently replaced; a bounded raw-byte API and download action are exposed.
- Running conversations refresh events every 1.5 seconds with an in-flight guard.
- A terminal allowlisted transient AI failure may resume once in the same thread,
  preserving the original deadline and recording recovery. Authentication, quota,
  permission, context and ambiguous startup failures do not trigger automatic replay.
- Fresh native test launch closes old Beaver instances before compilation and
  creates an isolated blank project and empty task store.

## Fresh verification

- TypeScript: 88 tests passed, 0 failed; `npm run typecheck` passed.
- Rust: 89 core tests and 3 desktop tests passed, 0 failed.
- Prettier and rustfmt checks passed for the final changed source.
- Independent read-only audit found no concrete newly introduced defect in the
  export, delivery, encoding and raw-resource boundaries.
- Release package: `release/Beaver-native-0.1.19-preview-r11c-win32-x64`, 606 files.
- Packaged executable equals the running acceptance executable, SHA-256:
  `d721d96f7ac51d52ad7593911a0334823a29c3b6e5b33c68251300a97c35c54c`.

## Current fresh round

- Evidence root:
  `output/validation/fresh-round-20260909-050622-f56e56d56b7a4d50b9511081d12bdf9a`.
- Project: `d07d81a3-449a-4382-95fa-dc3c74a898c7`.
- Task: `3150bd43-11a9-4300-bb1d-24900cf38d3c`.
- Launcher stopped old PID 35500 and started PID 28316. Initial store contained
  zero projects/tasks; initial project contained only `project.godot`, `main.tscn`
  and `export_presets.cfg`. No prior game's files were reused.
- Submitted the original short Chinese game request through `task.create`.
- Global ask ratio was 30; task initially followed global settings.
- Beaver asked three important questions, each with three options and a recommended
  answer. Two recommended options and one short custom answer were submitted using
  `task.answer`; execution resumed.
- Live UI check observed 3 `task.events` refreshes in 5 seconds. Earlier polling
  assertions made while the task was awaiting input or had failed were harness
  timing mistakes; the running-state check passed.
- Actual Chinese decision resource: 414 bytes, exact UTF-8/raw-byte round trip,
  no BOM, no replacement character. `../project.godot` was rejected by the raw API.
- The real UI raw-download button produced the same 414-byte Chinese document;
  filename, length, UTF-8 and no-BOM checks passed.
- The main task completed with 17 changed files and was accepted. A followup task
  `b9959b1f-1e58-496f-bb02-902e4222a60f` corrected external template references,
  completed and was accepted. Its task-level ask ratio 0 overrode global 30.
- Standard export then passed. `game.verifyExport` independently verified 2 payload
  files totaling 110,057,179 bytes in `Beaver-game-NBKTwi`. Persisted delivery became
  `verified`, with `runtimeVerified: false` retained.

## Issues encountered and recovery

1. Imported local provider configuration returned HTTP 403, limiting access to
   official clients. Beaver correctly did not retry this as a transient error.
   The previously used authenticated local provider was restored via settings API.
2. The initially detected Godot 4.0.2 first-frame headless import returned exit 1
   with warnings only; that version does not support the explicit import command.
   Beaver now reports unsupported import capability rather than hanging or accepting
   a failed export. ANSI-colored command help is covered by a regression test.
3. The local directory named `godot-4-4-1` actually contained a custom 4.5.2 RC build.
   Version detection exposed the mismatch; no success was inferred from its name.
4. `tools.install` ensured an already detected Godot was present and retained 4.0.2;
   it did not upgrade it. The old official-template checksum request returned 404.
   The creative task was asked, through Beaver, to prepare a compatible official
   stable toolchain and keep tools/downloads outside the game project.
5. Beaver's AI encountered a rejected same-file delete/add patch and reported it
   before retrying a single update. Successful downloads were not repeated.
6. The first standard export of the merged game rejected `../` template paths.
   A short followup through Beaver corrected them to absolute external paths and
   verified both its workspace and a relocated copy. The parent then successfully
   ran the real standard export and verification APIs.
7. The actual first `game.play` call launched a gray window. The engine log showed
   missing `.godot/imported/interface.ttf-*.fontdata` and consequent script preload
   failure. First play now imports the merged project before starting the game;
   merely observing a live process was insufficient. A new blank round is required
   to validate the fix without reusing generated caches.
8. The accepted task still displayed a generic pending-acceptance status label.
   The generic completed label is now simply "已合入"; the separate acceptance
   action/state continues to indicate whether the user has accepted it.

The upstream command semantics are documented in the
[Godot 4.3 CLI documentation](https://docs.godotengine.org/en/4.3/tutorials/editor/command_line_tutorial.html).
The old headless first-import behavior is also reported in
[Godot issue 83449](https://github.com/godotengine/godot/issues/83449) and
[issue 77508](https://github.com/godotengine/godot/issues/77508).

## Completion boundary

R11 creative tasks, standard export and bundle verification passed; R11 first-play
visual verification failed and exposed an additional import boundary. Fresh R12
verification of that fix passed, including an uncached project with a bundled
Chinese font and a visibly rendered Chinese start screen. The release manifest retains its broader
preview/runtime and migration limitations.
Game quality and fun are outside this round's primary acceptance requirement.
