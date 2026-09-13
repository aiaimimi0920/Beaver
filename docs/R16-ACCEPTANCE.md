# R16 NPR appearance workflow continuation

Recorded on 2026-09-11 (Asia/Shanghai). This continues the final request in
conversation `01a07f3e-3282-7a20-a358-b45520f8214b`: compare the straw-hat girl
with the Silver Wolf reference, improve the character, and improve reusable
production prompts while doing so.

The character revision, reusable prompt updates, standard game export and final
acceptance are complete. All four children and the parent are completed and
accepted. The last merge blocker was resolved in R19; see the final update below
and [R19 acceptance](R19-ACCEPTANCE.md). Earlier sections retain their historical
checkpoints, including the resolved model-service failure and partial progress.

## Retained implementation

The preceding session changed these files; this continuation preserved them:

- `resources/skills/blender-production/SKILL.md`: inspect real references;
  establish silhouette, face and primary hair masses before fine detail; assess
  target-renderer close-ups; integrate material work early; keep editable sources
  synchronized with exports; scope local edits to the affected region.
- `resources/skills/beaver-workflows/SKILL.md`: use visual checkpoints in task
  planning and acceptance; inspect matched before/after views; preserve the viewer;
  distinguish technical validation from artistic quality.
- `native/core/src/task_brief.rs`: route appearance tasks to the production skill
  and require plans to account for visible form and close-up quality.

Both complete skill files were found byte-for-byte in the compiled executable.
The real task event also contains the updated native appearance instructions.
This establishes prompt delivery, not successful artistic behavior.

## Native release and fresh verification

Package: `release/Beaver-native-0.1.19-preview-r16-win32-x64/Beaver.exe`.
The executable is 13,399,552 bytes. The complete release is 606 files and
17,011,797 bytes, including the license inventory.

SHA-256:
`143fcddda0f10a35431d14fe01e4cc0ee360f07b58036291c8a443cba4abeeb8`.

The prior release build completed successfully. During this continuation:

- Native packaging and package verification passed.
- `tsc --noEmit` passed.
- `cargo check --locked -p beaver-desktop` passed.
- The selected task-plan and task-board tests passed: 4 tests, 0 failures.
- Both skill folders passed `quick_validate.py`.
- Both skills passed Prettier and UTF-8 without BOM checks.

The packaged R16 was launched through `scripts/start-fresh-native-test.ps1`.
Its proof records zero old projects and tasks, the three blank-template files,
and the selected executable. Startup evidence is under:

`output/validation/fresh-round-20260910-171708-a486c17418594a3b89cd53f9683dc175/start-proof.json`.

## Continuing the existing character

The fresh blank project above proves host startup only. The requested appearance
revision continues the existing R15 character; it is not a new from-scratch
game-creation acceptance round. The existing project was registered using the
Beaver API, without copying it over the blank project or replacing prior evidence.

Project ID: `b5eb8b4d-996f-4800-a3ad-3e0e43503a3e`.
Task ID: `98b362b6-e6a5-4634-9044-dfd82f4a72cd`.

The task uses the configured local service, `gpt-6-astra`, 30 percent asking,
decomposition, and manual child approval for visual inspection. The original
review and front/multi-angle comparison images were read through Beaver APIs.
They show the existing face-line, repeated-hair, clothing-structure and fine
outline problems described in the preceding review.

## Historical model-service blocker (resolved below)

The user-specified `http://127.0.0.1:8317/v1` returned
`502 Bad Gateway: Unknown error` before model output. The first execution lasted
351,097 ms. A normal `task.continue` request with "Please continue" in Chinese
started another execution, which lasted 351,780 ms and failed with the same
error. Each execution recorded one automatic recovery attempt.

The task is `failed`, its changes are empty, and no child plan was produced.
The model service configuration was not silently replaced. No direct Codex,
Godot or Blender invocation, or externally authored game content, was used to
substitute for the failed Beaver task.

Once an authorized model service is available, continue this task, inspect the
actual visual checkpoints, complete the NPR validation and before/after views,
then export and verify the revised player program. Artistic quality and reliable
first-pass production remain unverified.

Structured evidence:
[`npr-r16-appearance-continuation-proof.json`](../output/validation/npr-r16-appearance-continuation-proof.json).

## Update: current Codex configuration applied

On 2026-09-11, the user explicitly selected the current Codex configuration.
Beaver code, review and translation now use `https://anyrouter.top/v1` and
`gpt-6-astra`, with the static API Key read from the current Codex auth file and
saved through `settings.save`. Credentials were not printed or written to evidence.

The same appearance task was continued. It now receives real model output,
has successfully discovered and inspected the NPR workflow, and has viewed the
existing reference comparisons. This clears the previous model-service blocker;
the appearance revision and its final export are still in progress.

Switch evidence:
[`npr-r16-service-switch-proof.json`](../output/validation/npr-r16-service-switch-proof.json).

## Recovery of conversation 01a08c08-1a6e-7ed0-8c5b-a3127f53e5da

The next continuation found no running Beaver process. It verified the same R16
executable SHA-256 and recovered the existing isolated round and its database,
without creating replacement tasks or replacing the project. Only the selected
R16 executable was running after recovery (PID 53948). This is recovery within
the existing appearance-revision round, not a new blank-project acceptance.

The same parent task has four manually approved stages: face, hair, clothing and
hat, then final validation and export preparation. `task.continue` on the parent
resumed the interrupted first child, `e34fdce1-4200-477a-ad55-57da6a2144cf`.
The remaining children are queued behind it. The retained before images and the
confirmed refinement scope were read through Beaver APIs. The code, review and
translation routes still report `https://anyrouter.top/v1`, `gpt-6-astra`, and a
stored credential; no credentials are included in this record.

The face stage is now completed, visually reviewed and manually approved. The
retained result is `face-02` geometry with `face-sdf-03` face shading. The review
corrected the jaw-to-neck transition and an isolated nose-shadow patch visible
under lateral light. Final face `validate` and `preview` both report `ok=true`,
`initialized=true`, `errors=[]` and `engineErrors=false`. The task also records
source/export consistency and 28 passing lab interaction checks. The dependent
hair task, `a5a73644-c317-4870-8a22-71462614bca4`, is running.

Live continuation evidence:
[`npr-r16-refinement-progress.json`](../output/validation/npr-r16-refinement-progress.json).
The remaining character stages, standard export and actual exported-program
smoke check are still pending.

## Final update: completed under R19

Conversation `01a08d8e-3333-72c1-803e-d3c7b50f7ce8` continued the same project,
data directory and task IDs. Face, hair, clothing and hat refinement completed
with visual review. The final validation child,
`8985991d-f082-4738-b0ab-19b20be5d83a`, finished its work but initially encountered
11 merge conflicts in Godot `.import` files. Each project file already matched
the child's recorded output.

The packaged R19 `task.retryMerge` operation left those matching files unchanged
and excluded them from the final task's rollback ownership, then integrated the
other 163 recorded changes. The task ID, report, thread, turn, baseline and workspace were
preserved. No new AI execution was started. All four children and the parent
were then accepted through Beaver APIs; the final state has no conflicts or
active tasks.

The existing standard export at `release/Beaver-game-DlYQUM/Game.exe` contains the
final accepted character and NPR laboratory. Its inventory was reverified through
R19. The managed final task recorded passing validation and preview, 67 project
lab checks, 67 embedded-pack checks through the bound editor, semantic asset
parity, and a separate real EXE window/control round with exit code 0. The
operator's standard export and package verification complete the handoff that
the child documentation leaves to the operator.

The native delivery is now `release/Beaver-native-0.1.19-preview-r19-win32-x64`.
Earlier native releases, character sources, task workspaces and progress evidence
remain available. The character still has simplified features and does not reach
the Silver Wolf reference's finish; this sample does not prove reliable first-pass
production from the improved prompts.

Final evidence:
[`npr-r19-completion-proof.json`](../output/validation/npr-r19-completion-proof.json).
Delivery paths, comparison images, exact hashes and verification scope are in
[R19 acceptance](R19-ACCEPTANCE.md).
