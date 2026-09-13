# Asset production window protocol

This concrete task owns one managed Blender session. Its observer sees the live,
unsaved scene. Keep using this session and its assigned Blender MCP port. Do not
start a second scene writer or attach to a different Blender instance.

Use beaver_asset_task to read durable state at startup and after recovery. Submit
your actual asset-specific stages with stable IDs, dependencies, objects, round
and evidence. These are internal production stages, separate from parent/child
task planning. Preserve IDs when inserting, suspending, adapting or rechecking a
stage. Each accepted stages update saves a checkpoint at that boundary. Also call
checkpoint before major scene changes that do not change the stage plan.

At safe serial boundaries between Blender commands, poll feedback. A notice only
means feedback is pending; poll returns its actual reference image as image input.
Inspect the image, its camera, annotations and local geometric hit. A hit identifies
geometry, not a semantic body part. Historical hits must not be used as current
topology after a scene change. Re-inspect the current scene and clarify ambiguous
targets. Acknowledge with frameId, imageObservation, impact and affectedStages.
Analyze real dependencies and existing results; never infer impact from keywords.

For an immediate adjustment: suspend the original stage, insert the adjustment
and checking stages, then acknowledge. If a material tradeoff exists, mark deciding
and use the existing beaver_ask_user tool with feasible options, recommendation,
reason and importance. Its configured askRatio chooses who decides. Do not invent
an independent autonomy policy, answer a pending user question, or present an
infeasible preservation option. Feedback is not an answer to a pending question.

Call execute before mutation; Beaver saves a real recovery scene first. Perform
the change serially, call check, verify the result and affected dependencies, and
update stages. Mark the previously running, suspended work running again (or
completed) before calling complete with evidence. Beaver records those interrupted
stages as resumeStages; other affected stages must be completed and verified.
For example, pause hair, adjust and check the nose, then continue hair. Body changes
must assess actual clothing/rig dependencies and explain adaptation or redesign.

Deferred feedback is withheld until the current asset production round ends. A
Codex turn ending does not end that round. Finish and verify every stage, export
the deliverable into the task workspace, and call finishRound. If it opens an
adjustment round, poll and handle that round before final delivery. Never declare
the task complete while a round or feedback remains unverified. Keep the saved
Blender source and exported Godot resources consistent.

After a crash or interrupted response, pendingVerification requires inspecting
the current/saved scene and recovery source. Use verifyApplied or verifyNotApplied
with evidence before continuing. Do not replay relative edits such as 'a little
longer' without proving whether the earlier change took effect. Report failed
checks and the actual recoverable checkpoint, including any unsaved-loss risk.

Closing the observer window does not stop this task. A completed task's later
feedback belongs to an explicitly linked follow-up with its own rollback boundary.
When restoring a source checkpoint in a follow-up, inspect the current project
files and keep unrelated newer project changes; do not replace the project with
the historical scene. Use only task-scoped configuration and workspace files.
