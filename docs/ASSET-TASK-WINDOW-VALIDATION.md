# Asset task window validation

Recorded on 2026-09-13 (Asia/Shanghai). Implementation is isolated in
`C:\Users\Public\nas_home\beaver2`, branch `beaver2`, based on
`393e2aa1c345312d1f6df85c9d1abe71c59d8f81`.

The integration candidate includes main checkpoint
`693aa781623b865639613e688095c46b41a2b8bb`. Engineering checks and the final
native merge smoke passed. Real production acceptance is partial; this record
does not claim all design scenarios passed. This is the pre-commit validation
checkpoint. The final branch update and preserved concurrent edit are recorded
in `output/asset-main-merge-proof.json` after the fast-forward.

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

The release executable is `target/release/Beaver.exe`, 16,592,384 bytes, last
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

## Remaining product acceptance

The open scenarios in
[the design, section 9](ASSET-TASK-WINDOW-DESIGN.md#9-验收标准) include:

- Hair suspension, completed nose-feedback verification and hair resumption.
- Clothing-dependent changes at `askRatio` 100, 0 and an intermediate threshold.
- One-mesh targeting, renaming, instances, modifiers/topology and actual high-DPI
  annotation behavior; broader busy/frame-age and latency coverage.
- Two-task isolation, deferred feedback through completed delivery, the full late
  followup lifecycle and uncertainty recovery after abnormal shutdown.

These remain unverified in real production even where engineering tests cover
the underlying rules. Use a new blank project for each further major round.

## Main checkout preservation

Before fast-forward, the original main HEAD was
`693aa781623b865639613e688095c46b41a2b8bb`. Its sole uncommitted path was
`resources/skills/blender-production/references/anime-npr-character.md`, with
SHA-256 `cc47f6adcb9863ed6415247cf1a01ed92ec93799e06f349df8bdb3a457a9c872`.
The integration candidate does not change this path relative to that main HEAD.
The final merge proof records the before/after branch heads, dirty state and
file hash. The `beaver2` branch and worktree are retained.
