# Beaver development

Read and apply [the code structure rule](resources/instructions/code-structure.md)
before editing source files. It also ships inside Beaver for Codex game tasks.
Aim for 100-250 effective lines per file. Keep 251-500 lines cohesive; split
501-700 unless a current exception documents the responsibility and protective
tests. New or modified files above 700 must be split; no exceptions above 700.
See [CODE-STRUCTURE.md](docs/CODE-STRUCTURE.md) for ownership boundaries,
the historical baseline and the exact checking commands.

Run `npm run check:effective-lines` with the relevant tests and formatter before
delivery. Do not update the baseline to accept a changed oversized file. Give
concurrent editors disjoint file ownership; keep integration under one owner.

## Vertical-slice-first development

Prioritize a complete, usable capability over isolated helpers or small safety
patches. For continuation requests, recover the current plan and worktree, then
choose the next bounded user workflow that closes an outstanding plan item or
removes its concrete blocker. State the observable completion condition before
implementation; do not expand the slice to unrelated features.

Implement the necessary Core behavior, persistence/recovery boundaries, Desktop
API, and production UI together when the workflow requires them. Reuse existing
modules. Internal helpers, eligibility flags, scaffolding, and additional test
evidence are intermediate steps; do not routinely stop and deliver after each
one while the selected workflow remains unfinished. A backend-only capability
is a valid slice when its actual consumer and acceptance condition are explicit.

Batch related edits before verification. During implementation, use only the
smallest compile, type, or focused regression check needed to resolve a concrete
uncertainty. Do not repeat Core tests, Desktop tests, formatting, and structural
checks after every helper, assertion, comment, or documentation edit.

At the completion boundary, run one focused verification batch for the changed
behavior and its direct dependencies, with the relevant compiler/type checker,
formatter, and effective-line check. Add meaningful regression coverage for
behavioral risks; avoid tests that merely duplicate implementation conditions.
After success, rerun only checks invalidated by subsequent changes. Expand scope
only for a failure or a specific unresolved integration risk. Documentation-only
changes need document validation, not application builds or functional tests.

Update the plan and evidence once per completed slice. Report the usable outcome,
checks actually run, and remaining blockers. Mark a plan item complete only when
its stated conditions are satisfied. If blocked, preserve the work and identify
the exact missing dependency; do not substitute more helpers or repeated tests
for closing the workflow. A user-requested checkpoint may report partial work.

## Beaver test workflow

During development, run functional tests for the changed behavior and its direct
dependencies, plus the relevant compiler, formatter and effective-line checks.
Development issues are expected; do not run full end-to-end acceptance after
every change. Run full acceptance only for a formal external release or an
explicit user request. Expand testing only when a failure or unresolved risk
requires it. After focused checks pass, stop testing and deliver the change.

The following native acceptance rules apply when that acceptance is in scope:

Before launching a compiled Beaver.exe for testing, close all older Beaver.exe
instances and verify that only the selected executable remains running. Interrupt
active tasks through their API when the connection is available, then close the
host. Do not terminate Godot, Blender or unrelated applications by name.

Use `scripts/start-fresh-native-test.ps1` for native acceptance rounds. It closes
old Beaver processes before its optional build, allocates unique data and WebView
directories, verifies empty state, creates a blank project through Beaver API,
and writes `start-proof.json`. Pass `-Build` when compiling a new test executable.

Each major acceptance round must start with a new blank Godot project. Never
reuse a previous round's generated code, assets, documents or task state. Keep
prior rounds as evidence. Continued interactions and recovery within one round
may use that round's project.

Game-creation acceptance must call Beaver APIs only. Do not directly invoke
Codex, Godot or Blender, or author game code, assets or game design documents to
help the application pass. Use short human-like input. Record minor issues and
continue using ordinary application recovery. The primary goal is interaction
and workflow completion, not gameplay quality.

## Native build environment

On Windows, run native Cargo builds and tests from the same PowerShell toolchain
environment. Some tool subprocesses omit `LIB`; alternating those processes with
the normal shell invalidates Cargo's SQLite and Tauri dependency fingerprints.
Preserve the existing MSVC environment rather than clearing its library paths.
Capture build logs in the normal shell and use context-mode to analyze the saved
logs. For an unchanged-build measurement, repeat `npm run build:native` in the
same environment and check the executable's hash and modification time.

## Product versions

Internal development versions use `X.X.X.N`; external release versions use
`X.X.X`. `package.json` stores the public base in `version` and the internal
iteration in `beaverBuild` (starting at 1). Increment only `beaverBuild` for
internal deliveries. Do not increment the public patch for each development
iteration or change versions merely because a check/build ran.

Keep npm, Cargo and Tauri package metadata on the same three-part SemVer base.
When deliberately changing that base for a release, synchronize its manifests
and lock files and reset `beaverBuild` to 1. Builds default to the development
channel; `BEAVER_CHANNEL=release` explicitly selects the public product version.
Build metadata, executable version strings and package manifests must agree.
See [versioning commands](docs/VERSIONING.md). Existing release records remain
immutable. Selecting a channel does not itself run acceptance or publish.
