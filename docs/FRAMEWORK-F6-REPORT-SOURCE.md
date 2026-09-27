# F6: traceable object report sources

## Delivered workflow

The production test page now reads `validation.objectReport.get` with the exact
`projectId`, `attemptId` and `requestId` selected in manufacturing. It displays
the original technical report, its frozen stage and the persisted candidate
reviews that explicitly reference that check request. Manufacturing retains its
existing report reader and renderer; both surfaces use the same stored report.

The response is `{ source, report }`. `source` contains `kind: objectAttempt`,
the project and full object/run/medium/fine/attempt target, `stageId`, and
`candidateReviews` with request IDs and source/output digests. These are actual
review records, not synthetic candidate IDs or inferred approval states.

Core verifies persisted check and candidate records, obtains the stage from the
frozen attempt, and matches candidate check request, target and output digest.
Desktop selects the open project-local runtime before acquiring any legacy
validation store lock. The method is a read-only query and does not wake the
scheduler. No check, model call, acceptance or publication occurs on reads.

The UI validates the response envelope and exact requested identity, clears
evidence on errors, and rejects late responses after cancellation. Missing
reports are not replaced with a newer report. Empty candidate references are
shown explicitly. Game-level testing remains available with separate approval
scope; historical technical success is not current workspace validity.

## Focused verification (2026-09-25)

Successful commands, all exit code 0:

```text
npx prettier --write <the 4 changed/new TypeScript files>
cargo fmt --all
npm run typecheck
npx tsx --test tests/object-check-report.test.ts tests/object-attempt-checks.test.ts tests/object-task-execution-ui.test.ts
cargo test --locked -p beaver-core --lib validation_object_report -- --nocapture
cargo test --locked -p beaver-desktop object_attempt_runtime -- --nocapture
npm run check:effective-lines
git diff --check
```

- Frontend: 14 passed. Covers exact selection, frozen source rendering, invalid
  identities, missing stage, duplicate candidate references, mismatched output
  digests and clearing evidence on cancellation or errors.
- Core: 1 passed. Creates two check reports and candidate reviews, reopens the
  project store, proves only the explicitly matching review is returned and
  verifies task state, queue and report count remain unchanged.
- Desktop: 12 passed. Includes catalog/query classification, actual project-local
  report lookup, and rejected foreign, closed, legacy or missing project routes.
- Effective lines: 1088 sources, 17 unchanged legacy files, 0 violations.

Logs: `output/f6-source-{format,types,tests,core-lib,desktop,lines}.log`.
The initial Core command omitted `--lib` and failed while compiling unrelated
integration targets with metadata-stub/missing-rlib errors and compiler ICEs.
That failed log is retained at `output/f6-source-core.log`; the corrected focused
library command passed. This does not establish a passing full integration suite.

## Remaining scope

F6.5's source protocol and production UI are implemented. F6.5 stays unchecked
pending native interaction verification of project switching, exact report
navigation and return to manufacturing. No native application acceptance or
release was performed. Configurable evidence policies, registered checkers and
AI evaluation remain separate unfinished plan items.

Previous navigation evidence: [report navigation](FRAMEWORK-F6-REPORT-NAVIGATION.md).
