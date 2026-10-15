# Explicit global face proportions v1

This isolated experiment exposes bounded global face proportions, not arbitrary
identity generation, semantic topology synthesis, age control, or local feature
editing. `prompt_contract.py` contains the reusable Chinese contract. Selection
is deterministic: explicit preset or numeric proportions; unknown intent fails.

- `baseline`: width 1, height 1, depth 1
- `soft_wide` / `柔和宽脸`: width 1.06, height 0.96, depth 1
- `slender` / `修长脸`: width 0.94, height 1.06, depth 1
- Width bounds 0.92–1.08; height 0.94–1.06; depth 0.96–1.04

The positive diagonal affine map uses Blender x/y/z = width/depth/height.
Its fixed pivot is (0, 0, Y0 + H/2 − 0.021), currently z=1.641 m. It preserves
bilateral symmetry and invertibility; mouth deltas use the linear map and custom
corner normals use its inverse transpose. Baseline geometry is a strict no-op.
Legacy calibration objects are never modified.

## Native jobs

These commands package JSON requests only. Submit the resulting requests through
Beaver's authorized native Blender job workflow; do not run Blender directly.

```sh
python examples/head-generation/general_face/bundle.py \
  --intent '{"preset":"soft_wide"}' \
  --prefix assets/aster/general_face_soft_wide/ --output /tmp/general-face.json
```

The wrapper reuses current `build_request` with a private source snapshot. It
freshly builds from the original 49 inputs plus the existing 617 preservation
step. No accepted final 662 or 664 geometry is read. It then maps the runtime Face
mesh, Basis/MouthOpen, delta attribute, normals, and exact reported editable Face
parts. Body and hair fingerprints must stay unchanged. Face-only exports are
retained as declared job outputs, and the merged GLB keeps original body/hair
binary data. All copied recipe sources and adapted pipeline manifest are retained.

After the geometry job succeeds, build and submit its matching SDF request:

```sh
python examples/head-generation/general_face/sdf_bundle.py \
  --intent '{"preset":"soft_wide"}' \
  --prefix assets/aster/general_face_soft_wide/ --workspace /path/to/project \
  --output /tmp/general-face-sdf.json
```

The SDF job requires matching generation intent and GLB hash. It uses the bundled
NPR distance transform/binding implementation, remapping proxy normals by
Godot diag(width,height,depth) inverse transpose before the lighting dot product.
UV2 remains fixed. Nonidentity profiles always get a fresh atlas; old atlas reuse
is unsupported. Adapted bake source and all helper sources are retained. Existing
outputs always receive optimistic hashes: `--workspace` is mandatory. Use the current
project snapshot for the SDF step, which updates definition and generation report.

## Scope and sampled acceptance

Pure-math, aliasing, contract and packaging tests run without Blender:

```sh
python -m unittest discover -s examples/head-generation/general_face -v
```

### Current engine validation: 2026-10-15, node-affine policy

Native run `c2858ee8-cb86-4c3c-9ba7-c9a2a95c71b9` passed 8 Godot/GUT tests
and 338 assertions. All three profiles were checked against the actual imported
same-job baseline at MouthOpen weights 0, 0.25, 0.5, 0.75, and 1, across every
vertex. The native `morph_node_affine_report.json` records the imported arrays,
relative node transforms, input hashes, and per-weight errors. The source passes
27 pure tests. Fresh node-affine visual acceptance is recorded below. These numerical checks
are engine-array evidence, not a GPU-normal capture.

The earlier 668 raw-stream correction failed actual engine-import probes despite
passing source algebra checks: Godot normalizes shape-target direction endpoints,
losing magnitudes needed by that representation. That failed result is retained
as historical evidence, not relabeled as passing. The 669 node-affine policy
supersedes it with the explicit affine-of-actual-imported-baseline contract below.
It does not promise recovery of raw endpoint lengths already lost in import.

### Final bounded visual acceptance: 2026-10-15

Final native run `4599e539-a23b-4d1e-b1ae-4524e0322b3b` passed 8 tests and
356 assertions, including explicit absence of optional performance/FaceRig
profiles and eye layers. The three native imported-baseline/node-affine cases
remained passing. A preceding visual validation run
`3c08a0b7-3da7-4b77-b4da-11d9cd88f9ce` also passed 8 tests/356 assertions.

Fresh 669 baseline, soft-wide and slender artifacts were used for visual review.
Each variant has 47 indexed paired captures (94 total), covering eight yaw
angles in render/white/wireframe modes, pitch 55 degrees, MouthOpen weights
0/0.25/0.5/0.75/1 at yaw 0/45/90 degrees, and four light directions at yaw 0/45.
No new sampled facial silhouette, lash attachment, mouth or lighting defect was
observed relative to the freshly generated baseline. Actual rendered captures
were inspected; no claim of a separate GPU-normal capture is made.

Acceptance covers the explicit head-only review path and direct imported
MouthOpen use. Full wardrobe/performance-driven morph rebuilding and optional
FaceRig eye/lid layers are unsupported by this example. Unchanged hair remains
outside head-only acceptance, including visible forehead intersection, and the
open-back face shell is inherited. The samples are not exhaustive clearance proof.

### Validation record: 2026-10-15 (before morph-interpolation correction)

The native and visual evidence in this section predates the shared-base-scale
morph-direction correction below. It does not validate the corrected native
artifacts, whose regeneration and visual checks are pending. The current source additionally uses the node-affine runtime policy described
below; earlier artifacts and their failures remain separate evidence. The
current source passes 27 pure tests, including node/merge and direction checks.

Three native generation jobs and three matching fresh SDF jobs completed with
exit code 0. The real Godot/GUT run passed 7 tests and 234 assertions, including
runtime dimensions, mouth keys, UV2, and SDF/report hashes. Pure regressions
passed: 19 general-face, 106 head-generation, 362 current-checkpoint, and 9 SDF
tests. Three preview path/containment tests also passed.

Native MiDot visual inspection compared both `soft_wide` and `slender` against
the actual generated baseline: 47 paired captures per variant, 94 total. Samples
covered eight yaw directions in render/white/wireframe modes, pitch 55 degrees,
MouthOpen 0/0.25/0.5/1 at yaw 0/45/90 degrees, and four light directions at yaw
0/45 degrees. No new facial silhouette, lash attachment, mouth, or lighting
regression was observed in these samples.

This is scoped acceptance of the bounded head-only experiment, not whole-character
acceptance or exhaustive clearance proof. Unchanged hair visibly intersects the
forehead, and the open-back face shell is inherited. Arbitrary local-feature
editing and natural-language identity generation remain unsupported. Global
scaling can move face boundaries relative to the unchanged body and hair;
positive determinant prevents affine folds but does not guarantee those external
clearances. Editable final-surface inspection and full-character fit require
separate acceptance when those are delivery requirements.

Legacy report geometry measurements describe the pre-transform generation;
`general_face.runtime_measurements` and editable measurements describe new bounds.
Generated reports deliberately leave visual approval false: job execution cannot
approve its own appearance. This dated record captures the separate sampled
visual review and does not change that generator behavior. This module changes no
upstream checkpoint or NPR source and performs no commit, deployment or release.

The inherited editable cage is saved before 617 restoration and final runtime
refinements. It is not an exact counterpart of the final runtime surface.
`aster_head_runtime_morph.blend` is the authoritative editable final geometry.
Reports retain explicit pre-transform clearance scope, min/max singular values,
and conservative transported bounds sigma_min × baseline. Requested margins
(including 0.35 mm) are not final measured minima. Historical samples may precede
final refinements; bounds are conditional on those same face sets and pose, and
do not cover unchanged body/hair. Actual mesh axis ratios and finite custom
normals are asserted. MouthOpen 0, 0.25, 0.5 and 1 were visually sampled as
described above; these samples do not prove exact clearance at every pose.

Some inherited replacement lash cages have neither shape keys nor a
`MouthOpenDelta` attribute. Their absence is preserved and reported as
`mouth_delta_present: false`; no mouth motion is invented. Any present delta must
be POINT-domain. A mesh with shape keys requires the delta; runtime Face always
requires both Basis/MouthOpen and the delta, including the identity preset.

## Runtime Face-node affine policy

Runtime delivery now explicitly uses `face_node_affine_imported_baseline_v1`.
`glb_node_affine.py` preserves every mesh/accessor/buffer byte from the same native
job's fresh `general_base_export.glb`. It sets only the Face node to Godot scale
S=diag(width,height,depth) and translation c−S*c, with c=(0,1.641,0). The original
49 Body/Hair nodes, mesh records and binary prefix remain frozen. The merge wrapper
explicitly propagates the Face transform because the legacy mesh-only merger
does not copy incoming node transforms. Existing nonidentity Face frames, shared
Face mesh instances, or unsupported runtime policies fail.

Godot normal packing normalizes absolute shape-target directions and discards
their lengths. Baking the affine into morph normal streams before import is
therefore not sufficient, even when raw GLB interpolation is algebraically correct.
The node policy instead applies S inverse transpose after the original imported
normal interpolation. Since normalization is homogeneous, this preserves the
affine direction of the engine-imported same-job baseline at every nonzero weight
mixture. It does not claim exact preservation of raw GLB endpoint lengths already
lost by baseline import. Baseline packing error and earlier raw-GLB failures remain
separate diagnostics. Existing generated assets are not retroactively updated.

The native runtime blend continues to hold transformed editable geometry; the
GLB holds equivalent world geometry through a node affine. Strict paired neutral/
MouthOpen position checks compare its mathematical world positions with the actual
native Blender export. UVs and all original local streams are unchanged. Identity
keeps the original Face-only GLB bytes exactly. Fresh proportion-aware SDF baking
still happens once: positive diagonal node scaling preserves normalized world
basis axes, so it does not double-apply the atlas's normal-space correction.

Static integration audit: model validation accepts nonsingular affine transforms;
GEOMETRY maintains local bounds and source world transforms; depth/auxiliary
proxies use source global transforms, shadow proxies inherit the Face node, and
outline mode 0 uses the model-view normal matrix. The current Face shaders apply
the normal matrix to tangent variables too, rather than the mathematically correct
forward tangent matrix. Active Face lighting and outline mode 0 do not consume
those tangent variables. This policy is not a general tangent-shader fix; tangent
shader consumers require separate validation or correction before being enabled.
Native imported-baseline probes and sampled head-only visuals passed as recorded
above. Direct GPU-normal capture was not performed.

## Historical float32 accessor transport (diagnostic helper)

The following helper remains available for strict mathematical and native-world
position comparisons. It is no longer used as the runtime delivery representation.

Blender re-encodes custom normals and regenerates tangents/morph normals during
export. The diagnostic `general_face_export.glb` remains separate from runtime
`general_face_transported.glb`; the latter now uses the node policy above. Both
originate from generation in the same native job, never accepted final geometry.
The runtime blend retains native transformed geometry and shape keys.

`glb_affine.py` reads float32 strided and sparse vector accessors, materializing
implicit zero deltas. It transforms neutral positions, inverse-transpose unit
normals, forward-transformed unit tangents (handedness unchanged), linear morph
position deltas, and linearly transformed direction deltas divided by the same
transformed-base length used to normalize each neutral normal or tangent.
For direction operator L, base b and delta d, delivery stores L(b)/|L(b)| and
L(d)/|L(b)|. Thus normalizing their weighted sum yields normalize(L(b + w*d))
at every weight, including 0, 0.25, 0.5, 0.75 and 1. Independently normalizing the
morph endpoint before constructing its delta would break intermediate weights
and is deliberately avoided. Nonunit endpoint vectors are allowed; zero direction
endpoints are rejected. A zero mixed direction has no defined normalized direction.
The runtime must add the raw weighted direction deltas before normalizing the
result; it must not normalize deltas or endpoint vectors independently. Current
Face shaders normalize the incoming morphed NORMAL for world-space lighting;
engine import/runtime blend order and tangent handling still require native
verification. Identity leaves every original byte, including morph deltas, intact.
This source correction does not update previously generated native assets. Changed accessor bounds are recalculated from serialized float32
values. Indices, UV/color streams, embedded image data and original binary prefix
are untouched. Identity returns the original bytes exactly after validation.
Unsupported node transforms, skinning, animation, geometry extensions, semantics,
accessor component types and malformed ranges fail explicitly.

Before merge, paired base/MouthOpen endpoint records must agree bidirectionally
with the actual native Blender export within 3e-7 m per component. This supports
export vertex splitting without losing morph correspondence or relaxing delivery
normal checks. Sharing a transformed accessor with a preserved color, UV or index
semantic is explicitly rejected. Tests also cover
sparse normal/tangent deltas and rejection of invalid layouts/geometry. This
format-level guard does not replace native visual or topology acceptance.

### Cross-run tangent limitation

Same-job geometric, normal, and tangent affine transport passes, as do the checked
non-tangent neutral invariants. Strict cross-run Face tangent equality still
**fails** at 1–2 upstream fresh-export vertices, with a maximum observed difference
of approximately 1.0002e-4. Do not claim bitwise reproducibility across independent
Blender exports. The current active Face shader, with outline mode 0 and no normal
maps, does not depend on these tangents. This conclusion does not cover future
normal-map use or other tangent-dependent shader/outline modes. The discrepancy
remains documented rather than being hidden by a relaxed equality check.

SDF packaging requires an existing workspace containing `project.godot` and the
selected generation's GLB, definition and report. The function also requires the
workspace argument and rejects missing inputs before emitting a request. Existing
definition/report outputs always carry their current SHA-256; no null-hash overwrite
fallback exists. CLI smoke tests use an isolated temporary input fixture, compile
the resulting code without executing it, and remove all fixture/request artifacts.

## Fail-fast scope guard and reusable native regression

This bounded example supports the head-only imported MouthOpen path. Generation
and SDF packaging reject explicit `face_motion_profile`, `face_atlas_profile`,
`eye_geometry_profile`, and `performance_data_path` assignments, including null.
Full-performance morph replacement and separately authored eye/lid geometry are
outside this contract; the node-affine policy is not general compatibility for
those optional systems.

Build a hash-bound manifest from current generated outputs, repeating `--case`
for additional profiles:

```sh
python examples/head-generation/general_face/native_probe.py \
  --case soft /path/to/generated/output res://assets/aster/general_face_soft \
  --output node_affine_probe.json
```

Stage the manifest at `res://tests/node_affine_probe.json` and the retained
`native/test_morph_node_affine.gd` through an authorized Beaver native validation
request in the NPR/GUT project. Do not launch Godot directly. The probe compares
actual imports of the same-job baseline and final character at weights
0/0.25/0.5/0.75/1, requires exact local arrays before vertex correspondence, then
checks affine world positions and normals. It binds definition, intent, baseline,
and final model hashes and checks the supported profile scope.

CPU unittest discovery validates fixtures, schema, guards and CLI packaging only;
it does not execute Godot or establish native acceptance. The retained reusable
probe has extra schema/hash/scope checks beyond the historical executed probe;
its own native execution must be recorded separately. The current 32 pure tests
pass. No generated models, fixture requests or local acceptance paths are needed
by these reusable source files.

### Recovery and report-bound probe updates

SDF retries accept an already complete owned atlas binding. The adapter validates
its reconstructed unbound form with the upstream binder, then changes only the
owned atlas path; a same-path retry is byte-identical and unrelated bindings are
preserved. Incomplete or ambiguous bindings fail explicitly.

The standalone probe now hashes `generation_report.json` and converts its recorded
`general_face.pivot_blender_xyz` to Godot `(x, z, -y)`, rather than assuming a
fixed calibration. The retained GUT source verifies the report hash and pivot
binding before its unchanged affine comparison. CPU fixtures cover a changed
pivot and normalized color streams. Actual 669 manifests preserve the same
`(0, 1.641, 0)` pivot; this source update does not modify any delivered model.

The preceding retained GUT version was executed in native run
`49fd8400-37aa-4016-b11a-02d809e92a5e`: 8 tests and 364 assertions passed,
with all three cases passing. Later report-binding assertion changes and the
SDF retry adapter require their own recorded verification; this earlier result
is not presented as execution of those new lines.
