# Executable creation plans

General creation tasks default to planning first. The task composer exposes a
checkbox to choose this workflow; review tasks remain single units. API callers
can set `task.create.decompose` explicitly. Existing persisted tasks without the
field retain their previous behavior.

The planner asks the user through `beaver_ask_user`, including suggested choices
and custom answers. The existing global/task ask ratio controls those decisions.
After clarification, the model submits `beaver_submit_plan` with a summary and
2-12 ordered steps. Each step contains a unique title, direction, execution goal,
and testable acceptance criteria. Steps are model-generated from this goal, not
fixed department names. A plain text plan or a completed planner turn without a
valid submitted plan cannot mark the goal complete.

After the planner process stops, its confirmed records pass through the existing
snapshot/conflict/journal merge. Only a successful merge creates the complete
child list atomically. The parent enters `waitingChildren`. Children appear as
individual task cards, with independent conversation, changes, continuation,
approval, and rollback boundaries. Ordinary delegated tasks still work.

Children execute sequentially. A child can run only after the preceding child is
completed and approved. Its first workspace snapshot is taken at execution time
from the latest integrated project, so it sees its predecessors' files. A retry
of an already-started child retains that child's original workspace and baseline.
For new tasks (`validationVersion: 1`), the parent queues an integration-only
GUT run after every planned child is completed and approved. It completes only
when that integrated candidate passes, the children remain approved, and the
normal safe-merge checks hold. Persisted legacy tasks retain their prior policy.
No visual-quality or export-success claim follows from this status alone.

## Approval and recovery

`task.create.autoAccept` defaults to true for new ordinary tasks, including plan
parents and their children. Tasks with runtime changes must pass GUT on the
candidate that will actually merge. Successful completion means execution has
stopped, required code checks passed, and changes merged without conflict;
automatic approval then records `approvalSource: automatic`. Failed, interrupted,
questioning, and conflicting tasks do not receive automatic approval. An explicit
manual policy remains available for both parents and children.

Use `task.approval` with `{ id, autoAccept: false }` to select manual approval.
For a parent this updates unfinished planned children; a child's setting can be
changed separately. A completed unapproved child is accepted explicitly through
`task.accept`, which records `approvalSource: user` and releases its successor.
Changing a policy does not rewrite prior approval history or silently accept a
completed task. The UI exposes the same operations.

Code failures retain the frozen run and failure context. Automatic repair is
bounded to two attempts and stops on repeated failure fingerprints or unavailable
engines. User instructions arriving during validation are retained and applied
before a fresh candidate is tested. Concurrent project edits also require a fresh
code receipt; a passing old workspace cannot approve a different merged tree.

Delivery records durable visual coverage work for the project validation queue.
Screenshot/video capture, comparison, and human review proceed independently and
never hold ordinary task completion or approval. Feedback on a completed task
creates related work with its own rollback boundary. See
[game validation implementation](GAME-VALIDATION-IMPLEMENTATION.md).

Interrupting a parent pauses its plan and interrupts its children. Continuing the
parent with empty text resumes interrupted children; questions and failures still
require their normal recovery actions. Scope amendments are sent to the concrete
child task. Application restart pauses plans and interrupts queued/running work;
it does not silently resume model spending. Durable child IDs prevent duplicate
creation after recovery.

Native R19 also exposes `task.retryMerge` with `{ id }` for an already-finished
task blocked by file conflicts. It retries the recorded output without launching
AI or recapturing edited workspace files. Content already identical in the project
is excluded from the task's changes and rollback ownership. Genuine differences
still block the whole merge; there is no force-overwrite option. Review violations,
unfinished execution, pending questions and incomplete journal recovery cannot
use this operation. Successful integration retains the task ID, report, original
baseline and child approval policy.

## Presentation

Each swimlane renders all matching task cards in a vertically scrollable container.
There are no per-column page counters or left/right page buttons. The child task
list also scrolls, and parent cards show the number of child tasks.

The ICO retains 16, 20, 24, 32, 48, 64, 128, and 256 pixel frames. Its first frame
is 256 pixels because the installed Tauri code generator uses the first ICO entry
as the runtime window and tray image. Native frontend builds regenerate this asset
before Rust compilation. Windows PE resources continue to use Tauri's existing
resource build; there is no duplicate icon resource injection.
