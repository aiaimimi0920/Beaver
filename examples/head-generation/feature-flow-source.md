# Experimental quad source flow

This is an opt-in authoring path, not a visually approved character preset.
The ordinary composition recipe remains the default. No reference mesh, UV,
texture, or private project artifact is included in this change.

## Supported input and execution

Use the six-component composition builder with `sampling_feature_flow=1`.
The versioned contract requires density sampling enabled, 13 shell columns,
detail budget 1, and subdivision, surface fairing and outline refinement 0.
Other profiles fail closed rather than partially applying the adapters.

Call `face_components.bundle.build_plan(..., source_seed=True)` or its CLI
with `--source-seed` and a sampling-controls JSON file. This explicitly selects
three native jobs; the default recipe still uses two. `build_request` rejects
seed mode because it cannot carry the complete receipt-bound plan.

1. Run the emitted source-seed request through the existing Beaver workflow.
2. Preserve its original successful receipt and call
   `source_layout.seed_evidence.finalize_seed_plan` before running generation.
3. Preserve the generation receipt and use `packaging_request` for the source
   packaging job. Do not rebuild a finalized plan or manufacture receipts.
4. Verify the full seed/generation/packaging chain before downstream SDF work.

Full serialized native requests remain limited to 1 MiB. Transport sharing
deduplicates identical readable source literals; it does not compress code,
increase the native limit, or silently introduce another job.
Owned static source outputs, seed files and packaging helpers use explicit
UTF-8/LF writes. A CPU simulation of Windows text defaults verifies byte/hash
stability; this is not a native Windows execution claim. Copied receipts retain
their original bytes.

## Source responsibilities

- `feature_flow_recipe.py` owns adapter order and retained source inventory.
- Feature, mouth, chin, eye and lateral-band modules own their named sampling
  or connectivity changes. The cheek guide preserves the existing mouth and
  ear attachment boundaries and distributes transverse curvature toward the
  nasal region; it does not create a high-valence nose-tip vertex.
- Ear geometry separates main/smooth-cap continuity from creased return walls.
  The fit preserves its fixed source anchors and keeps the existing bounded
  displacement, orientation, diagonal and area guards.
- Weld ownership captures numeric point attributes and named group weights
  before joining. Only matched skin/oral boundary vertices receive skin donor
  fields. Positions, loop-domain UVs and oral-only points are not overwritten.
  Ambiguous matches or incompatible object axes/scales fail closed.
- Editable skin and ears remain quads. Export chooses a common valid triangle
  split for neutral/open geometry; triangles are an export/runtime concern.

## Replaying retained source

The first generation executes complete inline source. Packaging writes the
separate readable modules and their hash-bound reconstruction record.
For the ordered face/shell/ear families, use
`source_layout.module_layout.execute_ordered_module(entry_path, record, namespace)`.
It validates the entry, ordered helpers and reconstructed source before executing
the verified in-memory snapshots in one shared namespace.

Legacy checkpoint loaders that use a bare `ModuleType` without a filesystem
location cannot directly execute these split entry files. Do not hide this
limitation by bypassing hashes or falling back to unverified source.

## Evidence and remaining work

Native source iterations on 2026-10-26 generated 920 skin/ear quads. The retained
source refactor was checked against the prior model: all 55 meshes had matching
positions, topology, UVs, point attributes and deform weights, and runtime GLB
bytes were identical. Later source-flow and ear corrections intentionally change
geometry, so that earlier equality is not evidence for those later shapes.

The later cheek guide removed the measured outward turnback and distributed
curvature across two traced bands. Ear source fitting reduced the measured
uncreased upper-cap fold and the nearly straight lower-quad corner. Original
ear-corner UVs and crease ownership were checked separately from new interpolated
split-point UVs. These are bounded geometric findings, not artistic acceptance.

Known remaining issues include a narrow upper-ear cap, the eye outer-frame
appearance and final same-camera expression/runtime review of the latest shape.
Successful generation, source tests, or CI do not certify visual completion,
absence of warnings, deployment, or final product acceptance.

The later public-recipe regression also matched the corrected experimental
model across all 55 source meshes and the complete runtime GLB bytes. Source
packaging reconstruction and the original three-job receipt chain were verified
independently of that geometric comparison.

The subsequent upper ear-to-skin attachment revision keeps the original 40 ear
points, their corner UVs and crease groups unchanged. It moves three attachment
points and one adjoining skin point per side, retaining all source face indices.
Actual saved-mesh checks found the affected attachment's largest 3D corner
reduced from about 155 to 135 degrees, with right-side and right-45-degree
projection maxima reduced from 162.56/165.59 to 139.21/126.71 degrees.
Both six-quad skin patches retain two-face incidence along the shared split
seams; new corner UV interpolation errors were below 3.1e-8.
Native generation, source packaging and opaque source-wire three-view rendering
completed for this revision. This does not certify global self-intersection
absence or final runtime/artistic acceptance. The adjacent front smooth cap,
eye outer-frame and mouth faceting remain unresolved.

The shared oral weld now restores skin-owned crease EDGE fields as well as
POINT fields. Duplicate skin/cavity edges previously made `crease_edge` and
`FacialCreaseGroup` ownership depend on weld ordering. Capture requires a
unique pre-weld skin/cavity boundary edge; restoration requires exactly two
mixed-owner incident faces and a bijective endpoint donor correspondence.
Only these shared edges are written. Native validation in the subsequent
experimental mesh confirmed all 30 shared oral edges retained the skin donor's
crease/group values; this is an attribute-ownership result, not a claim that
the experimental eye or lip shape has final artistic acceptance.

Focused CPU checks, from this directory:

```sh
python -m unittest discover -s source_layout -p 'test_*.py'
python -m unittest discover -s semantic_eyes -p '*flow.py'
python -m unittest discover -s semantic_eyes -p 'test_weld_ownership.py'
python -m unittest discover -s semantic_eyes -p 'test_ear_attachment.py'
python -m unittest discover -s face_components -p 'test_feature_flow_recipe.py'
python -m unittest discover -s face_components -p 'test_generated_source_size.py'
```

The ownership fixtures are CPU mocks, not a replacement for native attribute
and pose checks. The Python effective-line counter treats multiline data/code
strings as code and is conservative; it does not replace the repository's
`npm run check:effective-lines` Rust scanner. Record an unavailable scanner as
unrun/blocked rather than treating these tests as its result.

## Design references

- [Autodesk: Modeling the Head](https://download.autodesk.com/global/docs/softimage2014/en_us/userguide/files/face_modeling_ModelingtheHead.htm)
  discusses facial edge direction and deformation. Software-specific budgets
  are not adopted as this recipe's acceptance criteria.
- [Disney Research: Sketch-Based Generation and Editing of Quad Meshes](https://la.disneyresearch.com/publication/sketch-based-generation-and-editing-of-quad-meshes/)
  motivates controllable patch boundaries and edge flow, not a unique facial
  template or automatic recovery of original quads from triangulated assets.

Shape, edge direction, connectivity, shading and deformation must be evaluated
separately. Quad-only authoring by itself does not fix any of the latter four.
