# R13 acceptance

Status: the requested changes passed implementation, full-workflow acceptance,
and final packaged-EXE regression checks. The release remains an unsigned preview.

## Implemented changes

- Task board columns and the child-task list use vertical scrolling without
  per-column page counters or previous/next controls.
- The ICO keeps all eight sizes, with the 256 px entry first so Tauri uses the
  high-resolution image for the runtime window icon.
- General creation requests use AI clarification followed by a validated,
  model-generated executable plan. Each step becomes an independent child task.
  Children see the latest integrated project when they start and wait for their
  predecessor's approval. Parent completion requires all children to be approved.
- Automatic and manual approval share the API and UI. Approval history records
  its source. Plans can be paused/resumed without generating duplicate children.
- Pending child workspaces cannot be opened before their dependencies are ready;
  waiting parents cannot submit stale follow-up drafts through a disabled editor.

See [TASK-EXECUTION-PLAN.md](TASK-EXECUTION-PLAN.md) for the lifecycle contract.

## Fresh native round

The round started through `scripts/start-fresh-native-test.ps1 -Build`.
Previous Beaver host PID 53588 was closed. New host PID 31040 used unique data,
WebView storage, and a blank Godot project with zero prior projects/tasks.

- Start proof: `output/validation/fresh-round-20260910-005816-522d6732e5cf4acb8ea5ca5bc7196aa1/start-proof.json`.
- Executable SHA-256: `CD1D9D53FE01BAC592C5BCE3CE64858DD5991D611699214ADB1719988103679D`.
- Project: `a9345a57-47ae-4718-92bf-5c3ac5402524`.
- Parent: `472d7e80-557d-4020-a872-5e4ea08d01b2`.
- Initial project contained only `project.godot`, `main.tscn`, and `export_presets.cfg`.

The user's original short game request was submitted unchanged through
`task.create`. Beaver asked three questions with three suggested options each.
The test selected two recommendations and supplied one short custom answer:
an approximately one-minute Chinese minimal playable version. Subsequent
questions use the task's fully automatic policy. Game creation remains entirely
inside Beaver; the acceptance operator does not invoke Codex, Godot, or Blender,
or write game content.

Beaver submitted four real steps: combat and the complete game loop, random
upgrades, narrative/presentation/audio integration, and validation/export checks.
The parent entered `waitingChildren`; the first child started and the remaining
three stayed queued. The first child's approval policy was switched to manual
through `task.approval` to test the dependency gate.

## Verified evidence

- TypeScript: typecheck and all 91 tests passed.
  Log: `output/validation/r13-ts-all.log`.
- Rust core: all 93 tests passed, including real snapshot/merge/approval plan tests.
  Log: `output/validation/r13-core-all-final.log`.
- Targeted Prettier check and `cargo fmt --all -- --check` passed.
- Native release compilation passed in 4m 34s.
- Live HTTP API and business MCP both expose 47 methods. MCP initialization,
  tools discovery, and `state` completed successfully; the bridge exited with 0.
- The first child completed with 18 changed files and remained unapproved under
  its manual policy. All three successors stayed queued with no prepared workspace.
  Pausing the parent interrupted those queued children; continuing restored them
  without new IDs or tasks and preserved the manual approval gate. Explicit
  `task.accept` recorded `approvalSource: user` and automatically started child 2.
- All 18 integrated file hashes from child 1 matched child 2's starting snapshot.
  Child 2 subsequently completed with `approvalSource: automatic`, releasing
  child 3. Its integrated hashes also matched child 3's starting snapshot.
- All four children completed: 18, 9, 38, and 119 changed files respectively.
  The first used manual approval; the remaining three used automatic approval.
  The parent then completed and was explicitly accepted. There were exactly five
  tasks throughout the round. Evidence: `output/validation/r13-workflow-proof.json`.
- Beaver standard export and independent `game.verifyExport` passed: two payload
  files totaling 111,238,461 bytes, entry `Game.exe`. The persisted delivery state
  is `verified` with `runtimeVerified: false` and scope `export-snapshot`.
  Bundle: the fresh-round directory above, under `Beaver-game-PaNI8U/`.
- Live runtime icon handles contain 256 x 256 images. Shell extraction also
  produced the intended 32 x 32 and 16 x 16 variants with both brand colors.
  Images: `output/validation/r13-icon/`.
- At a 1120 x 480 viewport, a lane contained all three pending task cards with
  client height 314 px and scroll height 426 px. Mouse-wheel input moved it to
  scrollTop 112 px. Column pagination element count was zero.
  Screenshot: `output/validation/r13-lane-scroll.png`.
- Clarification UI screenshot: `output/validation/r13-clarification.png`.

## Issues observed and recovery

The preceding fresh round failed before reaching the AI because the new plan
tool omitted `type: function` while the existing question tool used the canonical
format. The actual error was `dynamic tools must use either canonical or legacy
format consistently`. The tool specification was corrected, a regression test
was added, and the new blank round above reached clarification and plan execution.
The failed round remains at
`output/validation/fresh-round-20260910-002358-7b5d2940d5d04ce98ce98e57842fbcc0/`.

During child execution, the AI encountered a rejected patch operation and a
synthetic keyboard-input test failure. Beaver retained the execution history and
the child continued its ordinary correction/validation flow. These did not stop
the acceptance round. The configured Godot executable path contains `4-4-1`,
but Beaver's actual version check reports `4.5.2.rc.custom_build.2890667c8`.

The custom engine had no recognizable matching official template release.
`game.prepareTemplates` rejected it. A short follow-up to the running child asked
Beaver to use an official stable engine with tools kept outside the game project.
Beaver prepared Godot 4.5.1 and reused matching installed templates. The operator
selected that engine through settings API, then ran standard export and bundle
verification through Beaver. The first export call incorrectly named a nonexistent
destination; retrying with the existing round directory succeeded.

The child executor currently lacks Beaver's standard export/verification tools.
The child reported that boundary accurately; the operator completed those actions
through the public API. Fully unattended in-task standard export is not established
by this test. Game quality and fun remain outside this acceptance scope.

## Final packaged executable

The final two small UI guards were added after the full-workflow executable was
compiled. The release rebuild passed in 4m 25s. The core lifecycle, API, icon,
and export code are unchanged from the full round.

- Release: `release/Beaver-native-0.1.19-preview-r13-win32-x64/Beaver.exe`.
- SHA-256: `091F5ACF2EFFB0AFB962A4CC19BF8CBFFB17CAEF7A872C176915B7F318A03B07`.
- Verified package inventory: 606 files, 16,094,805 bytes; previous releases preserved.
- Actual packaged executable launched as PID 28024 after closing build-directory
  PID 25624. A process-path check found only this selected Beaver executable.
- Its fresh blank-project proof is
  `output/validation/fresh-round-20260910-020105-1a69200399274dde9580e8bb2ae350d2/start-proof.json`.
- The original input under fully automatic decisions produced five automatically
  answered questions and eight child tasks. This differs from the four steps in
  the human-selected one-minute full round, confirming the plan is not fixed.
- The supplementary UI round was intentionally paused after plan expansion. Its
  eight children remain interrupted and there are zero active or queued tasks;
  it is not represented as another completed game.
- A preexisting unsent draft remained visible when the parent entered
  `waitingChildren`; both editor and submit action were disabled. A deferred child
  displayed a disabled work-copy action with its dependency explanation.
- The final release rendered all eight cards in one lane: client height 314 px,
  scroll height 1136 px, mouse-wheel scrollTop 450 px, and no column pagination.
- Final runtime icon handles again measured 256 x 256 px, with both brand colors.

Final evidence: `output/validation/r13-release-proof.json` and
`output/validation/r13-final-icon-proof.json`. Screenshots:
`r13-final-draft-guard.png`, `r13-final-workspace-guard.png`, and
`r13-final-lanes.png`, all under `output/validation/`.

The preview release manifest retains its broader `runtimeVerified: false` flag.
This report proves the explicitly listed native workflow/UI checks and exported
bundle integrity; it does not claim every product capability, game quality,
cross-platform support, or unattended in-task standard export is complete.
