# F6 frozen attempt technical checks

Date: 2026-09-25. Development slice; no native acceptance or release claim.

## Usable workflow

Open an object task execution history and select a terminal attempt with frozen
output. The technical-check panel loads saved reports and offers a new check,
an exact-request retry after an ambiguous failure, and history refresh. Checks
do not invoke a model or regenerate output. Reopening the view reads the
project-persisted reports.

- `objectTask.checkAttempt`: `projectId`, `requestId`, and the complete frozen
  attempt `target`; returns the immutable report.
- `objectTask.attemptChecks`: `projectId`, `attemptId`; returns saved reports.
- Reports include the request, attempt/input/output digests, timestamp, runner
  version, ordered rule versions, checked-file counts, issues and result.

Core stores `object_attempt_check_report` entities in the project store. Desktop
routes through the project storage router and runs checks outside the async
executor using `spawn_blocking`; no host-store fallback is allowed. The shared
TypeScript schema validates report shape, versions, identities and aggregate
results before the controller presents them.

## Evidence and ownership boundaries

The built-in rules are `checkpoint-integrity` and `code-structure`, version 1.
Integrity validates baseline/input/output manifests and blob content, rejecting
invalid paths, duplicate case-insensitive paths, missing blobs, links and corrupt
content. Structure checks compare frozen output against the original run
baseline. A retry input containing oversized failed output cannot make that
output an unchanged historical exception.

Checks preserve snapshots, task state and queue ownership. They never approve a
stage, launch its successor, publish an object or release its writer claim.
Running attempts and incomplete or mismatched targets are rejected. Historical
attempts remain checkable after cancellation, workspace disposal and reopening
the project, while retained blobs are available.

An exact request replay returns the historical point-in-time report even if a
blob later changes. A new request performs new checks and records a new result.
Thus a historical success is not proof of current blob integrity, current
workspace state or acceptance eligibility. UI cancellation only stops waiting;
it does not claim to cancel an already dispatched Core check. Late responses
cannot replace the active view state.

## Focused verification

- Core check regressions: the two tests in `object_attempt_check_tests.rs` passed
  in `cargo test --locked -p beaver-core --lib object_attempt_check`. The added
  history test initially failed because its assertion resolved an intentionally
  deleted workspace. The test now captures its path before deletion.
- Final targeted rerun:
  `cargo test --locked -p beaver-core --lib object_attempt_check_history`: 1 passed,
  0 failed. It verifies original-baseline enforcement after retry, disposal,
  reopen, historical replay and a fresh check without the workspace. Logs:
  `output/attempt-check-history-tests.log` and
  `output/attempt-check-history-final.log`.
- Desktop: `cargo test --locked -p beaver-desktop object_attempt_runtime::tests`:
  3 passed. Includes real launch-failure frozen output, replay/list, strict target
  validation, project isolation and host-store exclusion. Log:
  `output/attempt-check-desktop-tests.log`.
- UI: `npx tsx --test tests/object-attempt-checks.test.ts tests/object-task-execution*.test.ts`:
  30 passed. After adding synchronous cancellation protection, the changed check
  controller suite alone was rerun: 5 passed. Logs:
  `output/attempt-check-ui-tests.log`, `output/attempt-check-ui-final.log`.
- TypeScript typecheck passed, exit 0: `output/attempt-check-typecheck.log`.
- Targeted Rustfmt and Prettier checks passed; `git diff --check` passed.
- Final `npm run check:effective-lines`: exit 0, 979 files, 0 violations,
  17 unchanged legacy files, 43 immutable sources and 1 existing exception.
  No baseline was changed. Report: `output/effective-code-lines.json`; log:
  `output/attempt-check-lines-final.log`.

New production modules remain small: Core report service 157 effective lines,
check engine 110, shared schema 45, UI controller 127 and panel 79. Persistence,
file inspection, contract validation and presentation have separate ownership.

## Remaining plan work

F6.3 and F6.4 remain incomplete: this slice supplies built-in checks only, not
registered/configurable checker adapters or general evidence-tool integration.
F6.5 still needs shared manufacturing/test-page report associations. F6.6 still
needs AI evaluation, owner decisions, stage policies and fresh applicability
validation before approval. F5 stage approval and successor execution require
their own recovery-aware workflow. None of these items is implied by a passing
technical report. No executable build, full native acceptance, external release,
commit, push or version change was performed for this slice.
