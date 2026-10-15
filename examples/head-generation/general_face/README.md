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

## Scope and pending acceptance

Pure-math, aliasing, contract and packaging tests run without Blender:

```sh
python -m unittest discover -s examples/head-generation/general_face -v
```

Native Blender execution, actual object frames/custom-normal APIs, editable and
runtime visual inspection, neck/hair fit, mouth clearance, GLB round-trip and
front/side/back light-sweep checks are still required. Global scaling can move
face boundaries relative to the unchanged body and hair. Positive determinant
prevents affine folds but does not guarantee those external clearances.
Legacy report geometry measurements describe the pre-transform generation;
`general_face.runtime_measurements` and editable measurements describe new bounds.
The report deliberately leaves visual approval false. This module changes no
upstream checkpoint or NPR source and performs no commit, deployment or release.

The inherited editable cage is saved before 617 restoration and final runtime
refinements. It is not an exact counterpart of the final runtime surface.
`aster_head_runtime_morph.blend` is the authoritative editable final geometry.
Reports retain explicit pre-transform clearance scope, min/max singular values,
and conservative transported bounds sigma_min × baseline. Requested margins
(including 0.35 mm) are not final measured minima. Historical samples may precede
final refinements; bounds are conditional on those same face sets and pose, and
do not cover unchanged body/hair. Actual mesh axis ratios and finite custom
normals are asserted. MouthOpen 0, 0.25, 0.5 and 1 still require validation.

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
position deltas, and transformed direction endpoint differences for morph normals
and tangents. Changed accessor bounds are recalculated from serialized float32
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
