# F8: Historical PNG delivery to publication follow-up tasks

Date: 2026-09-26

## Usable outcome

The existing candidate feedback UI can defer PNG region feedback to a later
medium task. After publication, explicitly enqueueing that task now lets its
model inspect the original frozen feedback PNG, even when the published PNG
has changed since the feedback was recorded. The task still ends at the normal
human acceptance gate; this does not automatically accept or publish its output.

The actual consumer is the existing object-attempt worker and its model RPC
callback. No new Desktop API or UI action is required for this slice.

## Binding and delivery

- Resolve the current fine task's immutable follow-up receipt and validate its
  project, run, medium task and object against the executing attempt.
- Validate the referenced published operation, version and source target. Match
  its original deferred feedback using the persisted deterministic request ID.
  Existing receipts work without a schema migration or rewriting history.
- Select the feedback's original attempt output checkpoint, not the publication
  attempt or the new task's input. Preserve the original path, SHA-256,
  dimensions, numbered regions and prompts.
- Include `feedbackImage` provenance in the initial frozen model context and
  subsequent context queries. The read-only `feedbackImage` operation takes no
  caller-selected source ID, path or hash; unrelated tasks have no attachment.
- Deliver the original PNG as `inputImage`, with source and region metadata in
  text items. Reuse checkpoint membership, blob hash, inline size and bounded
  PNG decode checks. Verify the live callback identity and cancellation both
  before and after reading the image.
- Execution instructions require inspecting the historical attachment and
  re-locating its regions against current content before editing. The attachment
  does not assert engine hits or authorize arbitrary historical file reads.

## Verification

All commands ran from the repository root in the normal PowerShell native
toolchain environment, through `rtk proxy`.

| Check                                                                  | Result                                                 |
| ---------------------------------------------------------------------- | ------------------------------------------------------ |
| `cargo test --locked -p beaver-core --lib publication_deferred`        | 5 passed, 0 failed                                     |
| `cargo test --locked -p beaver-core --lib object_attempt_callback`     | 4 passed, 0 failed                                     |
| `cargo test --locked -p beaver-core --lib object_candidate_rework`     | 8 passed, 0 failed                                     |
| `cargo test --locked -p beaver-core --lib object_publication_followup` | 2 passed, 0 failed                                     |
| `cargo test --locked -p beaver-core --lib object_attempt_launch`       | 1 passed, 0 failed                                     |
| Targeted Rust formatting                                               | Passed                                                 |
| `npm run check:effective-lines`                                        | 1,149 sources, 17 unchanged legacy files, 0 violations |
| Prettier on the plan and this evidence                                 | Passed after formatting                                |
| `git diff --check`                                                     | Passed                                                 |

The new full RPC regression creates red PNG feedback, reworks the candidate to
a green PNG, publishes, closes and reopens the project, then executes the planned
follow-up through the normal worker. A deterministic app-server subprocess
changes the live workspace PNG and requests the feedback attachment. Assertions
verify exact original red PNG bytes, historical output attempt identity, original
regions, pinned publication version, stopped child processes and the normal
`awaitingAcceptance` result. Additional regressions reject source redirection,
wrong RPC identity, corrupted blobs, interrupted calls and unrelated-task access.

Logs: `output/feedback-image-{core,callback,rework,followup,launch,rustfmt,structure}.log`.

## Remaining scope

This proves Beaver's real worker/RPC image transport against a deterministic
process fixture. It does not prove external model perception or edit quality.
No external provider call, native UI acceptance or full release acceptance ran.
The existing 1 MiB inline image limit remains in force; oversized attachments
return a textual refusal. Archive-frame feedback submission, real engine hits,
Blender preview and the broader F8/F9 work remain open.
