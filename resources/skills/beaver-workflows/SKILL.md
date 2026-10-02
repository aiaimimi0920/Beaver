---
name: beaver-workflows
description: Discover and use Beaver standard production workflows and enabled project feature packages, especially NPR characters.
---

# Beaver standard workflows

You make creative, planning, workflow-selection and recovery decisions. Beaver
provides reusable production capabilities; AI may participate at every stage.
Call `beaver_workflow_list` to discover enabled workflows. Read
`beaver.runtime.json` when present. Do not replace a bound custom engine with a
stock Godot download or change package requirements to hide failed validation.

For NPR character work, call `beaver_workflow_run` with workflow `npr-character`
and action `inspect`. Read the returned docs/model_authoring guide, contract and all three prompts
(geometry, textures, feature_data) before modeling. The pinned package lives at
addons/npr_character_frame; contract 1.1.0 and plugin 1.3.0 are separate versions.
Old addons/npr_characters projects require an explicit migration decision; never
install both packages because their class_name registrations conflict.
The Silver Wolf sample and its showcase launchers are not distributed. Beaver
installs shader settings directly; it does not enable the upstream editor demo
menu. The production actor, runtime, validators and authoring resources are retained.
For character appearance work, also read blender-production's authored-character
and reference-to-construction guidance before planning or making detailed geometry.
Use Blender MCP for actual modeling, saving editable `.blend` and exported GLB.
For NPR execution tasks, Beaver starts an isolated empty Blender GUI session,
binds its MCP client to a dedicated localhost port, and closes only that session
when this execution ends. Planning tasks only inspect workflows and need no
Blender session. Verify a real Blender MCP call before modeling. Never reconnect
to a different port or an unrelated user's open Blender scene. On continuation,
open your previously saved .blend through this session's Blender MCP. Save your
editable work before asking questions or ending a turn; unsaved session state
does not survive task interruption. Preparation failures appear in task events.
Keep all character production in this task workspace. Do not copy the diagnostic
mannequin as the requested finished character.

Author Body, Face and Hair single-surface meshes, the required textures/material
set, an NPRCharacterDefinition `.tres`, and a usable scene. Equipment may be
merged as disconnected Body islands under the package contract. Follow actual
user intent for appearance; no fixed character design is prescribed here.

Plan production around visible progress, not only file categories. When artistic
quality is part of the goal, define screenshot-based acceptance for the face,
hair silhouette, clothing and overall read at the intended viewing distance.
For a reference-led revision, connect each user-reported gap to an observed
reference structure, a construction decision and a visible acceptance condition.
Keep approved regions as a preservation baseline. Group connected head/neck and
facial-feature work coherently rather than splitting shared geometry into isolated
feature tasks. Keep the reference recipe concise and available to execution.
Establish an early target-renderer head/look-development checkpoint before
expensive detail work; geometry and material work may need to iterate together.
Do not approve a detailed but visibly unresolved asset merely to unblock the
next task. Keep the number of tasks proportional to the requested change.

If the user requests human-guided rounds, schedule only the current revision.
After saving its candidate and relevant validation evidence, stop for the user's
visual review. Keep the candidate available in Blender when requested and report
the exact source path. Do not automatically approve it or launch the next round.
Record feedback before advancing reusable artistic guidance from proposed to
confirmed; this opt-in review cadence does not limit ordinary autonomous tasks.

Call the standard workflow with action `validate` and the project-relative
`definition` path; this argument is mandatory, as the upstream checker otherwise
defaults to its Silver Wolf sample. Read status, compliance_errors,
visual_suggestions, measurements and not_evaluated, then decide how to repair.
The official .ci_script/model/check_model.gd exits 0 for structural pass, 1 for
compliance failure and 2 for report I/O failure. Its unmodified report remains at
artifacts/npr/<run>/model_check/report.json; Beaver saves a separate report.json
envelope with process diagnostics, definition/model-source hashes, and verified
pinned framework hashes. definitionAndModelHashesVerified covers only those two
character source files; external textures, materials, profiles and feature-data
dependencies are not covered (assetDependencyHashesVerified remains false). There is no
initialized field in the official report. Then use `preview` to
render front/side/back PNGs and inspect them. Both actions run the project's
bound engine. Preview first runs the same official compliance checker, and only
renders on a verified structural pass. A rendered image is not visual acceptance;
visualAcceptance remains not_evaluated until a human or external visual review. Reports contain actual validation and logs; `ok:false` requires
repair or an explicit unfinished report. Preserve reported artifact paths in
your delivery summary so Beaver's asset UI can display them. Subjective artistic
acceptance still requires visual inspection. Do not add unrequested documents
or polish merely to inflate task count.

For a supplied visual reference or a substantial appearance revision, inspect the
approved reference alongside matched before/after views and a readable head
close-up. Use comparable anatomical scale and projection; hidden anatomy may need
a temporary unoccluded structural view. Add three-quarter/profile and outline/light
comparisons when they help diagnose the defect, without repeating a large view
matrix for every local change. Preserve the user's existing viewer and controls.
For a localized edit, compare the affected region and any connected silhouette;
do not add unrelated head or costume work to satisfy a full-character checklist.
Report technical validation separately from the visible improvements and any
remaining difference from the reference. Do not claim reference-level quality
or a reliable first-pass result based on one successful character.
