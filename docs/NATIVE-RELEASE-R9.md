# R9: selectable clarification and live interaction acceptance

Date: 2026-09-09.

## Fix

The Codex-facing beaver_ask_user schema previously omitted options even though
the persisted question format and UI supported them. It now requires 2..6
structured options per question, with labels and descriptions. The tool guidance
asks for one decision per question, up to three questions per round, and retains
custom answers without manufacturing an Other option.

The UI no longer forces the suggestions selector to an empty value after every
selection. It displays the selected option and its explanation, lets the user
edit the answer freely, and disables the selector during submission. Existing
text-only historical questions remain answerable; they are not rewritten.

## Actual release

- Executable: `release/Beaver-native-0.1.19-preview-r9-win32-x64/Beaver.exe`
- Complete payload: 606 files, 15,665,237 bytes.
- EXE SHA256: `25af10193af70bbe62daf999ce8ca7c038adce3e1cfe6277fd48d30d235022c5`.
- Release compilation passed in 3 minutes 10 seconds.
- Post-runtime release manifest/hash verification passed.
- Clarification tests: 3 passed. TypeScript typecheck and changed-source Prettier
  checks passed. The native UI and embedded Codex tool schema were rebuilt.

## Live interaction verification

This uses the actual packaged R9 host, its API, the imported live provider and
real Codex execution. The operator did not directly call Codex, Godot or Blender,
write game files, or produce game design documents. UI automation only inspected
and edited answer drafts; creative submissions went through Beaver's API.

- Project ID: `c7a3d454-1dfe-4497-85eb-0bfff3d95f78`.
- Task ID: `0adbc4f6-864e-47ad-9c13-d3e7d184a70f`.
- Isolated data: `output/validation/choices-live-1788915146157`.
- Original user input was submitted without extra instructions.
- First round: three questions, each with three actual structured options.
- UI selection, visible selected value, explanation, custom answer, per-question
  draft retention, all-answers submission gating and reload recovery passed.
  Script: `output/playwright/choice-check.mts` (7 checks).
  Screenshot: `output/playwright/choices-selected-r9.png`.
- Missing answers and an unknown clarification ID were rejected without leaving
  awaitingInput. Valid preset/custom answers were persisted and resumed execution.
- Immediate replay was rejected. The first-round custom request for mutual aid
  was retained in the executor's subsequent response.
- API interrupt reached interrupted; continue with the short request to choose
  protagonist identity and difficulty resumed the same task.
- Second round: two questions with four and three options. Questions incorporated
  the already selected direction instead of asking those decisions again.
- The UI rendered the new options and did not reuse previous-round answer drafts.
  Script: `output/playwright/choice-round2.mts` (3 checks).
  Screenshot: `output/playwright/choices-round2-r9.png`.
- Old-round replay and whitespace-only answers were rejected while preserving
  the current awaitingInput state.
- Second-round answers combined a preset difficulty with a custom protagonist
  background. Both were persisted and execution resumed.
- A short running-task instruction to confirm choices and wait before production
  was accepted after the startup-window issue below. The task finished with an
  explicit statement that the game had not been made and would await permission.
- `document.read` through the API confirmed Beaver's own decision record retained
  the protagonist, former enterprise employment, difficulty and mutual-aid intent.
  The operator did not author or patch that game document.

The final task status is completed for this confirmation-only turn, not for a
completed game. The host remains open for continuation; no creative task is left
running at this checkpoint. The earlier R8 unanswered test remains untouched.

## New issues recorded, without interrupting the test

- INT-003: The fixed-width selector truncates longer selected labels. The full
  answer and explanation remain visible elsewhere, so this did not block testing.
- INT-004: Calling task.continue immediately after task.answer can return HTTP
  409 / "任务尚未就绪，请稍后补充". The task was transitioning into execution.
  The rejected addition was not persisted. After checking state, one normal API
  retry succeeded. This startup-window interaction still needs improvement.
- Test harness note: the first reload assertion ran before the task list mounted;
  the harness was corrected to wait for the task entry. No product change was
  made for this automation timing issue; all 7 checks then passed.

The first-round missing-options defect is repaired and exercised with live model
output. This release does not claim finished full game-production acceptance,
play/export validation, entertainment quality, or completion of the remaining
project roadmap. The next end-to-end stage can continue this task with a short
human request to begin production.
