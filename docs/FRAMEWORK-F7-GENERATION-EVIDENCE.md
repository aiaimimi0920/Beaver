# F7.5 generation object workflow evidence

Date: 2026-09-25

## Delivered workflow

The production object workspace opens a project-bound generation dialog. Name,
category, prompt and acceptance text are persisted locally per target project.
Closing and reopening restores the form without invoking the model, preparing
tasks or submitting a request. Storage read failures are shown and prevent
submission; write failures preserve the in-memory form and prevent remote writes
until the frozen request can be saved.

Explicit confirmation creates a new object, one medium task, one fine child
with its own stage identity, and the associated planned run through the existing
Desktop objectTask.getDraft/saveDraft/commit APIs and Core transaction. The
dialog does not claim that files have been generated or accepted. Users can
schedule the medium task from the existing project task queue panel.

The request identity and plan are frozen before asynchronous preparation. A lost
save response is recovered by reading and comparing the exact server draft; a
lost commit response replays the same request and expected revisions. The Core
transaction provides durable receipt replay and atomic object/run/task creation.
No new backend endpoint, compatibility layer or persistence format is needed.

The validated receipt remains in the local draft, exposes an Open Object action,
and permits explicitly starting another object. Navigation clears catalog
filters, reloads the catalog, and selects the returned object. If the active
project changed while the dialog was open, navigation asks the user to switch
back instead of selecting an object in the wrong project.

Every generated object uses a fresh identity; no existing object is selected by
name or overwritten, and this workflow never writes or deletes source files.
The obsolete mock generation action was removed from the demo toolbar.

## Verification

- `npx tsx --test tests/object-generation.test.ts tests/object-framework.test.ts`:
  11 passed. Covers project isolation, restored fields, lost save and commit
  responses, stable replay, duplicate submit, storage failure, wrong receipt,
  close during snapshot preparation, and honest dialog wording.
- `cargo test -p beaver-core --lib object_tasks::tests::commit -j 1`:
  4 passed. Existing transaction tests cover hierarchy/replay, rollback across
  objects/runs/tasks/revisions/receipt, explicit position ordering and reopen.
- `npm run typecheck`: passed after correcting optional array element narrowing.
- Focused Prettier check: passed.
- `npm run check:effective-lines`: 1036 sources, 0 violations; baseline unchanged.
- Focused `git diff --check`: passed.

Logs: `output/generation-ui-final.log`, `output/generation-core-final.log`,
`output/generation-typecheck-final.log`, `output/generation-format.log`,
`output/generation-structure.log`.

The initial Core filter `object_task_commit` selected zero tests and is not
counted. The corrected module filter above ran four tests. Existing unused-mut
warnings are rendered as NativeCommandError by PowerShell; the Rust test harness
reports all four passing. No unrelated warning cleanup was performed.

## Boundaries

This completes F7.5 together with the already delivered import confirmation,
draft/history recovery, source protection and result-navigation workflows.
Generation here means creating the planned production task workflow. Actual
model execution, resource quality and final acceptance remain separate actions.

An optimistic plan-revision conflict retains the frozen request and server
preparation and reports an error. It does not automatically rebase a confirmed
plan or silently create a replacement request. Cross-device local-draft sync,
automatic business-conflict resolution and recovery after manually clearing
browser storage are outside this slice. No native GUI or release acceptance was
run; the result navigation has source/type coverage, not a real WebView click
acceptance result. F7.1 and F7.3 remain outstanding in the implementation plan.
