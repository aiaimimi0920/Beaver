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
  or connectivity changes. The cheek guide preserves the inner mouth aperture and
  ear attachment boundaries and distributes transverse curvature toward the
  nasal region; it does not create a high-valence nose-tip vertex.
- Ear geometry separates main/smooth-cap continuity from creased return walls.
  The fit preserves its fixed source anchors and keeps the existing bounded
  displacement, orientation, diagonal and area guards. The opt-in upper-front
  refinement requires SciPy in the native Blender Python environment; installing
  it on a separate host interpreter is not evidence of native availability.
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

Early native source iterations on 2026-10-26 generated 920 skin/ear quads. The retained
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

The opt-in eye chart also rounds the shared rectangular support locally with
a compact C1 field, while retaining the original aperture reference and fixed
head rim. Its four radial intervals are redistributed to 0/.25/.5/.75/1.
The upper lip's two internal samples move toward the existing peak and shoulder
with an angle-dependent fade; the lower-lip samples, aperture and outer boundary
remain fixed. This is sampling of the existing depth function, not additional
lip-depth smoothing or an increase of the reviewed support bound.

Actual experimental saved-mesh checks preserved both eye apertures and all
30 shared oral aperture points and their morph deltas. The upper-lip first
radial gap decreased from 13.723 to 4.803 mm. Both lower internal rings remained
unchanged. Eye-frame XZ displacement was about 2.02 mm, but reevaluating depth
on the existing surface produced up to 5.97 mm total 3D displacement; do not
describe this as a 2 mm spatial bound. Source generation and sampled mouth
poses passed their checks, and nine actual engine pose captures were verified.
These checks do not remove the remaining upper-lip crest/valley or establish
continuous-domain collision safety or final artistic approval.

### Local upper-lip depth and outer-ring transition

The opt-in source path now keeps the first upper support ring at its earlier
peak-sampling location, but uses a compact C1 upper-half window for the second
and third rings. One additional closed support ring splits the long outer
span. It does not globally densify the head or reconstruct reference quads.

Depth fairing is limited to internal upper-mouth samples: the inherited
reference and calibration remain unchanged, the total local depth difference
is capped at 5.2 mm, and the additional fairing move at 1.5 mm. All other depth
calls retain their existing reviewed bound (4.2 mm in the checked profile).
These are depth differences, not limits on total three-dimensional vertex
displacement. Non-finite values fail closed. The new lower-half support points
use midpoint depths, and those final interpolated depths are checked against
the original lower-region bound rather than checked only before replacement.

Saved-model checkpoints using these adapters retained the oral aperture and
added 30 source points/quads, producing 950 skin/ear quads. One central 15.192 mm
span became 7.139/8.114 mm. The following outer-ring transition reduced the
measured central-versus-shoulder height difference from 2.964 to 0.176 mm.
Only 28 moving mouth points required new UV and morph values in that step;
fixed oral points and unrelated mouth fields remained unchanged. Independent
81-pose CPU checks did not find degenerate or reversed sampled triangles.

A real nine-pose engine capture was verified for the added-ring checkpoint,
not for every later shape. Subdividing a nonplanar lower quad does not preserve
its triangulated surface exactly: sampled distances reached 0.117 mm near a
mouth corner. Narrow ear regions, concentrated nasal turns and final visual
likeness remain open. None of these measurements, source tests or CI results
constitutes artistic, global-collision or deployment acceptance.

### Rounded outer mouth frame and bounded upper-ear support

The mouth outer frame now uses a compact C1 source-chart rounding field.
Amplitude 0.12 and a broad lateral fade of 0.65 distribute the transition across
nearby cheek columns rather than leaving the next column rigid. The central
nasal strip, eye/ear attachment heights and original inner aperture references
stay outside the field. This changes sampling coordinates, not the analytic
nasal depth function, and adds no vertices or faces. The disabled-strength
path remains exactly the identity. Actual saved-mesh checks found 80 moving
skin points, at most 4.592 mm, with UV changes restricted to those points.
Sampled exported mouth poses passed; two unchanged ear-root points had about
1.2 micrometers difference in one MouthOpen component, so unrelated morph
buffers are not claimed byte-identical.

The upper-ear refinement keeps connectivity and authored UV weights. It holds
the previously repaired three-quad attachment patch fixed while allowing one
upper outer support point a separate 1 mm design limit. The remaining 18 fixed
points and all ear roots stay fixed. Existing free points retain their prior
scale-relative displacement limit; finite-value and float32 post-validation
checks retain the original diagonal, orientation and area gates. The tested
outer support moved about 0.886 mm, without moving the attachment patch.
The narrow cap's maximum required-view angle decreased, but an adjacent front
face became narrower in one view. This is a documented local tradeoff, not a
claim that every angle improved or that the ear matches the reference exactly.

The authoring recipe must retain the ear refinement module and its SciPy
runtime dependency. Source quads remain editable; the exporter automatically
selects tested diagonals. Passing the sampled exported mesh does not establish
that arbitrary retriangulation is safe. Inner-lip support concentration and
final same-camera visual likeness remain separate open review items.

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

## Joint first-support-ring refinement

The source recipe now solves the first mouth support ring jointly on the current
semantic rays. It does not use historical vertex IDs or a saved solution table.
The aperture, midline endpoints, second and third rings, and outer boundary stay
fixed. Upper first-ring fairness uses 0.33, while pinned midline points retain
the previous value and the other upper rings retain 0.35.

The optimizer reads a pure pair of current depth and the unchanged inherited
reference. Every accepted point is checked again through the strict depth
function before any vertex is written. The existing 4.2 mm raw-depth gate,
upper-local 5.2 mm total-reference bound and 1.5 mm additional move bound remain.
Ring gaps, projected orientation, both 3D diagonals and float32 output are checked.
An infeasible solve raises rather than partially updating the ring.

In the native process comparison, only 28 first-ring points changed. Upper
shoulder height above the intended ridge decreased from about 6.422 to 0.618 mm.
The mouth-corner turn increased from 29.905 to 30.885 degrees, within the explicit
one-degree limit. This does not fix all outer transitions: turns of about 40.6
and 44.5 degrees remain. Sampled export pose checks passed, but these checks and
the source comparison are not a claim of final visual acceptance.
