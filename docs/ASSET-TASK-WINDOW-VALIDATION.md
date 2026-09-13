# Asset task window validation

Initially recorded on 2026-09-13; extended on 2026-09-14 (Asia/Shanghai).
Implementation is isolated in
`C:\Users\Public\nas_home\beaver2`, branch `beaver2`, based on
`393e2aa1c345312d1f6df85c9d1abe71c59d8f81`.

The original integration candidate includes main checkpoint
`693aa781623b865639613e688095c46b41a2b8bb`. Engineering checks and the final
native merge smoke passed. The first sections preserve the v0.3 checkpoint;
the v0.4 sections record follow-on fixes and completed native production rounds.
Remaining acceptance limits are stated explicitly. The original branch update
and preserved concurrent edit are recorded in `output/asset-main-merge-proof.json`.

## Completed checks

Commands ran in the normal PowerShell toolchain environment with the existing
MSVC `LIB` preserved. Every `output/...` path below is relative to
`C:\Users\Public\nas_home\beaver2`, including when this document is read from
the original Beaver checkout. Generated logs and artifacts remain ignored.

| Check                                                                                         | Result                                                            | Evidence                                                                        |
| --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| `scripts/start-fresh-native-test.ps1 -Build -Executable target/release/Beaver.exe -Port 4322` | Release build passed, Cargo 5m 35s; fresh native launch passed    | `output/asset-merge-final-native-build.log`                                     |
| `rtk proxy cargo test --locked -p beaver-core`                                                | 134 unit, 20 asset contract and 4 structure contract tests passed | `output/asset-create-core-tests.log`                                            |
| `rtk proxy cargo test --locked -p beaver-desktop`                                             | 5 passed                                                          | `output/asset-merge-desktop-tests.log`                                          |
| `rtk proxy npm run test:effective-lines`                                                      | 7 unit and 4 contract tests passed; overlaps the full core suite  | `output/asset-merge-effective-lines-tests.log`                                  |
| Focused frontend asset, draft and autonomy tests                                              | 14 passed                                                         | `output/asset-merge-frontend-tests.log`                                         |
| `rtk proxy python -m unittest discover -s tests -p 'asset_observer*_test.py'`                 | 10 passed                                                         | `output/asset-observer-test.log`                                                |
| `rtk proxy npm run typecheck`                                                                 | Passed                                                            | `output/asset-merge-typecheck.log`                                              |
| Official Prettier check on integrated frontend and test files                                 | Passed, 41 files                                                  | `output/asset-merge-prettier.log`                                               |
| `rtk proxy cargo fmt --all -- --check`                                                        | Passed                                                            | `output/asset-merge-final-rustfmt.log`                                          |
| `npm run check:effective-lines`, included by native build                                     | Passed, 451 sources, no violations                                | `output/asset-merge-final-native-build.log`, `output/effective-code-lines.json` |
| Native integration smoke after atomic asset-state fix                                         | 11 checks passed                                                  | Final round's `native-merge-smoke.json`, identified below                       |

The v0.3 release executable was `target/release/Beaver.exe`, 16,592,384 bytes, last
written at `2026-09-13T08:30:37.641Z`. Its SHA-256 is
`80050f4b9f6d09748fa37c1df09803c531924f4cf3eb392c1605b4bbeae3fe45`.
The final launcher's `start-proof.json` independently records that hash, PID
46668, fresh data and WebView directories, zero prior projects/tasks and a new
blank project. This test instance has since been closed.

PowerShell captured some native stderr as `NativeCommandError` metadata in logs.
Results above use process exit codes and the actual test runner summaries.

The structure pass scanned 451 sources with zero violations and 18 unchanged
legacy files. Relative to the common baseline, 162 source files changed,
including 14 immutable NPR files. The 148 non-vendored changed sources have at
most 461 effective lines (`native/core/src/task_finish.rs`). No size baseline
was relaxed. Final staged whitespace, BOM and conflict-marker checks are
recorded in `output/asset-merge-final-checks.json`; the scoped document formatter
log is `output/asset-merge-doc-format-check.log`.

## Scope of the evidence

The Rust contract tests exercise stage CAS/DAG validation, suspension/resumption,
clothing revalidation, image integrity and rendered annotations, immutable picks,
idempotent submissions, cancellation, delayed production rounds, uncertain
relative edits, finalization races, followup isolation and crash repair. The
dynamic tool tests check real PNG `inputImage` response content and stale
thread/turn rejection.

Integration regressions cover asset feedback at the main branch's validation
gate. The first integrated native smoke exposed an HTTP 409 when an asset task
was interrupted before Blender startup. Asset state is now inserted atomically
with the task, initial event and related receipt. Two regressions first failed
and then passed: queued state survives interruption and reopening the database;
an injected asset-state insert failure rolls back all related records. Evidence:
`output/asset-create-red-tests.log` and `output/asset-create-core-tests.log`.

Frontend tests exercise resize/letterbox/high-DPI coordinate mapping, bounded
camera motion, old-frame rejection, latest-view coalescing, disposal, transport
failure, immutable retry identity, corrupt-draft recovery and storage failures.

Python tests exercise the actual loopback HTTP server with authenticated identity,
Origin rejection, body bounds, unsupported operations, eight bounded leases,
three cached frames, four queued commands, latest-view coalescing, twelve-thread
saturation and capacity recovery. They do not run Blender or validate its GPU.

The NPR package's published byte hashes remain the exemption authority. All 98
files match the manifest. Of the 94 checkout files restored from the verified
original files, 14 retain differences from the canonical Git baseline, confined
to CR line endings; the other 80 already match the baseline bytes. The new
`.gitattributes` disables text conversion only for that pinned package and
recognizes upstream CRLF in Git whitespace checks while retaining the normal
whitespace rules. No baseline or provenance manifest was relaxed.

## Native rounds completed

The user authorized closing Beaver and merging. All rounds used the native
launcher and a new blank project, with game creation driven through Beaver APIs
only. API secrets were not recorded in command arguments or evidence files.

The real production round is
`output/validation/fresh-round-20260913-063243-6d3763c0c5da4c148fd292875b990a37`.
Project: `67560046-93e4-43cf-81ea-dee38d1f4509`.
Task: `3822e79f-5c94-45dd-a77b-55191a609a22`.

The prompt requested a simple low-poly person with nose, clothes and hair, and
the feedback was "鼻子小一点". This round showed the independent native window,
real GPU frames, Codex consumption of the annotated reference, a local Nose
pick and a visibly smaller nose. Frame-matched camera latency was 183 ms orbit,
159 ms pan, 150 ms zoom and 154 ms reset. Evidence in that round includes
`camera-interaction.json`, `window-opened.json`, `nose-feedback-submit.json`,
`nose-feedback-reference.json`/`.png`, `nose-after-reference.json`,
`nose-after.png` and `stale-pick-rejection.json`.

Awaiting-input scene retention, feedback-ID retry, checkpoint recovery and stale
session rejection were observed. The round ended at its configured 20-minute
limit: task `interrupted`, asset phase `producing`, feedback `pendingVerification`.
Hair suspension/resumption and completed feedback verification were not proved.
Final snapshots are `output/asset-native-final-state.json`,
`output/asset-native-final-status.json` and `output/asset-native-final-events.json`.

The final integrated smoke round is
`output/validation/fresh-round-20260913-083037-9940e0d09aeb446a9e1e849e672a9bf1`.
Project: `ffd6f106-89f0-4c1d-83d5-c5d85bac769f`.
Task: `998c1e72-5963-4b7e-b2c6-9495f59de68d`.
Its `start-proof.json` and `native-merge-smoke.json` prove exclusive executable
identity, fresh blank state, coexistence of validation and asset APIs, asset
state after immediate interruption, native window opening and identity reuse,
high-frequency status response without a scene, and no remaining active tasks.
All 11 assertions passed. This round did not run GPU or production acceptance.

The selected PID 46668 was closed after an API check found zero active tasks.
No Beaver process remained; the owned encrypted temporary token was deleted.
Evidence: `output/asset-merge-final-shutdown.json`. Earlier failed rounds remain
available as evidence, including the pre-fix interruption smoke in
`output/validation/fresh-round-20260913-080635-a9732ca770ac40c49dff771c82268328`.

## Remaining product acceptance at v0.3

The open scenarios in
[the design, section 9](ASSET-TASK-WINDOW-DESIGN.md#9-验收标准) include:

- Hair suspension, completed nose-feedback verification and hair resumption.
- Clothing-dependent changes at `askRatio` 100, 0 and an intermediate threshold.
- One-mesh targeting, renaming, instances, modifiers/topology and actual high-DPI
  annotation behavior; broader busy/frame-age and latency coverage.
- Two-task isolation, deferred feedback through completed delivery, the full late
  followup lifecycle and uncertainty recovery after abnormal shutdown.

These were unverified at the v0.3 checkpoint. The v0.4 evidence below supersedes
that historical list without replacing the earlier round artifacts.

## v0.4 business-error fix and native production evidence

The original integration is committed at
`977b788ff28facdfa2045ed4a294515d3d2cbbdc`. The follow-on fix is limited to
`beaver_asset_task`: business failures return a successful JSON-RPC response
containing `success: false` and readable `inputText`. Previously, a missing
stage revision surfaced as the generic `dynamic tool request failed`, hiding
the actionable instruction to read state and submit its current revision.
Other dynamic tools keep their existing error behavior.

The dispatch regression runs an actual RPC child process. It proves the invalid
stage call exposes its error, the next state call still succeeds, and neither
stages nor checkpoints change. It failed before the fix and passed afterward.
Evidence: `output/asset-tool-error-red.log` and
`output/asset-tool-error-green.log`.

Matching validation passed without changing the size baseline:

- `cargo test --locked -p beaver-core --lib --test asset_contract`: 136 unit
  tests and 20 asset contract tests; `output/asset-tool-error-validation.log`.
- Cargo fmt, instruction-file Prettier, effective-lines and diff checks:
  `output/asset-tool-error-format.log`, `output/asset-tool-error-doc-format.log`
  and `output/asset-tool-error-lines.log`.
- Required native launcher with `-Build`: release build passed in 4m 25s;
  `output/asset-fixed-native-start.log`.

The fixed executable is `target/release/Beaver.exe`, SHA-256
`2d584d03c30628f97670a7d316ffed4afb3eea1a3dcb54a59a4ef6ac19e27d6f`.
Both new rounds record this hash and exclusive executable identity in their
`start-proof.json`. The live model correctly updated revision 0 to 1 and saved
a checkpoint. Deliberate malformed-call recovery by the live model was not
induced for the missing-revision case; its direct proof is the RPC regression.
The later rename round independently produced a real invalid feedback
transition. The tool returned readable `Invalid feedback transition: checking
-> deciding` content, and the model's next state call succeeded without ending
the task (`edge-rename-confirmation-state.json`,
`edge-rename-through-instance-events.json`).

### Completed production, isolation and rollback round

Round directory:
`output/validation/fresh-round-20260913-170320-439c45dc007743229634dd93214f6baf`.
Project: `1efb6e49-512c-4546-8a61-42d7c10150d2`.
Character task: `2622bb68-5986-4cb2-b5a3-0fd454d2fdc3`.
Stool task: `3c1dfe4a-c83c-47b2-a524-6e82868a65bd`.

The character completed three production rounds, all nine stages and all three
feedback records. Immediate "鼻子小一点" suspended hair, executed and checked
the nose change, then resumed hair. The model actually read the reference
images. At `askRatio: 100`, "腿长一点，胸大一点" produced an importance-78
choice about the chest silhouette; the user selected a wider/thicker chest.
The model adapted the trousers, top and sleeves and checked the result. A
deferred navy-shoe request submitted while that question was waiting neither
replaced the question nor ran early; it completed in round 3. Actual GLB shoe
material is `[0.012, 0.032, 0.12, 1]`, with cream soles retained.

Evidence includes `asset-observations.jsonl`, `main-latest-state.json`,
`main-latest-events.json`, `clothing-answer-state.json`,
`waiting-feedback-independence.json` and `window-reopen-proof.json`.
Closing/reopening the asset window retained the waiting question, queue and
same Blender session. Loss of an unsubmitted draft on close was not established
as a reproducible bug.

Two real Blender sessions rejected cross-task/session use and kept camera
state isolated (`two-task-isolation.json`, `camera-isolation.json`). A
frame-matched camera change took 106 ms. Real native DPR 1.5, a 1440 x 900 CSS
viewport and a letterboxed 720 x 540 reference were used for pointer box,
arrow and brush operations. The normalized SVG coordinates and actual rendered
screenshot were checked: `output/asset-three-annotations.log` and
`output/playwright/asset-three-annotations.png`. The separate resized-viewport
case records its emulated DPR and is not counted as physical high-DPI proof.

The stool's deferred blue-seat change completed before delivery. A later
white-leg request became followup task
`b184d2a8-76aa-45b0-883d-435b1fbd39ed`. After it delivered, `task.rollback`
with `keep: []` restored all eight changed files to their original SHA-256
values, left the source task JSON unchanged and restored the actual GLB's blue
seat/red legs. Evidence: `late-followup-create.json`,
`late-completed-state.json` and `late-followup-rollback.json`.

The character encountered real delivery conflicts in `main.tscn`,
`project.godot` and `beaver.validation.json`. Normal application recovery via
`task.dialogueRollback` created integration task
`a2011180-37be-47c5-b8e4-b23c9451f50d`. It completed delivery and code validation;
all 40 changed paths match their recorded `after` hashes. The actual
`Fresh game/docs/art/previews/integration_integrated.png` was visually checked
and shows the character with navy shoes and the blue-seat/red-leg stool.
The original character task's `conflict` remains historical state.
Evidence: `integration-delivery-proof.json`, `integration-latest-state.json`
and `round-complete-state.json`. The agent reported GUT 6 tests/73 assertions;
standard game export and package validation were not performed.

The round ended with zero active tasks, and the required launcher closed its
PID 40536 before creating the next blank project.

### Single-mesh, automatic-choice and recovery round

Round directory:
`output/validation/fresh-round-20260913-175245-bca923e4757a439b9a07129d0eaf290d`.
Project: `0391d1bb-8338-4f3d-aa42-52692f6f67d4`.
Task: `969c9b19-2891-43b7-a79b-e18f211dc962`.

The live Face mesh contains 352 vertices and 322 polygons. Nose and cheek picks
on the same frame both identify object
`5668022f-9bdc-4017-b49c-366c899f8196:initial:1073`, but return distinct local
coordinates and face indices (129 and 100). Evidence:
`single-mesh-picks.json`, `single-mesh-head-reference.json` and
`single-mesh-head.png`. The model's claim of a closed, single connected mesh
is separate from the independently observed same-object/different-local-point
proof.

At `askRatio: 0`, the first leg/chest feedback completed clothing impact
analysis, adjustment, visual verification and hair resumption. A further request
for a more pronounced change produced a genuine importance-60 clothing choice;
the automatic recommendation selected "重建贴身轮廓" over further local
stretching. Its answer and policy metadata are preserved in
`crash-before-state.json`.

During an actual `execute_code` busy period, eight camera targets with sequences
110 through 117 were submitted. Sampling saw the old frame at revision 10,
then the latest frame at revision 117; the final sample two seconds later was
still 117 with frame age 273 ms. Maximum observed stale-frame age was 24,116 ms.
This proves sampled recovery to the latest view, not a universal latency bound.
Evidence: `busy-latest-camera.json` and `live-observer-samples.jsonl`.

An intentional abnormal exit targeted only the verified owned Beaver PID 33472
while the second clothing feedback was `checking`. Restarting the same binary
and data directory produced task `interrupted`, a null asset session, a
disconnected observer and feedback `pendingVerification`. The recovery message
explicitly limits recovery to the last saved scene. A later check confirmed no
automatic task restart, and an old pick was rejected. Only an explicit
`task.continue` resumed execution. Evidence: `crash-stop.json`,
`crash-restart-process.json`, `crash-restart-state.json` and
`crash-explicit-continue-proof.json`.

After explicit continuation, the model compared the older recovery checkpoint
(`Top72`, leg length 0.91 m, extension 0.10 m) with the latest saved project
scene (`refit_applied`, `Top144`, leg length 1.03 m, extension 0.22 m).
Feedback `f276fe56-25a5-4cdd-a445-5b5c3ec9ba4e` records `verifyApplied` at
`2026-09-13T18:13:33.316Z`, followed by `completed` at
`2026-09-13T18:16:09.154Z`. The model initially reported loading too early;
the actual successful load is recorded after a failed call with an undefined
cross-call variable. Subsequent tool output confirmed the saved refit, and
the actual edit only refined the top's back and hem. Final geometry retained
the 1.03 m leg length, without repeating the relative extension.
Evidence: `recovery-events.json`, `recovery-final-events.json` and
`recovery-final-state.json`.

The native busy indicator was also captured during a real save operation:
`output/playwright/asset-busy-recovery.png` visibly shows
"忙碌: save recovery scene" together with the frame age. The screenshot was
visually inspected; sampled text in `output/asset-busy-ui.log` includes a
one-second frame age. The screenshot itself shows zero seconds and is not
used as proof of a long frame stall.

The recovered task completed at revision 10 with phase `ready`, all seven
stages and both feedback records completed. All 74 delivered paths match
their recorded `after` hashes (`recovery-delivery-proof.json`). The actual
`Fresh game/assets/character/godot_full.png` was visually inspected and shows
the clothed character fully in frame. Godot import and scene assertions
completed after normal recovery from an initial missing GLB import. GUT was
prepared for Beaver validation; standard game export/package acceptance was
not performed.

Reopening this completed task through `assetTask.open` retained the native
window identity. The real window displayed its saved model image, all seven
stages completed and the explicit label "交付时保存的画面 · 已停止实时同步".
Camera controls were disabled and the input explained the independent followup
boundary. The actual screenshot was visually inspected. Evidence:
`completed-window-reopen.json`, `output/playwright/asset-completed-reopen.yaml`
and `output/playwright/asset-completed-reopen.png`.

### Intermediate decision and observer identity followup

The same recovery round continued through asset followup task
`e4738193-f2c5-4cd6-92e8-f2a143cab623`. Submitting to the delivered task with
`assetTask.submit` created the new asset task and retained its source reference.
An earlier generic `task.followup` attempt did not inherit asset-task state;
that task (`d184503f-eb9c-498e-8785-f7dcac0e6c63`) was immediately interrupted
and not resumed. This records the route distinction without classifying it as
a product defect. Evidence: `edge-asset-followup-create.json`,
`edge-followup-create.json` and `generic-followup-interrupted.json`.

At `askRatio: 70`, a real importance-65 choice about further chest widening
waited for input, matching threshold 30. The selected answer was "局部适配上衣";
there were no automatic answers. The answer was submitted at
`2026-09-13T18:32:57.328Z`. The task was subsequently set to 100 for the observer
confirmation steps; this did not alter the original choice's recorded policy.
Evidence: `edge-intermediate-decision.json` and
`edge-intermediate-completed.json`.

Feedback `4bd29839-d6ac-43e2-ae75-51ac1fd5c25b` completed with top width changing
from 0.464 m to 0.51967996 m. Model tool output reports unchanged waist, collar
and depth, adapted sleeve roots, and per-vertex/transform preservation of the
other 27 meshes. It checked actual front, oblique and side previews and recorded
Godot `CHEST_CHECKS_PASSED`. A cross-call temporary variable failed after the
deformation; ordinary recovery verified that it had already been applied and
did not repeat the widening.

The deferred rename feedback stayed queued until that clothing round ended.
Independent observer picks then showed `Face` becoming `Head_Main`, retaining
the exact object/instance ID, 352 evaluated vertices, 322 polygons and the same
local hit. The old reference returned HTTP 409. The scene revision rose from
1 to 14 across both clothing work and rename, so this does not isolate rename
as the sole invalidation cause. Evidence: `edge-before-reference.json`,
`edge-before-pick.json`, `edge-post-rename.json` and `edge-rename-old-pick.json`.
Despite its filename, `edge-pre-rename.json` was sampled after the rename and
must not be used as a before snapshot. The model requested the explicit rename
confirmation at `2026-09-13T18:44:13.044Z`; the confirmation state and picked
reference are preserved in `edge-rename-confirmation-state.json` and
`edge-rename-confirmation-reference.json`.

The next deferred request placed a collection instance 0.48 m beside the original
head. Independent picks from the same frame returned the same object ID ending
in `:498`; the original instance ID equalled that object ID, while the collection
instance added suffix `:cfa1fcc5e9c7c735d414`. Both exposed 352 vertices and
322 polygons, with distinct world hits. The actual observer frame was visually
checked and shows the full character head beside the isolated comparison head.
Evidence: `edge-instance-wide-frame.json`, its referenced PNG,
`edge-instance-picks.json` and `edge-instance-confirmation-state.json`.

The following round added a level-1 Simple subdivision modifier without applying
it. Model tool output confirmed that the base mesh retained 352 vertices and
322 polygons. An independent observer pick returned the same object identity
with 1346 evaluated vertices and 1344 polygons. The pre-modifier reference
returned HTTP 409 (`Reference frame expired; refresh before selecting`).
The model saved the modifier, checked Blender/GLB sharing and Godot geometry,
then waited for the requested user confirmation. Evidence:
`edge-before-modifier.json`, `edge-after-modifier.json`,
`edge-modifier-old-pick.json` and `edge-modifier-confirmation-state.json`.

After applying the modifier and triangulating, the independent pick retained
the same object identity and exact local/world hit, with 1346 vertices and
2688 polygons; its face index changed from 551 to 1102. The previous reference
returned HTTP 409. The model reported no remaining modifier, preserved evaluated
surface geometry and successful shared-resource/Godot checks, then waited for
confirmation. Evidence: `edge-topology-submit.json`, `edge-after-topology.json`,
`edge-topology-old-pick.json`, `edge-topology-confirmation-state.json` and
`edge-through-topology-events.json`.

Saving and reloading the current source kept the same connected Blender session
but changed generation from `3b7506dff29340a491c311ff61a763fa` to
`14518b560be948ddaa8f4b1e1d43f32b`. The old reference returned HTTP 409. A fresh
pick succeeded with the new generation/object identity, the same local/world
hit and unchanged 1346 vertices/2688 polygons. The actual reloaded observer
image was visually inspected. Model checks reported all 35 objects' mesh and
transform data preserved. Evidence: `edge-reload-submit.json`,
`edge-after-reload.json`, `edge-reload-old-pick.json`, `edge-reload-proof.json`,
`edge-reload-confirmation-state.json` and `edge-through-reload-events.json`.

After the final confirmation, followup task
`e4738193-f2c5-4cd6-92e8-f2a143cab623` completed at revision 24 with phase
`ready`: all 13 stages and all six feedback records completed, with no unanswered
questions. All 80 delivered paths match their recorded `after` hashes.
Evidence: `edge-latest-state.json`, `edge-latest-events.json` and
`edge-delivery-proof.json`. No further acceptance scenarios were added.

## v0.4 coverage and remaining limits

The completed native rounds provide direct production evidence for live GPU
observation, reference-image consumption, high-DPI annotations, local picking,
rename/instance/modifier/topology validation, all three decision policies,
hair suspension/resumption, deferred feedback, independent followup rollback,
two-task isolation, saved-preview reopening, scene reload and explicit recovery
from an abnormal Beaver exit. They do not establish every section 9 requirement
as a separately injected native scenario:

- Direct camera input bypasses the model and uses offscreen view matrices in
  `resources/workflows/asset_observer_view.py`. Actual camera latency and
  isolation were measured, but no runtime negative trace counted LLM requests
  or compared the AI viewport/output camera before and after the mouse events.
- Busy-view recovery was sampled. The bridge's deterministic test submits 1000
  view updates and observes only the latest target with no main-thread queue
  backlog (`tests/asset_observer_bounds_test.py`); the live sample does not prove
  an invariant for every unobserved frame.
- The exact simultaneous finalization/submission race and validation-gate
  ordering are proved by deterministic core contracts, including both orderings,
  late invalidation, retry identity and deferred rounds. A live UI race was not
  deliberately scheduled at the atomic boundary.
- Actual scene reload and an abnormal host exit were injected. A standalone
  forced Blender crash was not separately injected. Recovery only claims the
  scene data saved before interruption.
- Parent/child and validation-gate behavior has contract coverage and real
  followup/integration evidence; every unrelated task mode was not rerun.
  Standard game export and installer/package acceptance remain outside these
  asset-workflow rounds.

Source/contract pointers for these limits include
`native/core/tests/asset_contract/delivery.rs`,
`native/core/tests/asset_contract/feedback.rs` and
`native/core/src/validation/task_gate_asset_tests.rs`. This record closes the
scoped fix and demonstrated workflows without marking all design scenarios as
fully native-validated.

## Main checkout preservation

Before fast-forward, the original main HEAD was
`693aa781623b865639613e688095c46b41a2b8bb`. Its sole uncommitted path was
`resources/skills/blender-production/references/anime-npr-character.md`, with
SHA-256 `cc47f6adcb9863ed6415247cf1a01ed92ec93799e06f349df8bdb3a457a9c872`.
The integration candidate does not change this path relative to that main HEAD.
The final merge proof records the before/after branch heads, dirty state and
file hash. The `beaver2` branch and worktree are retained.

The v0.4 follow-on starts from `977b788ff28facdfa2045ed4a294515d3d2cbbdc`.
It commits only the scoped asset-tool error fix, its regression/instructions and
these two evidence documents (seven paths). The fast-forward and unchanged
concurrent-edit hash are recorded in `output/asset-v04-main-merge-proof.json`.
