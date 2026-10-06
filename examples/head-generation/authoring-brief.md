# Face refinement acceptance specification after candidate 102

User review, 2026-10-06, 16:38 UTC+8. Baseline 102 remains recoverable.

## Authored generation requirements

- Generate an original stylized NPR head from semantic proportions and parameters. Do not load reference meshes, textures or UVs in the generation recipe. A character name is not an executable modeling specification.
- Cheek planes must transition continuously into the lower face. Remove unintended horizontal ridges and alternating concave/convex bands, while preserving local feature support.
- The lateral boundary from ear root toward the top of the head must describe a continuous, gentle arc without accidental dents or bulges.
- The whole jaw boundary must taper smoothly toward the chin, forming a curved triangular silhouette rather than a broad, flat-bottomed trapezoid. Do not achieve this by pulling a single chin vertex or collapsing edge loops to a pole.
- Preserve independently controlled shading transitions at the facial centerline and a subtle lower-jaw boundary. Maintain smooth cheek shading between these semantic boundaries. Distinguish subdivision edge crease, geometric dihedral and corner-normal discontinuity. Do not harden every edge.
- Eyes require a deliberate rim/transition around their circular rear structure, shallow embedded placement, independent iris, and visible original white catchlight. Inspect front, both obliques, both sides and rear. A depth reduction alone is insufficient.
- Ear shape, root continuity and shallow open membrane must be judged against the reference appearance and structure. Use original procedural detail; do not copy reference texture data.
- Refine oral cavity cross-section, posterior contour and inner-lip connection as a coherent structure; retain a narrow neutral lip seam, actual cavity and tongue, and localized mouth movement with stable chin.

## Verification

1. Inspect reference geometry, normals and rendered behavior separately. The unpacked FBX cannot establish original Blender modifier history; exported splits are not automatically errors.
2. Revise prompt/spec before changing authored parameters or code. Generate through the normal Beaver workflow.
3. Compare the candidate and reference at identical scale, view and lighting; include underside jaw and inverted/oblique cheek views from the user's review.
4. Keep geometry winding, degeneracy, semantic crease and mouth-motion checks. Add regional normal and contour checks where they measure the requested property.
5. Technical test success is distinct from visual acceptance. Preserve each useful verified source checkpoint remotely and keep runnable model artifacts recoverable.

## Diagnostic evidence, not yet a proven complete cause

Candidate 102 calculates area-weighted per-vertex normals after final triangulation and assigns them to smooth face corners. This averages across a shared facial centerline. The reference export contains differing corner normals on several anterior midline edges, in addition to geometric dihedral. This supports testing regional normal partitioning, without claiming that a Blender Crease attribute alone explains the observed rendering.

Candidate 102 also blends sparse horizontal cross sections and applies a lateral jaw-return correction. Their curvature and side-boundary derivatives require independent inspection before changing the cheek; global smoothing or normal-only masking is not sufficient.

## Additional rendering requirement

Outline is a shader-rendered effect. Its visibility and continuity at normal front/oblique angles must be compared at matched scale and lighting. Diagnose actual material settings, outline width, normal inputs and depth/culling behavior. Do not add black geometry as a substitute. Candidate 102 has some visible outline in the inspected front render, so the defect is inconsistent visibility/continuity, not a verified total absence. User resumed inspection after initially handing the screen back; defer UI input until the viewing interval is over.

## Candidate 103 isolation
First test left/right skin corner-normal partition at the shared facial centerline, while retaining area-weighted smooth normals within each side. Other geometry stays at baseline so the change can be inspected without conflating curvature and shading.

## Candidate 104 surface revision
Suppress short-wavelength variation in the vertical cheek base before adding nose, lips and orbital transitions. Integrate lower-face taper into the entire width profile, reduce concentrated lateral inset, and keep the posterior upper boundary a gentle arc. Retain a modest semantic lateral transition, never harden the full cheek.

## Candidate 105 base-section consistency
The base cheek cross-section must retain the same smooth lateral falloff family across heights. Blend independently authored front and posterior height guides, avoiding alternating lateral curvature from inconsistent section knots. Preserve facial feature relief afterwards and retain the small lowest-chin transition. This replaces the insufficient 104 height-only smoothing.

## Candidate 106 ear, posterior eye and oral structure
- The open ear membrane must have a shallow concave basin and a returning outer lip, not a monotonically receding convex sheet. Preserve shared root positions and an original soft pigment map.
- Use a slightly wider/taller posterior orbital ellipse behind the narrow front opening, with distinct rear-cap normal transition. Keep the pocket shallow and verify clearance rather than allowing the enlargement to protrude.
- The oral bag expands behind the neutral lip slit and then gently contracts toward its posterior wall; avoid a long constant-section tube. Retain a real closed interior, tongue, opening and localized motion, even if the unpacked reference has missing surfaces.

## Candidate 107 regional normals
Ear-root normals must retain a readable attachment transition while the root remains welded to facial positions. Do not average flat eye/口腔 rear-cap faces into smooth wall normals: preserve a deliberate rim instead of rounding it away. Use a subtle lower-jaw normal boundary separately from the smooth cheek and explicit facial centerline. Preserve actual geometry and exported vertex sharing; corner-normal splits are intentional shading attributes.

## Candidate 108 longitudinal curvature and cap integrity
Use a gradually increasing cheek-to-forehead anterior guide without a long flat depth plateau in the cheek. Keep the rim of the rear eye wall planar after fitting clearance, by moving the whole cap only toward the safe posterior side and measuring that extra change. A clearance correction must not quietly deform a deliberately planar end wall.

## User-marked green-curve revision, candidate 109
The accepted facial center division is frozen. The lateral cheek-to-chin transition must follow one continuous convex arc, without a step below the earlobe or a short flat segment above the chin. Move the semantic side transition nearer the perimeter, outside the orbital patch; avoid cutting a hard interior cheek strip. Reduce the localized jaw correction now that the base has a coherent contour, rather than stacking a large correction onto it. The ear upper arc must rise above its highest attachment and remain rounded, with a shallow concavity; avoid the current flat upper attachment and coarse dark lower wedge. All markup is diagnostic input, not a mesh-generation dependency.

## Candidate 110 ear lip and shader-facing surface
The skin pass culls back faces while the outline pass culls front faces. A single turned ear sheet can therefore expose the outline pass as a dark wedge where no skin-facing return surface exists. Author a thin skin-colored returning outer ear lip, still open behind the basin, so both viewing directions have appropriate skin surfaces. This is anatomical rim geometry and normal support; the contour remains generated by the existing shader, never a black modeled stroke. Verify the dark lower wedge in the user's two side views.

## Candidate 111 two-sided thin skin membrane
A narrow returning rim alone still leaves an unshaded back-facing patch visible near the lower ear in the side test. Give the thin curved ear membrane a skin-colored reverse surface separated by a submillimeter thickness, rather than exposing the outline pass. Keep its posterior attachment open and the ear bowl hollow; do not create a closed solid cylinder or a black contour mesh. Preserve front root topology and test both side views.
