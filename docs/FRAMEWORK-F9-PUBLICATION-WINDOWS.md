# Publication intent coordination across windows

Date: 2026-09-26

## Delivered workflow

Two windows already viewing the same project, task, run and object can no longer
silently overwrite or clear each other's saved publication intent. Before saving
an original publish or abort request, the production session compares its last
observed serialized record with current storage inside an exclusive Web Lock.
A stale window preserves the stored record, sends no new API request, and shows
the existing recovery action, `重试读取原发布请求`.

That action reloads the original request and queries publication status. It does
not submit. An explicit retry then preserves the original request ID, approval,
review and attempt. A late successful response cannot clear another window's
newer abort intent; read-only reconciliation can clear a confirmed terminal
operation after the current record has been adopted.

Each saved record, including a completed-request tombstone, receives a UUID
revision. This prevents an empty/request/empty cycle from making an old empty
snapshot appear current. Existing revision-less records remain readable and
receive a revision on their next write. No backend schema or API changes are
required. Cancellation while a lock is queued can retain recovery intent, but
the cancelled session does not dispatch the API when the lock becomes available.

## Verification

The focused batch passed 21 frontend tests, including five new regressions for
simultaneous stale submission and explicit recovery, late-response clearing
against abort intent, tombstone protection, revision-less recovery, and queued
lock cancellation / unavailable locks. Adjacent original-intent, publication and
approval-draft tests passed in the same batch.

```powershell
rtk proxy npx tsx --test tests/object-publication-windows.test.ts tests/object-publication-intent.test.ts tests/object-publication.test.ts tests/object-publication-approval-draft.test.ts
rtk proxy npx tsc --noEmit
rtk proxy npm run check:effective-lines
rtk proxy git diff --check
```

TypeScript compilation, Prettier and diff checks passed. Effective-line checking
passed with 1199 sources, 17 unchanged legacy files and zero violations. Logs are
in `output/f9-publication-windows/`. No baseline was relaxed.

A real Chromium session loaded the production `ObjectPublicationPanel` in two
same-origin tabs, using real localStorage and Web Locks with a simulated API.
Both tabs opened before the first submission. The first tab made one API call
and simulated a lost response. The stale tab displayed the conflict without an
API call; rereading remained read-only; explicit retry made one call identical
to the first tab's complete request. The script's assertions passed and saved
screenshots at both conflict and retry checkpoints. The only browser console
error was a missing harness favicon.

Browser harness, assertion script, CLI execution log and screenshots:
`output/playwright/f9-publication-windows/`. The isolated browser session and
harness server were closed after verification. This is real browser storage and
UI evidence with a simulated publication API, not native publication acceptance.

## Scope and remaining work

The lock protects only storage comparison and mutation; it does not serialize
whole server operations. Coordination requires the same origin, storage partition
and current implementation. Different devices, isolated WebView data directories,
and mixed-version running clients are outside this guarantee. Older clients may
reject the added revision field. Browser environments without Web Locks fail
closed before sending a request and show a storage error; native WebView support
has not been verified in this batch.

Final-candidate feedback re-localization, the remaining F9 navigation and delivery
conditions, and native EXE acceptance remain open. F9.2 and F9 stay unchecked.
