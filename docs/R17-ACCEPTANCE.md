# R17 production prompt refinements

Recorded on 2026-09-11 (Asia/Shanghai), while continuing the character work in
[R16](R16-ACCEPTANCE.md). The native package and its fresh blank-project host
startup are verified. Character work continues in the original R16 round.

Final status: the retained character round is now complete and accepted under
[R19](R19-ACCEPTANCE.md). The sections below preserve the R17 checkpoint.

## Changes grounded in the character review

Only `resources/skills/blender-production/SKILL.md` changed in this increment:

- Keep the intended jaw-to-neck break readable when the mesh is continuous.
- Inspect hair roots and hairlines from both sides, including under accessories,
  for unintended scalp exposure and flat-cut joins.
- Diagnose isolated face-shadow spots with changing light and outline comparison;
  inspect the face map or UVs before reshaping already-correct geometry.

These are conditional visual checks, not a prescribed character design or a
fixed topology or shadow-mask algorithm. They came from actual Beaver-rendered
images and ordinary feedback in the ongoing face and hair tasks. No game assets
or game implementation were authored by the acceptance operator.

## Package and fresh checks

Package: `release/Beaver-native-0.1.19-preview-r17-win32-x64/Beaver.exe`.

- EXE: 13,400,064 bytes.
- SHA-256: `69049878275224df237a6da3049fbaa6090e095594aa4fb468d2d40875a3ee63`.
- Complete package: 606 files, 17,012,309 bytes.
- Skill SHA-256: `45d0ee04e4673d033ca40695eaef62326a9e3883832c4a0edc67e44069599f02`.

`quick_validate.py` and Prettier passed for the changed skill. The full native
build and package were produced. The build tool transport timed out while the
build continued; afterward, a fresh `cargo build --release --locked -p
beaver-desktop` returned 0 and `verify-native-release.ts` returned 0. The changed
skill is UTF-8 without BOM and occurs byte-for-byte in the packaged executable.
The existing R16 release and active process were retained during this build.

## Fresh host startup verification

During the final character task's model-service backoff, R16 had no running or
queued execution. Its parent plan was paused through the API before switching
hosts. `scripts/start-fresh-native-test.ps1` closed the old R16 host and verified
R17 as the only Beaver process (PID 46052).

The round started with zero projects and zero tasks, then created one blank
project through the API. Its files are exactly `export_presets.cfg`, `main.tscn`
and `project.godot`; the post-start API state has one project and zero tasks.
The executable hash matches the package hash above.

Proof:
[`start-proof.json`](../output/validation/fresh-round-20260911-031158-dc309c11d145415e906418500d4dae00/start-proof.json).

The subsequent R18 fresh round closed R17 before starting. The original R16 data
and accepted stages were then restored for continued character validation.
The release manifest retains `runtimeVerified: false`; the independent proof
establishes host startup and blank-project creation, not full product acceptance.

The R16 character review is continuing through its original Beaver API and data
directory. R17 prompt changes are intended for subsequent executions; this record
does not claim that the running R16 task received them or that one character
establishes reliable first-pass artistic quality.
