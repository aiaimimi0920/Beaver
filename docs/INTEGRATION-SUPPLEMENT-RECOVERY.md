# Recover an unintended integration supplement

The native `task.withdrawSupplement` operation accepts a child `id`, the expected
`parentId`, and a nonempty `reason`. It is intentionally narrower than general
cancellation: only an unaccepted, unexecuted, unchanged user supplement appended
after the original integration plan is eligible.

Both the native desktop and headless host validate the request in Core. The
recovery workbench and native task conversation expose the action. The Core
verifies the actual working copy against its baseline, recovery fences, lineage,
session history, original planned steps, and dependents before changing metadata.
It does not delete files, change baselines, mark a task accepted/completed, or
reuse a cancelled validation result. A withdrawal records the superseded
instruction and reason in the child and both audit histories. Retrying the same
request is idempotent.

The original child remains failed/interrupted and remains in `subtaskIds` for
historical inspection. Normal reconciliation excludes the explicit withdrawal
from active integration dependencies, keeps all other acceptance requirements,
and queues a fresh integration GUT run. A paused parent stays paused. Resuming
cannot resurrect a withdrawn supplement. Completion still requires the current
integrated snapshot and a successful matching GUT receipt.

The direct workbench's Continue button now sends an empty continuation instead
of implicitly resubmitting its goal textbox. An empty continuation of already
running work does not interrupt validation. Genuine future steering and repair
children inherit the parent's execution mode, but never its session/run IDs.

Verification scope: focused Core withdrawal and receipt-gate tests, headless
contract tests, direct-workbench interaction tests, native build, and the real
existing-project recovery. The build workflow retains a checksum-bound headless
artifact. CI success is not a zero-warning or final visual-quality claim.
