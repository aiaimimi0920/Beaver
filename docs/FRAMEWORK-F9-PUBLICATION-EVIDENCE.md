# Publication feedback evidence integrity

Date: 2026-09-26

## Delivered workflow

Publication now revalidates archived numbered-frame feedback before producing a
preview, starting a publication journal, applying pending writes, and committing
acceptance. It resolves the feedback's original attempt, verifies the same run
and object, and uses the existing immutable-frame and historical correspondence
resolvers. Missing archives, damaged PNG/frame records, and mismatched sources
block publication with `OBJECT_PUBLICATION_FEEDBACK_EVIDENCE`, the feedback
request ID, and the underlying failure.

Restoring the original evidence allows a fresh preview or an exact retry with
the original publication request. No replacement frame or new request is created
automatically. A failure after the journal starts leaves an `applying` operation
with a durable error; listing and aborting it remain available after reopening
the project. Completed publication replay remains readable even if an archive
later disappears.

The existing Desktop publication APIs and production `ObjectPublicationPanel`
consume this Core behavior without schema changes. The panel already renders
preview errors and operation errors and offers refresh, retry and explicit abort.
The live evidence check belongs to `prepare::load`; frozen journal integrity
continues to use `prepare::build` without requiring live archive availability.

## Verification

Before the fix, regression tests reproduced both an accepted preview with damaged
evidence and publication completion with a missing source archive. The final
focused batch passed **13 Core tests**, including two new workflow regressions
and 11 adjacent deferred-feedback, frame and relocation tests:

```powershell
rtk proxy cargo test --locked -p beaver-core object_publication_deferred -- --nocapture
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
rtk proxy git diff --check
```

The new regressions cover corruption, foreign source metadata and missing records
for both historical and target frames; rejection before journal creation with
unchanged task state; restored evidence and project reopen; archive loss at the
pre-commit checkpoint; pending retry after reopen; abort while evidence is still
missing; restored exact-request publication; and terminal replay after later loss.

Rust compilation and formatting, effective-line and diff checks passed. Logs:
`output/f9-publication-evidence/{red,core,fmt,structure,diff}.log`. The initial red
batch intentionally failed; only `core.log` records the final passing batch.
Production UI error and refresh paths were inspected; no frontend source changed.
This batch did not run browser, Desktop compilation, native EXE, real engine or
external model acceptance. An attempted independent read-only review failed due
to the agent provider returning HTTP 503; it supplies no review evidence.

## Remaining plan boundary

This checks integrity of the original feedback evidence and its saved historical
correspondence. It does not re-localize old regions onto the final candidate or
require a new final-candidate mapping. Standalone PNG and published-version
correspondence, final publication re-localization, cross-window coordination and
the remaining release conditions stay open. F8.5, F9.2 and F9 remain unchecked.
