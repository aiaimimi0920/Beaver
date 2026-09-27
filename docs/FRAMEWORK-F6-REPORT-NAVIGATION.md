# F6: frozen attempt report navigation

This records the initial navigation slice. The subsequent
[traceable report source slice](FRAMEWORK-F6-REPORT-SOURCE.md) replaces the test
page reader with `validation.objectReport.get` and adds frozen stage and explicit
candidate-review references; the original verification below remains historical.

## Delivered workflow

From a technical report in the production manufacturing execution dialog,
choose `在测试页查看此报告`. The test page reads the same persisted report by
project, object, run, medium task, fine task, attempt and request identity.
`返回对应制造记录` reopens the matching task, highlights the attempt and expands
the selected report. `打开游戏测试工作台` retains access to existing game tests.

Both surfaces share `ObjectAttemptCheckReport`, including runner and rule
versions, input/output/attempt digests, checked-file counts and issues.
The test page calls only `objectTask.attemptChecks`; opening or rereading a
report does not rerun checks, invoke a model, accept a stage or publish an object.
The existing Core persistence and Desktop read API are reused unchanged.

## Identity and recovery boundaries

- The reader rejects foreign project/target identities and duplicate request IDs.
- A missing request is reported explicitly; the newest report is never substituted.
- Refresh errors clear displayed evidence, and cancellation discards late responses.
- Project changes clear navigation focus; return navigation checks the current
  task's object and run before opening its execution history.
- Missing tasks or attempts are reported rather than silently selecting another.

## Focused verification (2026-09-25)

All commands completed with exit code 0:

```text
npx prettier --write <the 9 changed/new TypeScript files>
npm run typecheck
npx tsx --test tests/object-check-report.test.ts tests/object-attempt-checks.test.ts tests/object-task-execution-ui.test.ts
npm run check:effective-lines
```

The focused suite passed 14 tests, including 4 new report-navigation tests:
historical report selection and read-only calls; missing, duplicate and foreign
identities; cancelled/failed reads; and manufacturing report expansion/link
rendering. The structure gate found 1085 sources, 17 unchanged legacy files and
0 violations. Its Rust dead-code warnings are not failures.

Logs: `output/f6-report-{format,types,tests,lines}.log`.
Independent read-only review found no concrete navigation defect.
These are controller and server-rendered UI checks, not browser/native interaction
acceptance. Actual project-switch callbacks and manufacturing return navigation
have not been exercised in a native application in this slice.

## Remaining scope

F6.5 remains unchecked: this slice connects manufacturing and testing to the same
attempt report, but does not add object/stage/candidate sources to the
`validation.*` protocol. Configurable evidence policies, registered checkers,
AI evaluation and full native integration acceptance remain separate work.
Historical technical success is not current workspace validity or approval.
