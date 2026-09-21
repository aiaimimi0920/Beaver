# Beaver task callbacks (protocol 5)

Beaver owns task lifecycle and asset stages. Use `beaver_task` to read the current
task, existing asset stages and durable callback receipts. Task identity is bound
by the host; do not supply another task ID, access Beaver's database or configure
the owner business MCP/API token in this managed session.

At the beginning of work and after recovery, call `state`. Read the actual
workspace and reconcile any prior receipts with the scene before repeating tool
operations. A callback receipt deduplicates the database mutation only; it does
not prove that an external Blender operation ran exactly once.
State includes only the latest 20 receipt summaries, truncated to 200 characters.
Use `receipt` with a request ID to read its complete stored input and output.

Use `report` at meaningful step boundaries. Supply a stable `requestId`, the
current callback `expectedRevision`, and `report` containing `kind`, `summary`,
`inputs`, `outputs` and optional `tools`. Record concrete paths, parameters,
references and observed tool/Skill/plugin versions. Keep payloads small; never
include credentials or inline large assets. For an asset stage include `stageId`
and its current `assetRevision`. Do not claim unobserved operations or versions.

For stepwise asset work requiring owner review, call `deliveryPlan` BEFORE creating
any stages. Supply `requestId`, callback `expectedRevision`, `assetRevision` and
`template: {id, version, stages: [{id, name}, ...]}`. Use 2 to 20 linear stages,
stable unique IDs and a positive template version. The template is immutable
within this task. Do not enable it on an already staged or planning task. Do not
invent a fixed NPR template: choose stages matching the confirmed goal.

In this opt-in workflow, only the first unapproved stage may execute. Read
`asset.delivery.approved` and the current asset revision from `state`. Use
`stages` or `beaver_asset_task` only to update the current stage's progress; keep
the complete stage list, identities, order and dependencies unchanged. Textual
completion cannot approve a stage. Each stage requires the owner's decision;
task auto-accept and ordinary continuation cannot bypass this gate.

Register meaningful asset steps with `work`. Every request includes `requestId`,
callback `expectedRevision`, `assetRevision`, current `stageId` and `change`.
These are serial records within this task and its shared workspace; do not use
`beaver_submit_plan` to create separate asset workspaces for these steps.

- `change: {action: "create", definition: {title, goal, acceptance}}` returns a
  host-assigned `subtaskId`. Register in execution order; no delete or reorder.
- `revise` takes `subtaskId` and a replacement `definition`; `cancel` takes
  `subtaskId` and `reason`. Running, completed and cancelled steps cannot be
  revised or cancelled. Prior receipts and attempt definitions remain available.
- `begin` takes `subtaskId`, an `inputs` object and required `inputFiles`, and
  returns `attemptId`. Use `inputFiles: []` to explicitly declare no file inputs;
  missing or null is rejected. Historical missing manifests remain uncollected.
  Only the first unfinished, noncancelled step may begin; only one attempt may
  run in the workspace. Retrying failed, interrupted or stale work also requires
  `recoveryNote` explaining the inspection of saved files and the actual scene.
- `finish` takes `attemptId`, `outcome` (`completed` or `failed`), `summary`, an
  `outputs` object and optional `tools` objects. It must use the same active
  thread / turn that began the attempt. Completion is a report, not acceptance.
- `reopen` takes a completed current-stage `subtaskId` and `reason`. No attempt
  may be running and no owner review may be pending. The selected and subsequent
  noncancelled steps in this stage become stale; their history is retained.
  Reexecute in order with fresh input capture and a `recoveryNote`.

Each input declaration is `{path, role, expectedSha256?}`. Use workspace-relative
paths with forward slashes; copy external files into the workspace first. Declare
at most 32 distinct paths, including ASCII-case duplicates in the uniqueness check.
Files must be nonempty, at most 256 MiB each and 512 MiB together. An optional
`expectedSha256` must be an actual lowercase SHA-256 and match the captured bytes.
Beaver freezes the bytes and stores host-computed hashes separately from `inputs`.
Use `role: "source"` for files this attempt may modify; only their frozen input
bytes are rechecked. Use `role: "dependency"` for files that must remain unchanged;
both their frozen bytes and current workspace hashes are checked. Roles and list
completeness are caller declarations. The optional workflow dependency scanner
finds stored Blender references; inspect its unresolved entries and explicitly
declare runtime-generated inputs as well. Scanning does not register inputFiles.

Successful `finish`, stage submission and owner approval verify the relevant
attempt inputs. If a dependency changes during running work, finish as `failed`,
inspect the scene and begin a new attempt. After completion but before submission,
use `work.reopen`. During pending review the owner must reject the candidate;
for an approved stage use the existing owner stage-reopen workflow. Failed finish
and owner rejection remain available after dependency drift.

Read state after each mutation. Beaver records execution identity, checkpoint
references and approved input candidate IDs. Report concrete IO and tool versions;
these reports do not prove external tools ran. Turn end, interruption or restart
retires unfinished attempts even without a Blender session. Inspect before
beginning a new attempt with a new request ID; do not finish an old attempt.
Pending owner review blocks new work. Reject/reopen marks affected subtasks stale
and preserves their prior attempts. Limits are 100 subtasks and 200 attempts per
task. Resolve meaningful steps without enumerating every small tool operation.

Save actual outputs before calling `submitDelivery`. Include `requestId`,
callback `expectedRevision`, `assetRevision`, current `stageId`, `inputCandidates`
equal to the entire ordered `asset.delivery.approved` list, `paths` and `summary`.
Submit 1 to 32 distinct workspace-relative file paths using forward slashes.
Files must be nonempty, at most 256 MiB each and 512 MiB together. Include saved
`.blend`, external dependencies, exports, previews and reports needed for this
delivery. A dependency scan does not submit files or establish modeling quality.
Use a report to explain the actual Skill, tool and plugin choices and versions.
All registered current-stage subtasks must be completed or explicitly cancelled.
Beaver binds their successful latest attempt IDs to the candidate and checks that
their approved input candidates still match. Owner approval checks inputs of the
candidate-bound attempts only. Multiple versions of the same dependency across
these attempts must all pass; a later version does not hide an earlier conflict.
Historical tasks without work records keep their existing submission behavior;
attempts without input manifests do not gain fabricated file evidence.

Submission freezes file copies and returns `candidateId`, `paused: true` and
`acceptance: "awaitingOwnerReview"`. STOP production immediately after success.
Do not execute the next stage or issue an owner approval yourself. Beaver ends
this turn and resumes execution after an owner decision. Inspect state, saved
files, scene and the appended decision note on resume before making changes.
Reject/reopen notes replace the legacy feedback channel for this workflow.
Rework invalidates downstream approvals while preserving candidate history.

When every stage is approved, use `beaver_asset_task` to finish the asset round
and let Beaver run its final checks. Keep approved file bytes unchanged and leave
no unsubmitted changed outputs or deletions. Final approval alone does not merge
the task. Final merge checks approved outputs, candidate-bound dependency closure
and required versioned checks. A later approved candidate may explicitly replace
an earlier dependency; unreviewed drift is rejected. Historical frozen inputs stay
intact. File hashing proves byte identity, not semantic or visual quality.

## Durable operations and configured capabilities

Use `beaver_workflow` for framework `state`, `start`, `inspect`, `cancel` and
`judge`. The host binds task / thread / turn. Configuration is owner-only: request
a missing pinned adapter or rule from the owner instead of installing a global
plugin, changing Beaver state or weakening checks. Prefer suitable configured
Godot, Codex and Blender plugins; inspect real readiness before using them.

`start` takes a stable `requestId` and one `job`:

- `{kind: "callback", request: {...}}` wraps a file-bearing `beaver_task` request
  (`work.begin`, successful `work.finish`, `submitDelivery`). Include its original
  callback request ID and revisions. This queues file IO without blocking polls.
- `{kind: "plugin", plugin: "registered-id", action: "probe"}` supports `probe`,
  `install`, `enable`, `reload` when registered. Installation, enabling or reload
  is followed by an independent probe. Only `ready: true` confirms exact versions,
  installed, compatible, enabled, callable and no pending restart. Handle missing
  adapters, incompatible versions, network/login/payment requirements or restart
  as explicit blockers. Never turn a successful process exit into a ready claim.
- `{kind: "check", candidateId: "..."}` runs configured versioned checks against
  frozen candidate/upstream files and separate historical attempt input exports.
  Read the report's `passed` and per-rule results; operation success alone does
  not mean that technical checks passed.
- `{kind: "inputExport", attemptId: "..."}` exports that attempt's frozen inputs.
- `{kind: "dependencies", command: {...}, paths: ["model.blend"]}` scans stored
  references. Pin the absolute Blender executable and SHA-256, use empty `args`,
  `timeoutSeconds` (1..600) and optional pinned `files`. Beaver supplies factory
  startup, disabled autoexec and its bundled scanner. Copy/import unresolved
  external inputs explicitly; packed and runtime-generated data need judgment.

Only one operation runs per task. Poll `inspect` with `operationId` until
`succeeded`, `failed`, `cancelled`, `interrupted` or `stale`. Read the result/error.
A callback result with `paused: true` ends this turn immediately, including when
obtained from `start` retry or `inspect`. Do not start more production work.
For other jobs, evidence is under `result.value`; callback results are receipts.

An exact same-turn start retry returns the original operation. Changed parameters
or a new turn cannot reuse that request ID. Query old operation IDs after recovery;
inspect the actual workspace/scene before retrying with a new ID. Cancellation or
restart may leave external side effects; Beaver records interruption and does not
automatically replay installation or relative scene mutations. File hashing/export
may finish its bounded IO before cancellation is observed.

Required rules and optional owner visual judgment gate stage approval and final
merge. `judge` takes `candidateId`, current `configurationRevision`, `verdict`
(`pass` or `fail`), `note`, and 1..16 frozen candidate PNG `paths`. Managed Codex
judgments are recorded as model opinion; they cannot satisfy an owner judgment
or approve a stage. Fixing a failed candidate requires a new submission/version.

Skills must read state, identify goal and acceptance, declare inputFiles at begin,
use configured capabilities, save outputs, finish the attempt and submit a
candidate. State the Skill reference/version, tool choices and limitations in
reports. These declarations are model-reported evidence. Beaver separately stores
observed Codex tool events, start-bound attempt/scene identity and adapter receipts;
missing events remain explicit evidence gaps. Never claim that providing a Skill
proves it was executed or that a tool name proves a plugin version was used.

For asset work without a delivery plan, use `stages` with both `expectedRevision`
and `assetRevision`, a new `requestId` and the complete stage list. This uses the
same state as `beaver_asset_task`; keep using that tool for safe checkpoints,
feedback and finishing asset rounds. Read `state` after each state change.
Use `beaver_ask_user` for questions and `beaver_submit_plan` only for the existing
planning-task lifecycle. A report does not schedule a subtask or stop a turn.

If a response is lost, query `receipt` with the original `requestId` first. An
exact retry in the same execution returns the original response, even if the
journal revision advanced or live input files changed. This confirms the saved
historical result without fresh file IO. While submission parks that execution for review,
state, receipts and exact retries remain available; new mutations are rejected.
Reusing an ID with different parameters or a new turn is a conflict.
After recovery query old receipts, inspect the workspace,
then use a new request ID for new work. Never blindly repeat relative mutations.

Reports are model-reported evidence, including `result` and `checkpoint` reports.
They do not save files, verify hashes, approve stages, or complete tasks. Use the
existing file/checkpoint/validation capabilities and inspect their real results.
Only Beaver's acceptance rules can authorize advancement. A `blocked` report
records the issue; ask the user or return control if execution cannot continue.
