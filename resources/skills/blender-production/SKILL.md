---
name: blender-production
description: Produce and integrate Blender game assets, including reference-led anime NPR characters, without requiring the user to operate the editor.
---

# Blender production

Use the configured Blender executable or enabled Blender MCP. Inside a managed
Beaver NPR task, use the isolated session described in beaver-workflows and verify
a real MCP call before modeling. For a directly requested offline asset review,
use a separate process and never attach to the user's unrelated open scene.
Reopen saved sources when continuing. Each execute-code call must include the
imports and state lookup it needs.

Inspect requested scale, axes, material style and actual reference images. Use
reproducible Blender Python when useful, but judge the rendered asset, not the
complexity of its generator. Save editable .blend sources and export GLB/glTF
with required textures. Validate scale, transforms, materials and requested
animation. Integrate into Godot when requested. Work inside the task workspace;
clean up only processes created by this task.

## Authored stylized characters

For anime / 二次元 NPR characters, first read
[the anime NPR character workflow](references/anime-npr-character.md). It defines
the anime-npr-character style, half-model construction, quad topology, open
face-shell options, surface-mesh budgets and layered features. Apply it with the review checks below;
do not impose this style on unrelated NPR or explicitly requested chibi designs.

Apply this section when the target is a finished stylized or anime character.
Respect deliberately simple, toy-like or graphic designs when requested. Do not
force one reference's proportions, costume or identity onto unrelated characters.
For a localized revision, apply the checkpoints to the changed region and its
affected silhouette. Preserve other parts; shortening a robot arm does not call
for a new face, hairstyle or full character redesign.

Use supplied references as a visible quality target. Inspect front, three-quarter
and profile head views plus the affected overall silhouette. For an existing
model, compare before/after with matched camera, framing, light, exposure and
outline settings. Record which parts the user already approved and preserve their
design; allow only the placement or attachment changes needed by connected edits.
New feedback that rejects a shape overrides earlier preservation limits for that
shape. Do not freeze rejected proportions or impose tiny displacement limits that
prevent the requested correction; preserve the unrelated approved design.
If a reference is unavailable, report that and use the confirmed intent.

Inside managed Beaver tasks, use the `preview` workflow for repeatable captures.
Pass `camera={target:[x,y,z],distance:positive,fov:degrees,
angles:[0,45,90]}` for a focused set and `grayscale=true` when value comparisons
help. Reuse `report.camera` unchanged before/after; check the returned definition
and model hashes identify the asset being edited. Choose head landmarks from the
actual geometry when setting a close-up target, not from whole-body bounds.

## Turn a reference into construction decisions

Follow the displayed scene or character definition to the meshes and materials it
actually uses. A file named "model" may contain only the body; a source file may
differ from the rendered face. Record the relevant scene, part and material paths.
Inspect actual vertex positions, surface/component boundaries, UVs, normals and
textures as needed. Do not infer construction from a screenshot or shader name,
or silently substitute the reference's assets for the requested character.

Check texture channel semantics and UV orientation when creating a Blender
reference view. Keep opaque skin opaque when source alpha carries masks, and
keep display conversions separate from canonical game textures.

Compare corresponding anatomy at a declared common scale and camera projection.
For head work, align head landmarks and head size rather than matching full-body
bounds or hair height. Distinguish exposed neck length from neck anatomy hidden by
a collar. Temporarily hide hair/accessories in a diagnostic view when they obscure
ears, the jaw or neck; keep the approved final appearance intact. Label measured
relationships and uncertain estimates instead of inventing universal proportions.

When imitation is requested, measure a small set of normalized relationships from
the actual reference: facial width at cheek, nose, mouth and chin levels; eye
spacing and opening; and forehead, nose, lips and chin projection in profile.
Exclude ears and open-shell cut edges from facial silhouette measurements. Use
those relationships together to construct the large forms and move attached
features with their support surfaces. Independent widening/lifting by height can
leave shelves or incompatible proportions even when each local edit is smooth.
Skin-only samples omit eye apertures; include eyelid corners and opening landmarks
explicitly. Reject depth samples that pass through an aperture to a rear surface.
Keep character-specific measurements in the task notes, not as universal defaults.

When a user requests side-by-side geometry, save an inspectable comparison scene
with clearly named reference and candidate groups at the same anatomical scale.
Keep the reference out of production exports. Identify open shells, cropped necks
and missing anatomy so their boundaries are not mistaken for intended contours.

For each important gap, make a short construction recipe in existing task notes:
the observed reference relationship, the geometry/normal/texture/material that
produces it, the corresponding edit, and the views that will show success. Turn
user feedback such as "flat eyes" into observable depth and overlap requirements.
Separate confirmed reference facts from proposed implementation choices. Keep the
recipe focused on the requested region; do not create a long design document.

Inspect both a simple-material structural view and the intended NPR rendering.
The first helps expose curvature, intersections and silhouette; the second shows
the combined authored result. Decide which layer causes the gap before editing.
Keep related head, jaw, neck and facial-feature changes together where they share
geometry or proportions. Recheck approved neighboring parts after those changes.

## Build connected forms

Work from large forms to small details. Establish silhouette, head/neck/shoulder
relationships and facial character first, then hair masses and clothing structure.
Show an early head close-up in the target renderer before spending time on tiny
weave, seams or decorative strands. Primitive combinations are useful blockouts;
high polygon counts, smooth shading and elaborate accessories do not turn an
unresolved face or silhouette into a finished character.

For an anime-like human face, evaluate these choices against the reference:

- Establish forehead, cheek, jaw and chin width/depth relationships in front,
  three-quarter and profile views. Check the chin's projection and underside with
  the mouth, jaw and neck, not by moving the chin tip in isolation. Compare neck
  length, width, tilt and head/shoulder attachment together; a vertical cylinder
  under a flat jaw is not a resolved head-neck transition.
  Follow cheek-to-jaw-to-chin width changes instead of a single conical taper.
  In profile, inspect both chin-to-throat and occiput-to-nape contours. Correct a
  displaced neck attachment or abrupt bulge in the connected volumes; smoothing
  the seam alone can leave the faulty silhouette intact or erase the jaw.
- Inspect the eye surface's curvature and depth, eyelid opening, upper/lower lid
  weight, corners and lash overlap. Reproduce the reference's spatial cues with
  suitable eye surfaces and conforming lid geometry; check their occlusion while
  turning the head. When the reference has an eye opening, build the opening and
  lid attachment in the skin; opaque skin left across it can hide recessed eye
  layers. Do not force realistic spheres into a stylized face. A uniform black
  ring around a white ellipsoid reads as a toy eye; a flat eye decal can lose all
  depth in oblique views. Iris color/value variation, UV detail and restrained
  highlights support this structure rather than replacing it.
- Determine how the reference combines brow support, surface-following eyebrows,
  offset/overlap, normals and shading. Match its subtle depth and placement relative
  to the upper lids; "three-dimensional eyebrows" does not mean thick raised bars.
- Blend the nose into the facial planes and compare the nose-to-mouth-to-chin
  profile. Use the reference to decide lip volume, mouth opening and accent weight.
  Avoid closed outlines around separate nose bulbs or a thick tube smile unless
  that graphic style is wanted. Preserve the intended facial expression.
- For visible human ears, inspect size, position, tilt, shell thickness, inner
  recess and attachment to the head. Retain the reference's readable rim, inner
  fold and lobe cues with appropriate simplification. Inspect each ear without
  hair occlusion and again with the approved hairstyle.

Organize hair into a few primary masses with varied secondary locks, taper and
overlap. Prefer shaped cross-sections/ribbons where the reference calls for them;
equal-radius tubes and repeated full-length grooves produce noodle-like hair and
outline noise. Author highlight flow and color variation along the hair rather
than relying on dense geometry for every strand. Inspect the roots and hairline
from both sides, including under accessories: avoid unintended exposed scalp and
flat-cut seams where locks join the main hair masses.

Make clothing read through fit, seam placement, thickness, overlaps and a small
number of structural folds. Attach folds to plausible tension/support points.
Keep hands, cuffs, collar and hem transitions coherent. Match accessory detail to
the final viewing size: dense subpixel weave and parallel lines can alias into
dark bands. Use texture/shading detail or simpler forms where that reads better.

## Visual checkpoints and delivery

Inspect real target-renderer images while editing, including a readable head
close-up, three-quarter/profile and full-body view. For NPR artifacts, compare
outline on/off and left/right lighting to separate bad form/UVs/normals from
material or shadow settings. Fix the responsible asset or parameter; do not hide
defects by moving the camera away or disabling required rendering features.
If a face shadow reads as an isolated painted spot, check its transition as the
light changes. A spot that persists without outlines may need a face-map or UV
correction; do not reshape already-correct geometry without checking that cause.

Do not postpone all materials until a detailed mesh is finished. Use a minimal
valid NPR setup early to check eyes, face shading, hair silhouettes and line
weight together. Keep component groups and clean normals in editable sources;
check exported geometry after modifiers, including mirrored/reversed islands.
After topology or thickness edits, check bounds for stray vertices and inspect
exported normals. Recompute stale custom normals on rebuilt regions while
preserving intentional normals elsewhere. When folds read flat, compare normals
and material light/shadow response before adding more geometry.
Synchronize .blend, GLB, textures and definitions after revisions.

At each checkpoint, inspect the actual image against both the approved reference
and the previous result. Check the requested construction relationships, not only
whether the new image differs from the old one. Fix the largest remaining gap
before adding detail. A compact matched set is preferable to many redundant views.
Honor the user's explicit review checkpoints. Otherwise use the task's autonomy
policy; routine artistic corrections do not need a new question each time.
Report technical validation and visual acceptance separately, including unresolved
user-requested defects. A saved recipe or successful import alone does not
establish improved generation ability
or parity with a reference; exercise the recipe on the actual requested asset.

## Human-guided review rounds

When the user requests review after each round, make one scoped revision and
stop after saving the candidate and completing relevant technical checks. Reopen
the latest saved source first; inspect its revision notes so a resumed task does
not apply the same deformation twice. Preserve approved regions and the previous
candidate for comparison. Do not start another aesthetic pass before feedback.
If the user has an open scene with unsaved changes, use an isolated session and
save a distinct candidate; do not replace or close their inspection scene.

Provide the exact candidate .blend path, a useful saved inspection view, matched
before/after evidence and a short account of the changed forms. Open that saved
candidate for inspection when requested. Mark visual acceptance as pending;
passing an import or validator does not approve the appearance.

In existing production notes, connect the observed issue, construction decision,
visible result and human feedback. Keep proposed lessons distinct from accepted
ones. Generalize demonstrated relationships and checks into the production skill;
do not promote one character's proportions or an unreviewed image to a universal
beauty rule. Autonomous work remains the default when no review pauses are asked.
