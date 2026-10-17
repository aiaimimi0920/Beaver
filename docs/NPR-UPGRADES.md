# Explicit NPR dependency upgrades

The host supports two compiled, trusted historical NPR pins:

- `48839ce137ce2749210a613c565b602c5a6bbc4a`, from Beaver `380f3e4bf3afd486a772d2efba46a59e5170c743`
- `05f3dceee965c9a5ec78ee8d159e962c133cfc57`, from Beaver `bff0ca6`

Both upgrade to the current compiled package. Historical trust manifests are
retained byte-for-byte under `resources/workflows/`. Project provenance cannot
authorize different files. The host checks every old dependency and the installed
provenance, refuses customization/new-path conflicts, and verifies the complete
current package before committing the runtime pin. No engine is launched.

## Scope and prerequisites

The project must use opened project-local storage. Stop all project tasks,
managed jobs, game players and live previews. A paused `waitingChildren` parent
is allowed only while its children and operations are also stopped. Unconfirmed
external-process cleanup blocks the operation. A task target must belong to the
project and be interrupted or failed, with no accepted result.

The project root and a task workspace are separate targets. Upgrading one never
implicitly changes the other. Task paths are resolved from the owned storage
layout, never from a caller-supplied path. The native NPR panel exposes inspection,
upgrade, rollback and the separate dependency-review action.

## API flow

1. `project.npr.inspectUpgrade`: `{id, taskId?}`. Returns the exact runtime SHA-256,
   source/target pins, changed paths, target, eligibility/blockers, pending and
   latest operations, current workflow, frozen workflow and review state.
2. `project.npr.upgrade`: `{id, taskId?, expectedOldCommit, expectedRuntimeSha256}`.
   Use the inspected identities unchanged. The response includes `operationId`,
   `status`, `runtime`, `runtimeSha256`, `changedFiles`, and `reviewRequired`.
3. Inspect again. For a task with a frozen NPR binding, explicitly review the
   changed dependency and the existing acceptance scope.
4. `project.npr.reviewUpgrade`: `{id, taskId, operationId,
   expectedRuntimeSha256, expectedFrozenWorkflow, reviewedWorkflow,
   reviewReason, acceptance}`. `reviewedWorkflow` must exactly equal the current
   compiled workflow descriptor. The host requires the successful operation for
   that exact task workspace. It retains append-only review history and a current
   reviewed execution binding; original task and parent plans are never rewritten.
   This is a dependency-only review, not new task-goal or output acceptance.
5. Continue the task separately. Launch verifies the durable host review against
   the unchanged frozen plan, operation, target and current dependency hashes.
   Normal continuation text can be appended without mutating the reviewed plan.

A lost upgrade response can be resolved through inspection's `latestOperation`.
Completed upgrades are not automatically rerun. Model validation and visual
acceptance remain separate from dependency integrity and planner review.

## Recovery and rollback

Before mutation, original bytes are saved as verified project-local content
blobs and a FULL-synchronous host-store receipt records every before/after hash.
Runtime status becomes `upgrading` before dependencies change. File writes use
atomic temporary-file replacement and compare current hashes before mutation.
The runtime pin commits last; engine and unrelated metadata remain unchanged.
Timestamps belong to the operation receipt rather than runtime bytes, so root and
task copies with identical original manifests produce identical upgraded bytes.

Errors attempt rollback. A crash or blocked rollback leaves a durable pending
receipt that fences continuation, launch, object claims, validation and ordinary
project writes. Inspect it, resolve any external-file conflict, then call
`project.npr.rollbackUpgrade` with `{id, operationId}`. Every path and backup is
preflighted; concurrent edits are preserved. Task rollback still requires a
stopped, unaccepted task. Backups and history remain after rollback, and the
superseding dependency review becomes invalid.

## Focused verification

- `node --import tsx --test tests/vendor-npr.test.ts tests/npr-upgrade-trust.test.ts tests/task-plan.test.ts`
- `npm run typecheck`
- `cargo test --locked -p beaver-core --lib workflows`
- `cargo test --locked -p beaver-core --lib task_plan`
- `cargo test --locked -p beaver-headless`
- `cargo fmt --all -- --check`
- `npm run check:effective-lines`

The upgrade tests use exact historical byte fixtures and temporary projects;
they never launch Godot, Blender, or an agent.
