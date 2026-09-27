# F4 managed Godot import

## Observable completion condition

When the host enables Godot and a successful object fine attempt produces a
Godot project, Beaver imports that workspace with an owned headless Godot
process before capturing the output checkpoint. Interruption, timeout and normal
exit close the entire owned tree first. Failure remains visible in the existing
execution and recovery UI; restart does not replay the import or the model turn.
An import is not an F5 acceptance decision. Blender sessions and interactive
Godot preview remain outside this slice.

## Delivered behavior

The existing host `ExecutionSettings.mcp.godot` switch enables this optional
post-turn import. `tools.godot` selects the executable; an empty path retains
the existing tool discovery behavior. The model's isolated MCP configuration
stays disabled. Desktop's production launch factory already supplies these
settings, so object execution uses the same worker and existing execution and
recovery UI without adding another endpoint or toggle.

After a successful Codex turn and confirmed RPC tree shutdown, the worker
revalidates the attempt and runs `--headless --editor --import --quit --path`
against its own workspace. Only a root-level `project.godot` enables import;
asset-only work skips it and parent projects are never searched. The default
timeout is 120 seconds. Diagnostics are written to `godot-import.log` under
the attempt's Codex HOME.

Normal exit, failure, timeout and interruption close the owned process tree
before freezing output. A stopped importer failure retains partial output in
a Failed attempt. Spawn, attachment or shutdown errors do not authorize a
checkpoint: the attempt retains Running/recovery-required ownership. Successful
import ends at AwaitingGate. Existing explicit interrupt, recovery verification
and confirmed retry continue to operate on the same attempt lifecycle. Reopening
does not replay the importer or Codex.

## Shutdown regression and fix

The initial descendant-lock test failed with Windows error 33. A second probe
checked the writer's process handle immediately after close and proved it still
returned WAIT_TIMEOUT even though job ActiveProcesses was zero. The shared
`rpc_process_tree` shutdown boundary now prevents new descendants with an active
process limit, retains current job member handles, terminates the job, and waits
for both signaled process handles and zero active members. Existing Codex-owned
trees use this stronger shutdown boundary too; unrelated applications are not
terminated.

The test independently asserts root and descendant termination, then allows a
bounded wait only for byte-range lock release. Windows explicitly documents that
OS unlock after process termination may be delayed by available resources:
[LockFileEx remarks](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex).
The failing pre-fix probe is retained in `output/f4-godot-diagnostic.log`.

## Fresh focused verification, 2026-09-25

All commands ran in the same PowerShell/MSVC environment through `rtk proxy`.

- `cargo test --locked -p beaver-core object_attempt --lib`: 32 passed,
  1 explicitly ignored real-engine test, 0 failed. Includes four fake process
  entry points, normal/failure/timeout/interrupt descendant cleanup, asset skip,
  parent-project isolation, existing execution controls and scheduler behavior.
  Log: `output/f4-godot-core.log`.
- With `BEAVER_TEST_GODOT_EXE` set to
  `Z:\project\godot-4-4-1\bin\godot.windows.editor.x86_64.console.exe`,
  `cargo test --locked -p beaver-core worker_imports_real_godot_project_before_freezing_checkpoint --lib -- --ignored`:
  1 passed. Fake Codex creates a minimal project and SVG in a fresh fixture;
  the real Godot editor imports it. The resulting frozen checkpoint contains
  `icon.svg.import`, Codex output and descendant output, equals a fresh workspace
  capture, and retains AwaitingGate. Log: `output/f4-godot-real.log`.
- `cargo check --locked -p beaver-desktop`: passed.
  Log: `output/f4-godot-desktop-check.log`.
- `cargo fmt --all -- --check`: passed. Log: `output/f4-godot-fmt.log`.
- `npm run check:effective-lines`: passed, 966 sources, 17 unchanged legacy
  files, 0 violations. Import runner: 120 effective lines; process tree: 222;
  importer tests: 314, keeping the shared process fixtures and their lifecycle
  assertions together. No baseline update. Log: `output/f4-godot-structure.log`.
- `git diff --check`: passed. Log: `output/f4-godot-diff-check.log`.

## Remaining plan boundary

F4.4 stays unchecked. Interactive Godot preview and managed Blender session
recovery still need implementation. F5 acceptance and successor-fine advancement
remain open. This delivery covers bounded host-owned import, without offering
model engine access or reattachment to an old engine session.

The real-engine test is Core fixture integration evidence, not native product
acceptance. No Beaver EXE was launched or built, no real model was invoked, and
no visual/full native acceptance was run. No frontend files changed, so frontend
tests and typecheck were not repeated. Existing compiler warnings remain in the
logs. The dirty worktree was preserved; no commit, push or release was performed.
