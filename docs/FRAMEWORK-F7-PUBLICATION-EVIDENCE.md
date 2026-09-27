# F7 candidate acceptance and publication

Date: 2026-09-25. Development verification only; no external release or native GUI acceptance was performed.

## Usable workflow

The production execution dialog exposes final acceptance under each frozen candidate review. The owner reviews the complete owned-file registration and SHA-256 changes relative to the current accepted head, confirms replacement when the candidate originated from a historical or empty baseline, records an acceptance note, and explicitly resolves or waives every recorded rework feedback with a note.

Core freezes content and persists a file journal before materialization. The final SQLite transaction updates the accepted version, final fine task, medium task, run, queue and plan revision together. It does not assume filesystem and SQLite commits are atomic. The coarse parent remains subject to its own integration conditions. The next eligible task claims the newly accepted baseline.

Pending operations keep ownership. Reopening queries saved operations without publishing or starting model work. Exact-request retry and explicit abort are available. Abort reverses only attributable writes still matching the candidate and retains external modifications and frozen output. An uncertain abort remains an abort after refreshing an applying journal. Completed publication refreshes execution and workspace state, including when completion is recovered by query after a lost response.

Frozen candidate reports and attempts remain historical evidence; their capture-time blockers and AwaitingGate state are not rewritten to represent current publication state. Current publication status is displayed separately.

## Implementation

- Core: native/core/src/object_publication*.rs, separated into preparation, files, journal storage, integrity, final transaction and retained history.
- Desktop: native/desktop/src/object_attempt_runtime.rs and object_task_catalog.rs; preview, publish, list and explicit abort routes. Consent schemas use boolean type plus a true-only enum supported by the business validator.
- UI: src/shared/object-publication.ts, src/ui/object-tasks/object-publication.ts and ObjectPublicationPanel.tsx; integrated with candidate reviews and execution history.

## Fresh verification

- cargo test --locked -p beaver-core --lib object_publication -j 1: 11 passed. Covers file-boundary reopen, one-time commit, next-task baseline, exact replay, corrupt journals, live/catalog drift, ownership/reference isolation, historical replacement consent, feedback disposition and abort preservation.
- cargo test --locked -p beaver-desktop publication_routes -j 1: 1 passed. Real scheduler fixture, read-only preview/list, foreign project rejection, publication, replay, retained history and abort consent/terminal rejection. The initial object_publication filter selected zero Desktop tests; publication_routes is the verified filter.
- Publication TypeScript tests: 5 passed. Lost-response exact retry, query-only reopen, uncertain abort, canceled late responses, foreign receipt rejection, explicit consent/feedback and completed-query refresh; production panel SSR checks.
- Candidate review tests: 4 passed alongside the initial publication tests.
- Rework and execution sibling tests: 28 total. Initial run had one obsolete UI text assertion; after correcting the frozen-history assertion, all 5 tests in that file passed. The other 23 passed without changes.
- npm run typecheck: passed.
- Targeted Prettier and rustfmt: applied to touched publication/integration files.
- npm run check:effective-lines: 1013 sources, 17 unchanged legacy files, 0 violations. Baseline unchanged.
- git diff --check: passed before documentation update; final document check recorded in output/f7-diff-check.log.

Logs: output/f7-core.log, f7-desktop.log, f7-ts.log, f7-adjacent.log, f7-ui-recheck.log, f7-publication-final.log, f7-typecheck.log and f7-lines.log. Existing dead-code and unused-mut warnings remain outside this slice.

## Remaining work

F7.2 is complete at the development verification boundary. F7.1 still includes broader configurable stage/report policies. F7.3 includes the wider managed-session cancellation workflow; this slice proves publication recovery and replacement consent only. F7.4 and F7.5 formal import commit/recovery and navigation remain open. No native executable was launched, no version was changed, and no commit or push was performed. Existing dirty-worktree changes were preserved.
