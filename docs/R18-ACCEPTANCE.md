# R18 topology and shading prompt checks

Recorded on 2026-09-11 (Asia/Shanghai) while completing the existing
[R16 character refinement](R16-ACCEPTANCE.md). The native package and fresh
blank-project startup are verified. The existing character round continues in
R16 for its final validation and export.

Final status: the retained character round is now complete and accepted under
[R19](R19-ACCEPTANCE.md), which includes the unchanged R18 prompt refinements.
The sections below preserve the R18 checkpoint.

## Prompt change

The only manual production-source change in this increment is four lines in
`resources/skills/blender-production/SKILL.md`:

- After topology or thickness edits, inspect bounds for stray vertices and check
  the exported normals.
- Recompute stale custom normals on rebuilt regions while preserving intentional
  normals elsewhere.
- When folds look flat, compare normals and the material light/shadow response
  before adding geometry.

These checks came from actual events in clothing task
`6ca90f57-049d-4cf0-b182-c3370a4e89cf`: a thickness spike changed the model bounds,
and inherited custom normals plus the cloth ILM flattened the visible folds.
The managed task repaired these issues and produced passing validation and
reviewed images. No particular thickness, topology algorithm or costume design
was made mandatory. The R17 face, hair-root and shadow guidance remains included.

The ongoing character tasks run in the retained R16 executable. Their results
do not establish behavioral effectiveness of the newly packaged R18 prompt.

## Package and verification

Package: `release/Beaver-native-0.1.19-preview-r18-win32-x64/Beaver.exe`.

- EXE: 13,400,576 bytes.
- EXE SHA-256: `292f58bcd21856a978ec1b367f2cff1fcc2f77d17a9639c6233c85fb11aa5d48`.
- Complete package: 606 files, 17,012,821 bytes.
- Skill SHA-256: `875669747a8b138e43a793e946c2052fb22b33807cfc70dc5e7228e473284cb3`.

The skill passed `quick_validate.py` and Prettier. It is UTF-8 without BOM and
occurs byte-for-byte in the packaged executable. The initial full build tool
transport timed out after 300 seconds; a fresh
`cargo build --release --locked -p beaver-desktop` then returned 0. Packaging
through `package-native.ts` returned 0 and ran `verifyNativeRelease` against the
new package, including its complete file inventory. Existing release directories
were retained.

## Fresh host startup verification

While the final R16 task was stopped by a model-service error, its parent plan
was paused through the API. After the separate R17 startup check,
`scripts/start-fresh-native-test.ps1` closed R17 and started the packaged R18 as
the only Beaver process (PID 51052). No character work was active during either
host switch.

The round had zero projects and zero tasks before creating a new blank project
through the API. Its initial files are exactly `export_presets.cfg`, `main.tscn`
and `project.godot`. A fresh API read confirmed one project and zero tasks, and
the executable hash matches the package hash above.

Proof:
[`start-proof.json`](../output/validation/fresh-round-20260911-031613-5aad2074448f49d9b598560b077f6e66/start-proof.json).

The test R18 process was closed by its verified PID and executable path. The
original R16 round was restored with all three accepted character stages intact.
The release manifest retains `runtimeVerified: false`; this independent startup
proof does not claim full product, migration or artistic-quality acceptance.

The ongoing continuation is recorded in
[`npr-r16-completion-proof.json`](../output/validation/npr-r16-completion-proof.json).
This work does not establish reference-level art quality or reliable first-pass
character production.
