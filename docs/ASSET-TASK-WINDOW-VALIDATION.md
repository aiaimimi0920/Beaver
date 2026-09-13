# Asset task window validation

Recorded on 2026-09-13 (Asia/Shanghai). Implementation is isolated in
`C:\Users\Public\nas_home\beaver2`, branch `beaver2`, based on
`393e2aa1c345312d1f6df85c9d1abe71c59d8f81`.

Status: engineering checks passed; native acceptance and merge into `main`
remain pending. This record does not claim the design's acceptance scenarios
have passed.

## Completed checks

Commands ran in the normal PowerShell toolchain environment with the existing
MSVC `LIB` preserved. Log paths below are relative to this checkout; generated
logs and artifacts remain in the ignored `output` directory.

| Check                                                                                                            | Result                             | Evidence                                                               |
| ---------------------------------------------------------------------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------- |
| `rtk proxy npm run build:native`                                                                                 | Release build passed, 7m 07s       | `output/asset-native-build.log`                                        |
| `rtk proxy cargo test --locked -p beaver-core --test asset_contract`                                             | 20 passed                          | `output/asset-contract-test.log`                                       |
| `rtk proxy cargo test --locked -p beaver-core --lib task_`                                                       | 20 passed                          | `output/asset-regression-task.log`                                     |
| Core filters `autonomy::`, `clarifications::`, `codex_home::`, `blender_session::`, `scheduler::`, `assets::`    | 13 passed                          | `output/asset-regression-*.log`                                        |
| `rtk proxy cargo test --locked -p beaver-desktop`                                                                | 5 passed                           | `output/asset-desktop-test.log`                                        |
| `rtk proxy npm run test:effective-lines`                                                                         | 7 unit and 4 contract tests passed | `output/asset-effective-lines-test.log`                                |
| `rtk proxy npx tsx --test tests/asset-preview.test.ts tests/asset-feedback-draft.test.ts tests/autonomy.test.ts` | 12 passed                          | `output/asset-ui-test.log`                                             |
| Asset preview and draft tests after strict TypeScript fixture correction                                         | 10 passed                          | `output/asset-ui-focused-test.log`                                     |
| `rtk proxy python -m unittest discover -s tests -p 'asset_observer*_test.py'`                                    | 10 passed                          | `output/asset-observer-test.log`                                       |
| `rtk proxy npm run typecheck`                                                                                    | Passed                             | `output/asset-typecheck.log`                                           |
| Official Prettier check on changed/new frontend and test files                                                   | Passed, 20 files                   | `output/asset-format-check.log`                                        |
| `rtk proxy cargo fmt --all -- --check`                                                                           | Passed                             | `output/asset-cargo-format.log`                                        |
| `rtk proxy npm run check:effective-lines`                                                                        | Passed, no violations              | `output/asset-effective-lines.log`, `output/effective-code-lines.json` |

The release executable is `target/release/Beaver.exe`, 14,501,376 bytes, last
written at `2026-09-13T05:19:21.323Z`. Its SHA-256 at this checkpoint is
`e2095edda66416b4c8ffabbf65681a0ef3bef50363998b732e1c9020ba71e834`.
The native acceptance launcher must independently record the selected executable
hash, PID, fresh directories and blank project in its own `start-proof.json`;
the build artifact identity does not establish those runtime claims.

PowerShell captured some native stderr as `NativeCommandError` metadata in logs.
Results above use process exit codes and the actual test runner summaries.

The final structure pass scanned 385 sources with zero violations and 19 unchanged
legacy files. The 67 changed, non-vendored source files have at most 447 effective
lines (`native/core/src/codex_home.rs`). All 77 changed, non-vendored text files
were checked for BOM markers; none were found. The final `git diff --check` and
scoped Markdown Prettier check passed; logs are `output/asset-diff-check.log` and
`output/asset-doc-format-check.log`.

## Scope of the evidence

The Rust contract tests exercise stage CAS/DAG validation, suspension/resumption,
clothing revalidation, image integrity and rendered annotations, immutable picks,
idempotent submissions, cancellation, delayed production rounds, uncertain
relative edits, finalization races, followup isolation and crash repair. The
dynamic tool tests check real PNG `inputImage` response content and stale
thread/turn rejection.

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

## Native acceptance still required

Use `scripts/start-fresh-native-test.ps1 -Build -Executable
target/release/Beaver.exe` once the original Beaver runtime is available to close.
Supply a random API token through the environment, never command arguments or
evidence files. Start a new blank project; drive creation through Beaver APIs
only. Retain each major round as separate evidence.

The required scenarios remain those in
[the design, section 9](ASSET-TASK-WINDOW-DESIGN.md#9-验收标准), including:

- Unsaved scene preview, independent camera operations, busy/frame-age behavior
  and measured interaction latency on the actual Blender/GPU version.
- Real local picks on one mesh, renaming, instances, modifiers/topology and stale
  rejection; reference annotations after resize and display scaling.
- Hair in progress, "鼻子小一点", safe suspension, checked local edit and hair
  resumption; actual image consumption by Codex.
- "腿长一点，胸大一点" with actual clothing dependencies and feasible choices
  at `askRatio` 100, 0 and an intermediate threshold.
- Deferred feedback across turns, awaiting-input session retention, close/reopen,
  two-task isolation, restart/reload uncertainty and independent late followups.

At the preparation checkpoint, PID 12240 was running from the original
`Beaver\release\Beaver-native-0.1.19-preview-r23-win32-x64\Beaver.exe`.
It was left running to preserve the concurrent development workflow. No native
acceptance round has been launched from this branch, and no merge has occurred.
