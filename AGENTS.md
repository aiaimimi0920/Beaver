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

## Beaver test workflow

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
