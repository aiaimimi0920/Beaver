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


## Brow asymmetric taper128
Read-only sparse eyebrow cross sections indicate the reference's inner tenth is thicker than the outer tenth (about0.007H versus0.0037H), while the current symmetric sine-width brow has equal endpoint-region thickness. Preserve the existing brow center path and attachment depth. Add a bounded inner-fullness parameter to its crosswise width so the inner body is fuller and the outer tail tapers more finely. Offset both lanes around the unchanged center path rather than raising the whole brow or changing brow-eye spacing indiscriminately. Keep both ends closed and retain smooth interpolation. Verify actual front and user-saved pitch view; do not imitate any unpacking-related broken triangles in the reference eyebrow.


Underside isolation refinement: fade the added arch from zero at the first posterior band to full at the rear-most lift0.03 band. The126 projection improved the tilted view but its near-front subdivision also narrowed the ordinary front tip. Keeping the first underside band at the original chin_return_lift isolates the intended posterior curvature and preserves the frontal edge more closely.


126 visual rejection and127 replacement: raising the whole transverse underside arch created a localized tip-like bump and small shading marks in the user's saved view. Do not retain that strategy. Restore all original underside heights and instead apply a small center-only posterior-depth bow, fading from zero at the front band to at most0.016H at the rear center. Keep its side endpoints and all frontal cage positions unchanged. This targets the tilted-view closure without narrowing the ordinary frontal tip; the reference's posterior central underside is also farther back in the sparse diagnostic. Recheck actual view and winding/clearance.


## Iris depth and lower reflection129
In the user's saved matched-light view, sampled candidate iris patches have a lighter upper region and darker lower reflection than the reference. Treat the samples as diagnostic rather than pixel-transfer targets. Increase the original iris's controlled top-to-bottom contrast: darken its upper pigment, brighten a soft lower crescent, and reduce the strength of the flat lower graphic disc. Preserve pupil proportions, iris depth/concavity, existing white glint geometry and highlight sizes in this step. Wateriness should emerge from depth, reflection layering and corrected lid coverage rather than protruding eyeballs or oversized white dots. Add a bounded reusable iris_depth_contrast control and test its intended directional effect plus finite color bounds.


## Original ear fold130
The user rejected the oval-looking ear pigment on123. Retain the corrected side-plane UV and the asymmetric pinna, but reduce broad concha redness, soften the upper arch and draw a shorter inner fold with a gentle lower curl toward the attachment. Raise the upper arch within the existing ear patch and keep the lower lobe mostly plain skin. Use explicit original pigment parameters rather than the reference pixels. Verify that the result reads as a shallow stylized ear rather than an oval red mark or a long stripe.


Final view calibration130: the saved-view brow diagnostic shows the reference has slightly more middle arch and a subtly rising projected outer end, while the candidate outer end descends slightly. Apply small independent authored brow rise/height/arch adjustments, retaining asymmetric thickness and attachment. For the posterior underside depth bow, use a smooth rounded-V crosswise profile with finite center curvature instead of a broad quadratic plateau. Preserve its0.016H maximum depth, unchanged side/front attachments and all original underside heights. This targets remaining saved-view flatness without collapsing any vertices or changing ordinary frontal tip width.


## Open upper helix131
Side review of130 still shows too much closed oval pigment. Keep the same original geometry and color palette. Restrict the helix stroke to the upper part of its ellipse, using the authored arch-center height as the gate, so the lower half remains open and the separate curled inner fold supplies the descending contour. Do not paint a full closed ring. Verify actual side and saved front-tilt views; all other four-item corrections remain unchanged.


## Revision 132: wrapped open pinna
User review identifies the ear as a three-dimensional wrapped sheet, readable in untextured white view, with an open rear. The previous duplicate front membrane offset by 0.2 mm is not that construction. Use one connected concave front sheet with an asymmetric upper-full/lower-small outline; roll the outer rim inward into a distinct short return sheet, leaving its inner rear edge open. No full-area duplicate reverse membrane, no cap, no solidified slab. Preserve welded face-root positions and original painted detail. This isolated ear iteration freezes all other 131 geometry, shader normals and mouth behavior. Side-face silhouette calibration follows in its own measured iteration, including the pointed rear jaw boundary.


## Revision 133: measured profile envelope
After the isolated ear change, calibrate the complete side silhouette. At normalized heights 0.08–0.15 the prior jaw back boundary was 0.034–0.039 H too far forward, while the nose-tip projection at height 0.28 already agreed. Correct the posterior jaw trajectory without pulling the frontal chin into a point. Add lower/upper lip-support volume and a shallow glabella-to-nasal transition instead of a flat face plus isolated protruding nose. Preserve mouth aperture, morph fixation and accepted central normal division. Use a small explicit front/back semantic station list, not reference mesh sampling in the generator. The last underside return must remain inside the continuous rear-jaw envelope; inspect the user-circled rear lower corner after subdivision.


## Revision 134: posterior lower-jaw envelope continuity
133 improved lip support and nasal bridge recession while keeping the nose tip projection; its side render still shows the circled tiny rear-bottom spur. Read-only section audit finds posterior deficit at heights 0.04–0.12. The low-height cross-section table still blends the old shallow back profile with the new semantic envelope. Correct that low station and lower-cheek width, leaving the frontal chin tip and underside height fixed. This is a parameter-source correction, not a post-export mesh patch. Compare left and right side silhouettes, then inspect front and mouth morph.


## Revision 135: underside endpoint meets the jaw envelope
134 audit shows the rear underside endpoint, not the cheek station, still controls the small low-height spur: changing the upper rear curve does not move the 0.025–0.04 H silhouette. Raise and extend the final underside station along the intended posterior jaw trajectory, keeping the front lip/chin tip fixed. Fade the under-chin center-depth bow to zero at the final open rear edge, so it cannot project a separate central spike behind the side-return edge. Verify the whole side silhouette after subdivision; no claim of success from parameter values alone.


## Revision 136: retain the smooth join, calibrate underside arc length
135 removes the observed posterior spur in the actual side render. Its lower underside turns upward too early compared with the reference envelope. Increase the middle underside depth stations and lower the final lift modestly; keep the free rear edge center-depth bow at zero using the actual final-station height. Preserve the flat-tip-free smooth join, front chin, ears, mouth and accepted normal division.
