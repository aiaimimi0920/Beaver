# Bounded semantic eye family

This separate recipe exposes numeric eye/iris/brow/lash controls. It contains no
character-name selector. A reference-informed planner chooses explicit values;
matching a character is a visual acceptance goal, not a consequence of accepting
valid parameters. Existing `general_face` / 669 code and evidence are unchanged.

## Package for Beaver

From this directory:

```sh
python bundle.py --workspace '/path/to/current/workspace' \
  --parameters '{"eye_width":1.04,"upper_arch":0.78,"lower_arch":0.70,"corner_slope_offset":0.002,"iris_width":0.96,"iris_height":0.92,"brow_thickness":0.70,"brow_arch":1.10,"brow_height_offset":0.004,"brow_rise_offset":0.004,"lash_band_thickness":0.65,"supplemental_lashes":0,"lid_fold":0}' \
  --prefix assets/aster/general_face_eyes_target/ --output request.json
```

The sample is a qualitative first-pass proposal, not measured reference data.
Use `{}` for the new procedural baseline. Submit the resulting code/outputs/
timeout through an authorized Beaver native job. This command only packages.
Required inputs: `project.godot`, `authoring/head_generation_guide.md`, and
`assets/aster/head_recovery_49/{aster_head_editable.blend,aster_head.glb,
aster_definition.tres,hair_base.png,hair_ilm.png}`. The old Face is removed and
procedurally regenerated; original49 supplies frozen body/hair and definition.
No accepted617/662/669 final Face is read or restored.

The bundle retains adapted source/style, input/source hashes, numeric controls,
editable and runtime blends, and the same-job Face export. Existing output hashes
are mandatory optimistic-overwrite guards when outputs exist.

## Numeric contract

All multipliers default to 1:

- eye_spacing .96–1.04; eye_width .92–1.08
- upper_arch / lower_arch .70–1.12
- iris_width / iris_height .90–1.08
- brow_thickness .70–1.20; brow_arch .50–1.50
- lash_band_thickness .55–1.15 (rail width, not mesh extrusion thickness)

Offsets default to 0 and are fractions of face height H=.236 m:

- corner_slope_offset ±.008; eye_height_offset ±.006
- brow_height_offset / brow_rise_offset ±.012

supplemental_lashes and lid_fold accept numeric 0 or 1 (default1). Zero removes
supplemental tuft/fork/outer-wing/lateral components or the fold after inherited
attachment validation, before atlas/normal-region/join assembly. Upper lash band,
skin lids and eyebrows remain. Removed geometry is absent, not hidden by alpha.
Booleans, unknown fields, nonfinite and out-of-range values are rejected. Combined
controls must also remain inside the inherited eye-patch vertical envelope.

## Boundaries and validation scope

- This is a new procedural baseline, not byte-identical669: accepted617 restoration
  and its fixed outer-eye postwarp are intentionally absent. Other inherited
  procedural nose/ear/jaw calibration remains; there are no new controls for them.
- Eye centers/widths, iris UV dimensions and brow placement are coupled explicitly.
  Lash rails follow changed upper contour; evaluated support fitting is retained.
- Supplemental pieces are constructed and validated before omission; their legacy
  checks can still reject a candidate. No thresholds are reduced to force success.
- Fixed patch topology and inherited normal/ear fitting need native validation.
  CPU checks do not prove zero intersections, visual likeness, or mouth-open safety.
- Generated control maps use the adapted procedural field. For directional SDF,
  use this directory’s `sdf_bundle.py` with the same `--parameters`, `--prefix`,
  `--workspace`, and `--output` arguments after geometry completes, plus
  `--generation-receipt /path/to/native_poll_receipt.json`. The trusted captured
  successful Beaver receipt must bind blend/GLB/parameters from the same job
  and workspace; receipt hash/job/script identities are retained. New generation
  reports also bind the saved runtime blend and final semantic-region counts. It hashes the
  current runtime blend/GLB/report/definition/parameters and verifies them before
  execution. Existing output hashes and idempotent atlas binding are retained.
  The default `smooth_proxy` policy fits the current neutral front-shell silhouette
  and dominant authored UV2-to-Blender-XZ map. It uses explicit art-direction gains
  0.83 lateral / 0.62 vertical and excludes local eye/nose/lip/depth relief from
  shading directions. It is not a physical mesh-normal field. At least 70% of
  interior samples must support the map within 2 micrometers; fit conditioning,
  bounds, inlier coverage and excluded local-refinement residuals are reported.
  This is an approximation to the current geometry, not an exact global map.
  Static neutral SDF excludes expression-dependent shading and cast occlusion.
  Native multi-light visual review remains required.

  The diagnostic `--shading-policy mesh_normals` retains the failed visual
  experiment: geometrically front-facing NormalRegion1 triangles, strict UV
  coverage/degeneracy/overlap gates and bounded uncovered-pixel continuation.
  Successful rasterization did not prevent disconnected NPR shadow islands.
  Do not treat that diagnostic policy as visually accepted.

  By default the generation receipt must name this workspace. To bake a verified
  copy in a separate Beaver workspace, explicitly pass `--allow-workspace-copy`.
  This still requires all three immutable blend/GLB/parameter hashes to match the
  original successful generation job; both workspace identities and the copy
  verification are retained. It does not authorize unmatched or stale geometry.

- Native review should compare baseline/target front, three-quarter, profile and
  mouth weights 0/.25/.5/1; inspect actual bounds, iris/lid containment, lash roots,
  nondegeneracy and body/hair hashes before retaining results.

CPU verification: `python -m unittest discover -s . -q` and `python -m compileall -q .`.

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
