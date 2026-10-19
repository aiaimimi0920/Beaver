# Face-only creation and visual review

This is the current tested face-generation workflow, not a claim that every
parameter combination yields an attractive finished character. It covers an
original NPR face, eyes, lashes, brows, ears and mouth. Hair, body, costume and
full-character animation are outside this pass.

## Primary route: independent components (2026-10-19)

Use [face_components](face_components/README.zh-CN.md) for new work. Its direct consumer is dot, preparing requests for Beaver's existing native tools; this is not a newly shipped preset-picker UI.

1. Select eyes, brows, mouth, nose, ears and face explicitly. Use the saved per-component authoring intents and supported parameter IDs. Existing eye-family prompts remain intact.
2. Resolve and save the complete combination. Brows are selected separately; iris and lash ornament currently remain eye-family-coupled. Nose/ear/mouth alternatives are bounded initial variations, not complete researched category libraries.
3. Generate to a fresh output directory through Beaver, retain the successful receipt, then build matching SDF from the actual generated runtime model and saved composition.
4. Check front, both oblique sides, side silhouette, render/white/wire, blink and mouth behavior. Do not transfer a single preset's acceptance to a new combination.
5. If a combination fails a geometry gate, preserve the failure and correct the relevant attachment/construction. Do not raise the gate or silently change the selected family. An explicit fit policy and fresh output are required for such a retry.
6. Retain the selection, authoring text, source hashes, receipts, model and view evidence. Prioritize closing this whole workflow and extending missing component families. Local polishing is for a concrete blocker or visible regression, not an indefinite replacement for the six-library task.

Current counts and limitations are documented in the component catalog and research index. A Cartesian product of options is not a count of validated attractive models.

## Historical single-face authoring brief

> Create an original anime-game face from explicit proportions and procedural
> controls. First establish a clear front and three-quarter silhouette. Shape
> the eyes as soft, intentional curves: a restrained upper arch, a rounded lower
> return and a tapered outer corner. Keep the main lash band close to its support
> surface; use a small separate pointed tuft only where it adds a readable layer.
> Avoid thick spikes, floating roots, square lower corners and exposed iris-rim
> slivers. Keep the iris, pupil and catchlight readable at normal viewing size.
>
> Compare a candidate with the retained best version using the same camera,
> scale, pose and light. Examine rendered, white and wire views separately.
> Name the visible problem, change one coherent cause, regenerate, then inspect
> the result. Reject a technically successful candidate when it looks worse or
> provides no convincing improvement. Keep the best version available.
>
> Preserve already satisfactory nose, ear, mouth and jaw behavior while changing
> the eyes. Check both sides, oblique and pitched views. Inspect attachment,
> winding and posed geometry independently of shading. When optional eyelid
> closure is enabled, inspect intermediate closure, full closure, independent
> winks and mouth combinations. A successful job, export or screenshot count is
> evidence of execution; it is not an aesthetic judgment.

A character name or a reference picture is visual direction, not an executable
model specification. Extract proportions, curves and layering choices; do not
copy reference mesh coordinates, textures or UVs into the generated Face.

## Earlier reviewed starting point

Use the retained numeric file
[`facial_surface/emerald-layered-eyes.json`](facial_surface/emerald-layered-eyes.json)
with:

- style profile: `emerald_layered`
- facial-surface controls: `axial_smoothing=0.5`, `cheek_roundness=0.05`,
  `orbit_shallowing=0.25`
- reviewed support bound: `0.0042` meters
- optional eyelid closure: explicit `--blink`; omit it for MouthOpen-only output

The layered profile includes reduced nose-plane pigment; it does not change
nose geometry. Do not silently describe it as an eye-texture-only change.
The support bound is a displacement guard, not a clearance or beauty threshold.

1. Inspect the actual project inputs and use a distinct output prefix. Package
   the request with `facial_surface/bundle.py`; this step creates JSON only.
2. Execute that request through Beaver's native generation workflow. Keep its
   successful terminal receipt and declared outputs. Do not use files from a
   failed or still-running job as a completed candidate.
3. Validate `semantic_face_export.glb` against that same job's `face_base.png`
   using `semantic_eyes/export_contract.py`. Select `--with-blinks` only for the
   explicit MouthOpen/blink.L/blink.R contract.
4. Package a fresh directional SDF with `semantic_eyes/sdf_bundle.py`, bound to
   the successful geometry receipt and current hashes. Execute the bake and
   inspect the material's actual enabled/debug state in the native viewer.
5. Inspect the native views and poses below. Record reasons to retain or reject
   the candidate. An export validator cannot establish visible layer quality.
6. Regenerate a retained recipe independently. Use
   `semantic_eyes/repeat_streams.py` to compare the actual Face exports, including
   all attributes, morphs, oriented triangles, materials and embedded images.
   Ordering-only equality does not prove import behavior; reopen the repeat.

The packager takes facial controls as JSON text in `--parameters` and eye controls
as JSON text in `--eyes`, not as file paths. Supply `--style-profile`,
`--reviewed-support-bound-m`, `--workspace`, `--prefix` and `--output`; the prefix
must match `assets/aster/general_face_eyes_<case>/`. The export validator takes
the GLB and PNG as positional paths plus `--output`. The SDF packager instead
takes eye controls in `--parameters` and requires `--generation-receipt` along
with `--workspace`, `--prefix` and `--output`; its default shading policy is
`smooth_proxy`. Repeat comparison takes `--first`, `--second` and `--output`.
Packaging does not execute either native generation or baking.

The current private-project recipe still reads the `head_recovery_49` baseline
for its editable input and frozen Body/Hair assembly roles. These roles are
hidden by default in the face-review viewer and are not newly authored; the
viewer can toggle their visibility. The standalone Face output is
`semantic_face_export.glb`; the assembled `aster_head.glb` is not a face-only
file. A completely seed-independent public generator remains separate work.

## Visual acceptance sequence

- Front, both 45-degree views, both true sides and moderate upward/downward
  pitch: look first at eye shape, spacing, lid thickness and the whole expression.
- Close-up angle sweep: inspect roots, overlap, independent small tuft, outer
  return and the near/far eye. Preserve natural occlusion instead of hiding
  surrounding geometry to make a defect disappear.
- White and wire views: inspect silhouette, topology, attachment and surface
  transitions. Distinguish a geometry fold, a normal split and a lighting edge.
  Do not mask a geometry defect by changing the material alone.
- Neutral and mouth weights 0, 0.25, 0.5, 0.75 and 1: check the lip opening,
  local chin motion and stable eye/ear position.
- Optional blinks: check bilateral weights 0, 0.25, 0.5, 0.75 and 1, both winks,
  and mouth combinations. Read back actual imported weights. The continuous
  orientation gate rejects folded triangles but does not prove no intersections.
- Directional lighting: inspect ordinary color and actual SDF diagnostic masks.
  Static neutral SDF is art-directed shading, not expression-dependent physical
  shadow validation.

Use the native comparison panel's candidate-only expression sliders for manual
review. The reference stays neutral. F12 and sweep records include actual
weights; confirm those against the requested state before using a capture as
proof. Restore the ordinary head-review scene after any temporary probe.

## Current evidence and remaining limits

Case704 repeated the layered open-eye case702. Optional-blink case709 repeated
case708 exactly up to vertex/triangle ordering and was visually reviewed across
69 captured views. See
[`semantic_eyes/blink-evidence.json`](semantic_eyes/blink-evidence.json) and
[`facial_surface/layered-eye-evidence.json`](facial_surface/layered-eye-evidence.json).
These are internally reviewed cases; final user approval and all-parameter
stability are not implied.

Additional internal reviews compared case710 as a gentler/rounder variation and
case712 as a narrower-eye variation. Each completed native geometry and a fresh
SDF bake, passed the continuous orientation gate, and had 69 rendered/white and
wink-with-mouth captures reviewed with weight readback and source identity
checks. These remain comparison candidates pending full directional-light acceptance;
they are not selected for delivery or replacements for primary case709. They do
not establish all-parameter stability.
The existing white-model cheek shading bands remain in both variations.
A further 42 native views per case cover wire topology, the opposite true side
and mouth weights 0.25/0.5/0.75/1 (neutral is already in the initial set).
The per-case controls, native job IDs, all 111 capture hashes/poses and bounded
review records are retained in [variation evidence](semantic_eyes/variation-evidence.json).
These blink-pose captures are separate from the UI cancellation probe; the latter
does not provide visual acceptance.

The rejected full-translation blink prototype folded eight eye-corner triangles.
The retained closure keeps a small monotonic skin span and passes a positive
continuous orientation margin. Other rejected curvature/flare trials remain
rejected even when their generation or CI succeeded. The lateral white-model
cheek shading bands remain an independent open quality issue.

Record delivery status separately: source written, native generation completed,
export checked, repeat matched, rendered review completed, code pushed/merged,
and user acceptance. None of these statuses should silently stand in for another.
