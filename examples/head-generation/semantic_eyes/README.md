# Procedural eye-family experiment

This packages source and numeric controls for the existing Beaver `blender.start`
workflow. It does not run Blender or Godot itself. A valid request or passing CPU
tests do not establish visual quality, freedom from intersections, or a generally
accepted character-generation pipeline.

## Inputs and scope

The procedural Face is regenerated, with no restoration from a final accepted
Face mesh. The private `head_recovery_49` blend/GLB still supplies frozen Body/Hair
and the definition. Required files are `project.godot`,
`authoring/head_generation_guide.md`, and that baseline's editable blend, GLB,
definition, `hair_base.png`, and `hair_ilm.png`. The current renderer has three
mandatory roles; hiding non-face roles in review is not a standalone-face export.

The packager retains input/source hashes, adapted style, controls, editable and
runtime blends, and the same-job generated Face export. Existing outputs require
current hashes. The facial-surface wrapper currently uses all 64 native output
slots; do not silently omit an evidence file when extending it.

## Controls

Multipliers default to 1:

- eye_spacing .94–1.04; eye_width .92–1.08
- upper_arch .70–1.40; lower_arch .50–1.12
- upper_roundness 1–2; upper_horizontal_roundness .70–1
- lower_roundness 1–1.25; lower_horizontal_roundness 1–1.5
- iris_width and iris_height .90–1.08
- brow_thickness .70–1.20; brow_arch .50–1.50
- lash_band_thickness .55–1.15

Offsets default to 0, in fractions of calibrated face height:

- corner_slope_offset −.018…+.008; eye_height_offset ±.006
- iris_vertical_offset ±.01
- iris_horizontal_offset ±.012; negative moves both iris centers toward the nose,
  after the eye-width mapping. Pupil/catchlight geometry follows each iris center;
  aperture, eye spacing, and whole-orbit geometry are unchanged.
- brow_height_offset and brow_rise_offset ±.012

Numeric layer controls accept only 0 or 1. `supplemental_lashes` and `lid_fold`
default 1. `simple_iris` defaults 0. Booleans, unknown keys, nonfinite values and
out-of-range values fail. Combined values must also satisfy the inherited
aperture-height envelope.

`ocular_skin_outline_holdout` is a separate numeric 0/1 control, default 0,
effective only with the illustrated art profile. It changes only FaceMap B around
the calibrated skin aperture, preserving existing island masks and R/G/A. This
experimental art-direction control leaves geometry and gaze unchanged. With the
current shader's lip fix disabled, only its full-B plateau removes outline;
partial B values do not produce a smooth width taper. Test it separately from gaze.

`lash_contour_follow` is 0–1, default 0. Full follow uses 21 stations, variable
width, pointed ends and a Hermite outer continuation after t=.75. The outer tip
returns toward the eye corner instead of continuing the old raised curl.
Fractional values blend sampled inherited rails; they are not visually approved.
Actual retained-band attachment, obstacle, volume and degeneracy checks remain.

When supplemental components are omitted, the supplemental-only connection
check is explicitly not applicable; final assembly asserts that all those
components are absent. This does not waive checks for retained geometry.

`simple_iris=1` removes the separate pupil-accent geometry and lower graphic
circle/outline, shortens pupil height to 84% and scales catchlight dimensions to
80%. Iris height/width and UV mapping remain coupled. The iris vertical offset
moves the eye layers together without moving eyebrows or skin.

## Visual case and limitations

The optional `clean_emerald` style profile belongs to
`../facial_surface/bundle.py`. It keeps the inherited default unchanged and
reproduces the original eye-art direction inspected in case685: shallow rounded
lash section, restrained dark outer tint, thinner lower line, a recessed iris rim
with shallower concavity, and softer arched brows. It is a reviewed case, not a
claim that every control combination is attractive or collision-free.

Case683 was rejected after rendered close-ups revealed iris-rim slivers above
the upper band and an overlong plum tail. Cases684/685 resolved the visible
slivers in the inspected front and three-quarter views. Five MouthOpen samples
had no degenerate triangles. BVH overlap candidates between the hidden iris,
pocket and upper lid remain to be classified; these are not zero-collision
results. White-model cheek curvature remains a separate open issue.

## Matching SDF and validation

After successful geometry generation, `sdf_bundle.py` requires a trusted captured
Beaver poll receipt. It verifies the same job's blend, GLB and numeric-control
hashes and the current workspace. It never substitutes a previous atlas.

The default `smooth_proxy` policy fits a smooth art-directed normal field to the
actual runtime front shell and excludes eye/nose/lip microrelief from the SDF.
It is not physical mesh-normal shading. Explicit `mesh_normals` instead uses the
actual front-shell UV2 corner normals with coverage, overlap and continuation
gates. Both policies require native rendered light-sweep review; static neutral
SDF does not establish expression-dependent shadow correctness.

Inspect neutral front, both three-quarter views, side and pitched views; inspect
white/wire geometry independently. Check saved runtime morphs and visible
layering before retaining a candidate. Do not equate a `workflow.run` technical
pass with visual acceptance.

Run pure regressions with `python -m unittest discover -s . -q` from this folder.

## Verified-copy SDF mode

`--allow-workspace-copy` explicitly permits baking an unchanged verified copy in
a different workspace. All three immutable blend/GLB/parameter hashes must still
match the original successful job. Both workspace identities are retained; this
does not authorize stale or unmatched geometry.

## Retained native regression

Install NumPy for CPU checks. Build a manifest from the current project snapshot:

```sh
python native_probe.py --workspace /path/to/project \
  --base-prefix assets/aster/general_face_eyes_base \
  --target-prefix assets/aster/general_face_eyes_target \
  --output semantic_eyes_probe.json
```

Stage the manifest at `res://tests/semantic_eyes_probe.json` and the retained
`native/test_semantic_eyes.gd` through an authorized Beaver native validation
request in the NPR/GUT project. Execute only through that native workflow.
The two-case contract expects baseline first and an optional-layer-reduced target
second. The native test deliberately requires fewer target vertices; arbitrary
pairs without layer removal are outside this regression case.

The manifest binds the original model plus each case's model, definition,
generation report and numeric parameters. It derives local Face bounds from the
actual GLB at MouthOpen 0/.25/.5/.75/1, with a fixed 3e-6 m tolerance. Native checks
compare frozen Body/Hair arrays exhaustively, require complete finite Face basis
and morph arrays, and bake actual imported mouth mixes. These checks establish
sampled import behavior, not geometric clearance or rendered likeness.
CPU tests cover packaging, controls, SDF gates and manifest fixtures only.

## Evidence recorded 2026-10-15

The bounded 670 procedural baseline and numeric eye target both completed actual
native generation. Native Godot/GUT run
`36829722-b0f4-45b0-9f40-5b76ea727db7` passed 9 tests and 464 assertions.
The retained GDScript is byte-identical to the executed regression; the generic
manifest builder reproduces its actual two-case manifest exactly. This record
does not imply engine execution of CPU fixture tests.

The first directional-SDF attempt failed strict projected-chart diagnostics.
The corrected front-chart selection excludes geometrically back/edge-facing
return surfaces before rasterization; the second native bake succeeded. The
failed attempt remains failed evidence. Coverage, degeneracy, overlap-normal and
propagation gates were retained, with excluded-triangle diagnostics recorded.
Every bake requires a trusted successful same-job generation receipt binding the
runtime blend, GLB and parameters. A different workspace requires the explicit
verified-copy option above; stale or unmatched output hashes are rejected. Receipts are caller-trusted evidence, not signatures.

Actual updated-SDF visual review failed: disconnected shadow islands were observed
around the nose, lips and chin. Passing geometry/import checks and strict chart
gates did not establish acceptable NPR shading. The mesh-normal atlas remains
failed diagnostic evidence. The smooth-proxy correction and its bounded acceptance
are recorded below. This example makes no
Faye likeness, complete-face, full-performance, or whole-character acceptance
claim. The later 671 candidate failed its fairness check and is excluded from
this source evidence; `semantic_surface` is outside this publication.

The smooth-proxy correction passed both native bakes: baseline job
`0bf01cdb-2dda-462f-82d3-36bbb48872d4` (124517 ms) and target job
`6843eec1-4800-4534-b2ab-6e19cb72996f` (122958 ms).
Actual front-render review covered 125 lighting configurations (25 azimuths by
5 elevations). The disconnected nose/lip/chin islands were absent in those
samples. There are 250 preserved captures across two UI states, not 250 distinct
lighting conditions. The debug-toggle captures still show a colored face, so no
diagnostic-mask visualization coverage is claimed. Actual right-eye review additionally covered 111 captures: yaw 0 through 180
degrees in 5-degree steps, each in render/white/wireframe mode. No new observed
eye-layer rupture or floating was found in those samples. Mouth renders at
front weights 0.5/1 and yaw 45 at weight 1 were also inspected without a new
sampled mouth defect. Existing cheek bands and identity differences remain
outside this eye-control slice. Final refreshed native Godot/GUT run
`2bfc3573-ccb6-4f0a-be18-ed009fb6362a` passed 9 tests and 464 assertions
in 37.117 seconds after the proxy bakes. Acceptance is limited to these bounded
generative eye controls and NPR shading samples; full Faye likeness is incomplete. On the actual baseline fit, 79.77% of interior samples
matched within 2 micrometers, with condition number 7.62. Excluded local
refinements reached 9.16 mm residual; they are reported and are not claimed to
follow the dominant map. The previous failed mesh-normal atlas remains separate.

## Exact repeat-export comparison

`repeat_streams.py --first face-a.glb --second face-b.glb --output comparison.json`
compares standalone same-job Face exports. It joins every attribute and morph
component per vertex before canonicalizing oriented triangles. Vertex/triangle
ordering may differ, but values are not rounded; winding, unused vertices,
materials and embedded image payloads remain part of the comparison. Duplicate
joint vertex records make a unique topology bijection ambiguous and fail closed.
Animations, skins, node transforms and unsupported extensions are rejected.
The supported material texture-transform extension is compared in full.

The native 685 and independent 687 exports have 7,284 unique joint vertex records
and 9,852 oriented triangles. They are exactly permutation-equivalent although
their raw GLB hashes differ. All three face texture PNGs are byte-identical. This
is evidence for that one configuration, not all-parameter stability, collision
freedom, aesthetic acceptance, or identical engine-import behavior.

### Standalone Face export and atlas identity

`semantic_face_export.glb` contains the assembled Face mesh, including eyes,
small facial parts and the `MouthOpen` morph. It has no body or hair node.
Its embedded base-color PBR material requires `KHR_texture_transform` support.
The NPR shaders, ILM, map and SDF resources remain separate and are referenced
by the generated character definition; a standalone GLB is not the complete
NPR presentation.

The semantic packager keeps the generated atlas's same-job absolute path and
packs its bytes before saving or exporting. Setting `//face_base.png` while
the inherited source blend was still open previously rebound the image to the
inherited atlas. Native case 701 fixes that: the embedded PNG is byte-identical
to its declared same-job atlas, while the joint geometry, morph streams and
oriented triangles remain identical to case 697. See `export-evidence.json`.
The frozen checkpoint is unchanged.

Run the separate structural check after generation (Pillow 12.3.0):

```sh
python examples/head-generation/semantic_eyes/export_contract.py \
  path/to/semantic_face_export.glb path/to/face_base.png \
  --output /tmp/face-export-contract.json
```

This checks scene reachability, the narrow Face/morph/material contract,
finite streams, reference indices, embedded PNG decoding and same-job RGBA
identity. It is separate from `repeat_streams.py`: equality alone cannot prove
that either file is valid. Neither check establishes aesthetic quality,
continuous collision freedom, or native importer compatibility.

## Optional independent eyelid closure

Both packagers accept the explicit `--blink` flag (Python: `blink=True`). The
unchanged default exports only `MouthOpen`. The optional mode adds neutral
`blink.L` and `blink.R` shapes, requiring `supplemental_lashes=0`, `lid_fold=0`,
and `lash_contour_follow=1`. The supported short upper tuft may remain enabled.
Positive X is `blink.R`; negative X is `blink.L` in the model coordinate frame.

Skin closure inside the aperture retains a 2% vertical span, while the lash/lid
boundaries meet on a gently curved seam. This prevents the collapsed or reversed
corner triangles found in the rejected full-translation prototype. Eyebrows,
iris, ears and rear shell remain fixed by the explicit semantic membership.
The neutral shape and existing mouth shape are not edited.

For the opt-in illustrated profile without an independent upper tuft, final
main-lash columns use their generated lower edge as the closure anchor. This
avoids carrying an authored below-aperture offset underneath the closed seam.
The entire signed-x column translates together, retaining its cross-section.
Other profiles and retained tufts use the existing analytic-aperture mapping.
This does not prove posed attachment or collision freedom; native review is
required, including the transition to stationary outermost vertices.

The native generator rejects any triangle coupling both independent eye
controls. For every triangle it minimizes the quadratic oriented-area projection
onto its neutral normal over blink and MouthOpen weights in [0,1]. A positive
margin above 1e-5 is required. This floating-point calculation is not an interval
proof, collision test, or evidence of attractive poses. Native multiview renders
and actual imported blend-shape readback are still required.

`export_contract.py --with-blinks` explicitly selects the three-name morph
contract and validates every position/normal stream. Without that flag it still
rejects extra targets. Both modes require zero default weights and the actual
same-job embedded atlas. The optional runtime helper is retained inside the
hashed generated `semantic_eye_runtime.py`, without exceeding the facial
wrapper's 64-output limit. No automatic blink driver is enabled.

The retained `blink-evidence.json` records native case709: all three morph streams
repeat case708 exactly up to a unique vertex permutation, with 8,183 exported
vertices and 10,980 triangles. Its independent SDF bake matches case708. The
native orientation minimum is 0.03567719. All 69 actual render/white/wink-with-mouth
captures were inspected and read-back weights checked. Case707 was rejected for
eight reversed eye-corner triangles and is not the retained implementation.

For explicit native review, stage `examples/head-review/tests/blink_pose_probe.gd`
in the existing Beaver project and temporarily point the head-review scene at it.
It records six views at five bilateral closure values in render/white modes,
then independent winks and bilateral closure with MouthOpen 0.6. Check its
manifest hashes, requested/read-back weights, images, and unchanged source hash;
restore the ordinary head-review scene afterward. The probe does not establish
acceptance merely by writing a manifest.

`iris_plane_slope_scale` defaults to 1 and is bounded to [.85, 1]. It scales
only the baseline horizontal iris support-plane slope, pivoting at each iris
center; pupil and catchlight use that same calibration. This is an art-plane
control, not a certified gaze direction. It is independent of iris translation
and ocular outline holdout. Facial-surface preflight adds the maximum absolute
slope-change displacement over the entire iris/pupil/catchlight domain to its
previous conservative support bound, and rejects an insufficient explicitly
reviewed bound. At the current .85 experiment this adds about .625 mm; the
reviewed experiment budget is .0045 m (unchanged controls retain .0042 m).
Native limbus/socket/lid clearance, side intersections and posed acceptance
remain required; a displacement bound does not certify those clearances.


### Optional lower-lid contour shoulder

`lower_roundness` is a bounded multiplier (1.0–1.25, default 1.0) for the lower aperture exponent. Increasing it raises intermediate lower-lid stations while preserving the central depth, corner endpoints, eye center and width; use `lower_arch` separately to adjust central depth. The default preserves the prior contour. It does not add asymmetric peak positioning or change the upper eyelid. Non-default values also use the matching inverse-power lower-pocket clearance classification. Blink, skin support and preflight receive the same adapted calibration. CPU tests do not certify native clearance or visual appeal; compare matched views and unchanged SDF shading policy before retaining a candidate.

The range is provisional: a 2.0 candidate with lower_arch 0.70 failed the native continuous-pose triangle orientation guard; it was rejected, not retained. A 1.25 candidate passed generation, which does not certify all parameter combinations. Exactly 1.0 retains the legacy pocket classification for compatibility; opting into a non-default value also opts into the inverse-power classification. This compatibility switch is discrete even for an arbitrarily small shape change. Native pose and clearance checks remain mandatory.

### Experimental upper-aperture asymmetry

`upper_asymmetry` defaults to zero and is limited to [-0.25, 0.25]. Nonzero
values require level corner calibration. The physical horizontal station,
aperture width, lower contour, iris shape/position parameters and eye centers stay fixed. The ocular support plane is refitted and must be checked for perceived gaze changes. Only the upper
arch's shape coordinate uses `t + a * (1 - t*t)^3` inside [-1, 1], with identity
outside. Positive values move the upper peak toward the nasal side for level
corners; this is a stylistic experiment, not a universal beauty preference.
The warp is globally increasing (derivative at least 0.57067 in this range),
and its value and first two derivatives meet the identity extension at both
ends. The existing outer inverse remains descending on its [0.4, 1] branch;
heights above that branch keep the inherited endpoint saturation behavior.

One helper contract drives the derived native contour, upper orbital support,
lash guide and tail derivative, blink boundary and CPU interval enclosure.
Facial preflight receives the same shape before fitting. The upper pocket
classifier changes only its horizontal coordinate, retaining the legacy vertical
classifier. Zero mode preserves inherited contour sources and calculations.
The helper source texts are embedded in the retained packager as provenance;
packager replay still requires the checkpoint recipe and its CPU dependencies.
The generated native request is self-contained and remains within 64 outputs.

The existing support certificate bounds the facial correction relative to its
supplied support field, not the entire shape difference from a prior candidate.
Patch envelope, clearance, attachment and continuous pose-orientation gates
remain mandatory. Inspect neutral, partial/full blink, oblique and white-model
views before retaining any nonzero candidate. CPU tests alone establish neither
native validity nor visual improvement.

## Independent lower contour controls

`lower_shoulder` is optional, default 0, bounded to 0–1. It symmetrically
raises intermediate lower shoulders while preserving the central bottom and
both aperture corners. It requires level calibrated eye corners. Its monotone
coordinate warp is shared by the face, pocket radius, lateral fit, blink and
conservative support analysis; it is not a post-export mesh edit.

`lower_horizontal_roundness` defaults to 1, bounded to 1–1.5. Increasing it
flattens and broadens the lower center, with a steeper return near the corners.
This is distinct from raising the shoulders and can partly counter that change.
Neither control guarantees a prettier face or a valid combination. Native
support, topology, attachment and full-blink orientation gates remain mandatory.

The facial-surface preset `emerald-lower-profile-eyes.json` records candidate759,
which received relative-improvement feedback, not final character acceptance.
The private authoring baseline is still required. See
[`lower-profile-native-verification.md`](lower-profile-native-verification.md)
for the actual checks and limitations.
