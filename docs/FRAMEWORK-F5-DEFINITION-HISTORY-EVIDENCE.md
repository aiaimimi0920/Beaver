# F5 frozen attempt definition history

Date: 2026-09-25

## Delivered workflow

Open a medium task's execution records, then expand an attempt's frozen task
definition. The production dialog displays the saved title, prompt, acceptance
requirements, definition revision and attempt ID. Failed, interrupted, awaiting
gate and recovery-required records support the same read-only view. Reopening
the project reads the persisted records without starting work.

The completion condition for this slice is satisfied: Core query, the existing
Desktop `objectTask.attempts` route and production UI expose the definition that
belongs to each attempt, including retained retry/cancellation history. The UI
does not substitute the current task title or prompt for historical content.

## Implementation boundaries

- `native/core/src/object_attempt_view.rs`: `list_with_definitions` reads
  `Attempt.fine` and the validated execution view under the same Store lock.
  The compact `list` projection remains available to existing control callers.
- `native/core/src/scheduler_object_control.rs`: query-only `Execution.definition`
  contains `title`, `prompt`, `acceptance`, and `revision`. No persisted schema,
  interruption receipt, retry receipt or ownership transition is changed.
- `src/shared/object-attempts.ts`: strict query contract validates the definition.
- `src/ui/object-tasks/ObjectTaskExecutionDialog.tsx`: per-attempt expansion uses
  plain React text, preserves line breaks and wraps long text. Empty acceptance
  requirements are explicitly identified. No edit, execution or approval action
  is introduced by opening the definition.
- `src/ui/object-tasks/object-task-execution.ts`: interruption acknowledgement
  updates execution state while retaining the already queried frozen definition.

The displayed prompt is the frozen task prompt. It is not a reconstruction of
the complete Codex RPC request, system instructions, tool trace or credentials.
The definition revision belongs to the frozen fine task; the existing execution
`taskRevision` remains the medium task's control revision.

## Fresh verification

- `cargo test --locked -p beaver-core object_attempt --lib`: 32 passed,
  1 ignored, 0 failed. Includes project reopen with a deliberately changed live
  task prompt: the query still returns the saved attempt definition and does not
  launch a worker. Log: `output/f5-definition-core.log`.
- `cargo test --locked -p beaver-core retries_chain_checkpoints_and_keep_immutable_history_after_disposition --lib`:
  1 passed. Queries all three attempts after retries and cancellation, comparing
  each definition with its own saved record. Log: `output/f5-definition-retry.log`.
  An initial filename-based filter selected zero tests; the named test above was
  then run successfully. The zero-test invocation is not counted as evidence.
- `npx tsx --test tests/object-task-execution*.test.ts`: 21 passed. Covers
  independent historical definitions, escaping script-like prompt text,
  read-only reopening, query validation and definition retention after an
  interruption receipt. Log: `output/f5-definition-ui.log`.
- `npx tsx --test tests/object-task-resume-execution.test.ts`: 1 passed.
  Log: `output/f5-definition-resume-ui.log`.
- `npm run typecheck`: passed after updating the retry test fixture to preserve
  the required query field. Log: `output/f5-definition-types.log`.
- `cargo check --locked -p beaver-desktop`: passed with existing warnings.
  Log: `output/f5-definition-desktop.log`.
- `cargo fmt --all -- --check` and Prettier check of the seven affected
  TypeScript files: passed. Logs: `output/f5-definition-fmt.log` and
  `output/f5-definition-prettier.log`.
- `npm run check:effective-lines`: 966 sources, 17 unchanged legacy files,
  0 violations. No baseline changes. Log: `output/f5-definition-structure.log`.

A separate read-only scout reviewed query ownership, retained history, receipt
compatibility and text rendering and found no concrete defect. This review is
supplemental to the checks above.

## Remaining plan scope

F5.2 and F5.4 remain unchecked. Auto-acceptance prompts, output-format settings,
requirement provenance, loading historical prompts for a new edit, stage rework
and downstream invalidation are not delivered here. The preview manufacturing
workspace remains separate from this real execution-record workflow.

F5 gates and subsequent fine dispatch remain unimplemented; a successful attempt
still waits at `AwaitingGate`. F4 interactive Godot/Blender recovery and F6-F9
remain open. No new model call, native EXE build, browser visual acceptance,
full native acceptance, commit, push or release was performed for this slice.
