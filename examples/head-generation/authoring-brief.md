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

## Candidate 113 continuous half-ellipse pinna
The upper ear boundary must be a genuinely curved half-ellipse in side projection, not a straight ruled ray lifted from the root. Use a sinusoidal outward/posterior sweep and a cosine-spaced height arc with a modest upward bias. Interpolate the arc nonlinearly toward the fixed facial root so the upper attachment turns smoothly. Retain the verified reverse skin surface and accepted central facial division. Candidate112 graphic/silhouette experiment did not sufficiently match the reference and is not the final result.

## Candidate115 orient the concavity laterally as well as posteriorly
A side-visible hollow ear needs its basin to recede toward the head, not only toward the back. Apply a bounded inward lateral displacement in the interior of the membrane while preserving both the fixed root and the rounded outer lip. This separates visible concavity from the outer silhouette and prevents a posterior bulge from being mistaken for a hollow pinna. Retain thin reverse skin and existing face geometry.

## Candidate116 preserve the ear-rim normal transition
Do not smooth the returning outer ear lip together with the basin wall. Retain a dedicated corner-normal group at this geometric fold so the lip catches light distinctly while the basin remains smooth. Keep all positions and the accepted facial centerline unchanged for this isolated shading comparison.

## Candidate117 monotone ear cross-section informed by normal directions
Read-only reference inspection shows the visible ear membrane normals remain outward and forward across the basin; they do not flip to the back-facing side. Replace the over-bent posterior/lateral displacement with a monotone cross-section whose slope is stronger at the attachment and outer lip and weaker in the basin. A smooth integrated cosine slope gives that controlled fold while avoiding a flipped strip. This uses a generic mathematical profile, not copied reference vertices or normals.


## Ear lower-arc correction 118
Preserve the accepted facial center division and welded ear roots. The side silhouette must turn smoothly from the lower root into a rounded lobe, rather than leaving a pointed diagonal. Weight the upper-arc lift toward the upper ear so that it contributes zero first derivative at the lower endpoint. Keep the shallow, non-flipping cross section and original skin/texture workflow. Verify front and both side views against the reference; this is an iteration, not aesthetic acceptance.


## Ear pigment correction 119
Keep the rounded lower silhouette from118. Original ear pigmentation should suggest a short upper arch, a shallow concha and a short inner fold ending above the lobe. Do not draw a full-length stripe from the upper root to the lower attachment. Keep the lower lobe nearly plain skin. The reference informs structure only; no reference texture pixels are imported.


## Lower-cheek contour correction120
The green-marked cheek-to-chin outline must taper continuously without a pinched lower cheek followed by a straight oblique segment. Use a small number of independent rounded width controls: increase lower-cheek fullness most around 12 percent of normalized head height, fade the change toward the ear root, and preserve chin-tip width and all center shading rules. Read-only reference cross sections show the current lower-cheek span is too narrow. This is sparse semantic calibration, not a mesh fit or reference geometry import. Inspect the same front and oblique white/render views after Beaver generation.


## Active parameter contract121
Remove obsolete calibration controls left behind by earlier ear/lip implementations. Every advertised calibration field must be consumed by the current runtime recipe. Retired fields: mouth_lip_relief, ear_outward_profile, ear_outer_depth_profile, ear_basin_depth, ear_upper_arc_lift, ear_lateral_basin. This cleanup intentionally preserves120 generated geometry and texture; validate byte identity after regeneration. Historical notes describe prior revisions only and are not active parameter definitions.


## Ear inner arch placement122
The original ear pigment should remain visibly inside the shallow pinna in true side views. Move the compact upper arch and short inner fold away from the anterior face attachment toward the exposed ear surface. Keep a clear plain lower lobe and a soft shallow concha. Expose arch/fold/concha placement as explicit reusable style controls, preserving geometry and shader outline behavior. Validate both sides and do not accept the adjustment merely because the texture file changed.


Parameter audit correction: conditional subscript expressions consume both upper_lip_edge_depth_delta and lower_lip_edge_depth_delta. These remain active controls and are retained. The first121 attempt failed on the missing lower-lip key before producing outputs; the revised test traverses conditional subscript expressions and verifies all possible C/config keys are present. Only six inactive legacy controls are removed.


## Ear pigment projection123
The122 inner arch remains compressed in the actual side view. Its row-relative UV coordinates follow a nonlinear depth sweep, which can squeeze the painted arch even after changing pigment centers. Test a whole-ear side-plane UV projection of our own front membrane and return rim, normalized by their own depth/height bounds. Keep the reverse skin plain. Preserve all vertex positions, root connectivity, shading normals and texture colors in this isolation step. This is an authored projection experiment, not a claim about the reference UV implementation. The same pattern should occupy a coherent visible shallow ear region from normal side and oblique views.


## Ear upper attachment124
The actual evaluated upper ear attachment in123 stops at normalized height about0.508, below the approximate0.52 semantic reference landmark. The visible upper silhouette consequently sits low. Extend the maximum root sampling height from0.52 to0.53 to admit the next evaluated boundary sample, preserving exact shared root positions and the rounded lobe. This is an isolated bounded attachment-height adjustment; preserve123 whole-ear UV projection, original pigment, accepted face center division and all other geometry. Judge the actual upper arc and root transition in front and both side views, not only a parameter value.


## User review after123: required corrections125 onward
The user explicitly handed the screen back after reviewing123. Keep the accepted facial center division. Four aspects remain unaccepted: asymmetric ear silhouette and original ear texture; the chin's flatter closure specifically at the user's current view; eye depth/iris/highlight/lid relationships that should read more lively and watery without protruding eyeballs; eyebrow shape, taper, angle and brow-eye spacing. Do not treat earlier visibility fixes or technical passes as aesthetic acceptance. Saved view: yaw-0.8, pitch-25.25, zoom1.05, render, light-45, neutral mouth.

## Asymmetric pinna125
Read-only sparse front-ear cross sections show the upper-middle ear body has a fuller lateral span than the matching lower region, with a smaller lower lobe rather than a symmetric ellipse. For example, lower normalized height0.33 lateral span is about0.054H while the upper-middle at0.48 is about0.078H. These are diagnostic semantic proportions, not imported geometry. Add an explicit bounded upper-fullness coefficient to the authored sine outline, applying the same asymmetric envelope to outward extension and posterior sweep. Keep exact root attachment, rounded lower tangent, shallow membrane, original side-plane UV and plain reverse skin. Verify silhouette at front, side and user's saved view. This step isolates ear shape; the other three requested aspects remain pending.


## View-specific chin closure126
The user's saved pitch-25.25 view, not a generic demand for a sharp chin, exposes a flat underside. Read-only projected skin-vertex checks show123 has a near-bottom width about0.129H within0.005H of the lowest projected point; the reference closes more tightly. At zero pitch the candidate's tip is already at least as narrow as the sparse reference sample, so do not narrow the frontal tip or collapse topology to a pole. Increase only the transverse curvature of the posterior underside bands with a separate bounded chin_underside_arch control, keeping frontal chin_return_lift and tip width unchanged. A quadratic crosswise arch retains a rounded center, supports the reference-like view-specific taper and preserves stable mouth/chin behavior. Verify the exact saved view and ordinary front/side views after generation.


## Stable eyelid ink plane127
The lively-eye request includes eyelid coverage, not merely larger white dots. A read-only check of our current upper ink ribbon found its inner/lower row sits roughly10–12mm farther forward than the lid's lower skin at representative positions, because it samples facial skin depth inside the eye aperture. At the user's tilted view that depth difference compresses the visible ribbon and produces a visor-like slope. Align both ink rows to the same local orbital-rim depth plus the existing small surface offset; preserve eye shape, iris/sclera geometry, iris depth, highlight size and all pigments for this isolation step. The change moves the old over-forward lower ink edge back toward the lid rather than pushing the eye outward. Verify front, both sides, oblique and saved pitch-25.25 view, preserving attachment and front winding guards.


Underside isolation refinement: fade the added arch from zero at the first posterior band to full at the rear-most lift0.03 band. The126 projection improved the tilted view but its near-front subdivision also narrowed the ordinary front tip. Keeping the first underside band at the original chin_return_lift isolates the intended posterior curvature and preserves the frontal edge more closely.


126 visual rejection and127 replacement: raising the whole transverse underside arch created a localized tip-like bump and small shading marks in the user's saved view. Do not retain that strategy. Restore all original underside heights and instead apply a small center-only posterior-depth bow, fading from zero at the front band to at most0.016H at the rear center. Keep its side endpoints and all frontal cage positions unchanged. This targets the tilted-view closure without narrowing the ordinary frontal tip; the reference's posterior central underside is also farther back in the sparse diagnostic. Recheck actual view and winding/clearance.
