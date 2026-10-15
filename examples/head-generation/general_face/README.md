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
outputs receive optimistic hashes when `--workspace` is supplied; use the current
project snapshot for the SDF step, which updates definition and generation report.

## Scope and sampled acceptance

Pure-math, aliasing, contract and packaging tests run without Blender:

```sh
python -m unittest discover -s examples/head-generation/general_face -v
```

### Validation record: 2026-10-15 (before morph-interpolation correction)

The native and visual evidence in this section predates the shared-base-scale
morph-direction correction below. It does not validate the corrected native
artifacts, whose regeneration and visual checks are pending. The corrected source
passes 20 pure tests, including intermediate-weight direction checks.

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

## Precise GLB attribute transport

Blender re-encodes custom normals and regenerates tangents/morph normals during
export. The diagnostic `general_face_export.glb` therefore remains separate from
`general_face_transported.glb`, the delivery Face stream. The latter is an affine
transport of `general_base_export.glb` generated earlier in the very same native
job. It never reads accepted final geometry. The resulting Face is merged into
the original body/hair GLB. The runtime blend still contains the native transformed
geometry and shape keys; its shading serialization can differ from delivery GLB.

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
