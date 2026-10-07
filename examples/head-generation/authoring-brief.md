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


## Revision 137: bounded chin follow and ear-root return
The user now requests a small positive chin drop and slight facial elongation with MouthOpen, replacing a fully fixed chin as final intent. Add a separate adjustable 1.5 mm maximum downward lower-face follow, with smooth height and lateral falloff; preserve the existing lip opening and anti-folding gates. Verify neutral remains identical and the open pose is far below the early excessive jaw drop.

Preserve accepted single-sheet inward-wrapped ear construction. The circled black wedge is localized at the lower ear-root return, not a full-ear shadow. Reduce return width smoothly near attachment endpoints so the free return border does not poke into the adjacent cheek. The broad cup remains. Refine original pigment into a narrower inner fold and upper arch with controllable sharpness, avoiding the old blurred thick ring. This iteration does not claim the remaining jaw corner, complete ear contour or eye-layer changes are finished.


## Revision 138: rounded aperture and distinct eye-line layers
The user describes the reference as rounded-corner and slightly narrower below, not a literal rectangle. Use a smooth superelliptic horizontal aperture exponent, independently controlled upper/lower arch powers, and retain shallow recessed eyes. Preserve the original iris and catchlight geometry. Keep eyebrow, rose upper-lid crease, thick upper lash ribbon and outer vertical liner as separate semantic features. Replace the scattered lower lash triangles with a coherent tapering outer liner, and keep the upper accent attached to the actual rim-depth surface. Do not reproduce broken reference brow triangles or apparent extraction defects. Verify frontal and side views and pocket clearance after this change.


## Revision 139: locally fair the posterior jaw boundary
The user marks a small remaining tangent break where the bottom rear edge turns into the side edge at 90 degrees. The large spur was removed, but sparse rear-return boundary samples still produce a corner after subdivision. Apply a bounded, distance-weighted fairing to only the low posterior open authoring boundary, before mirror/subdivision and normal calculation. Keep frontal chin/lips, eye/ear regions and all topology unchanged. Do not smooth accepted central shading normals away or hide the corner through lighting. Verify both exact side views at enlarged scale; a small residual corner is still a failure.


## Revision 140: multiview pinna contour and orbital influence limit
Sparse front/side sections show the generated lower-middle ear too broad and the upper posterior crown too far back, despite similar frontal peak width. Do not fit an ellipse: taper the lower envelope, trim the upper crown and return its depth toward the attachment. Preserve the accepted inward-wrapped structure and end-faded root. These are independent semantic controls; no reference geometry is used by generation.

Read-only diagnosis also finds the orbital blend reaching the outer facial boundary beyond the actual eye corner: at normalized side height about0.44, the generated root is roughly0.03 H too far forward. Limit orbital blending smoothly outside the lateral eye corner rather than compensating by distorting the ear. This restores the posterior facial trajectory, with eye-front clearance and side clipping checked afterwards.

Actual138 front-view review rejected the protruding C-shaped outer liner. Keep the rounded aperture but place the liner inward over the outer white margin, leaning toward a lower tapered endpoint; do not build a free external loop. Verify face winding after reversing the strip direction.


## Revision 141: normalized rear cavity proportions
Actual back white-model comparison shows the generated eye backs vertically flattened and the oral back smaller than the reference, independent of the dark back-outline rendering. Increase orbital rear ellipse height modestly and shift its center upward, retaining the original aperture and shallow front recess. Keep posterior depth inside the previously established short-pocket budget rather than reproducing a deep tunnel. Enlarge the oral middle/back ellipse while retaining a gently contracting planar rear cap, tiny neutral lip seam and inward-facing walls. Do not copy rough triangulation or nonplanarity from the unpacked reference; preserve containment and rear-planarity assertions and check actual side/back/open-mouth views.

137 left-oblique review reduced the root wedge, but139 exact opposite-side review still exposes a dark root triangle. Shared ear-root vertices retain separate regional corner normals, which can diverge under shader outline extrusion. Use one area-weighted geometric normal at the actual welded attachment vertices on both face and ear; preserve all other normal partitions, especially the accepted facial centerline. Record the matched root count and inspect both sides before claiming resolution.

140 matched only15 of28 attachment vertices using rounded coordinate keys. Use nearest actual welded vertices with a2-micrometre guard and exact expected-count assertion to cover both attachments reliably, then apply common geometric root normals.


## Revision 142: correct the direction of the pinna return sheet
Complete common-root normals did not remove the exact-side black root triangle on141; the normal-seam hypothesis alone is insufficient. Cross-section inspection finds the return was moving inward laterally but farther posterior in depth, retaining a front-facing lateral normal. The read-only reference reverse patch has predominantly opposite lateral normals and returns anteriorly toward the attachment. Correct the original return path to travel inward and anteriorly, preserving the open free edge and the user-approved wrapped-sheet construction. Add an area-weighted reverse-lateral-normal assertion, not a texture mask or disabled shader outline. Keep the endpoint fade, multi-angle silhouette parameters, complete root-normal matching and all other improvements. Check both exact sides and the white back view before claiming the black wedge fixed.


## Revision 143: prevent the lower ear membrane folding forward
The exact-side black triangle persists on142. Read-only exported-triangle inspection identifies negative lateral normals in the LOWER FRONT membrane, not merely its return lip. The independent posterior contour lies anterior to the actual skin attachment near the lower endpoint; this reverses the visible sheet. Preserve the measured silhouette target where feasible, but enforce a parameterized nonnegative posterior clearance from the actual root at all heights. Keep lateral width and the existing open wrapped construction. Verify all front triangle normals around the lower root in the generated mesh and inspect the actual NPR render. The return-normal fix alone is not counted as black-triangle resolution.


## Revision 144: contain the return lip inside the actual ear basin
143 removes the lower front-sheet reversal but a thin black return wedge remains at the endpoint in exact-side NPR. The return depth still uses a fixed budget larger than the local basin depth, crossing anterior to the attachment. Bound its anterior travel and inward span to 65% and75% of the actual local basin dimensions respectively. This preserves the free back edge without piercing the front/root envelope. Validate both exact sides and oblique views.


## Revision 145: oblique-view return containment
144 clears the exact-side black wedge but a residual reverse-face patch is visible at45 degrees. Independent width/depth fraction limits still allow the return to sit in front of the curved basin. Retain at least80% of local posterior depth and50% of lateral span so the free return edge stays behind the front sheet across oblique viewing. These explicit bounded parameters remain configurable. Recheck both side and45-degree renders before acceptance.


## Revision146: five scoped refinements after145 owner review
The owner accepts the overall direction and asks for ears, eyes, brows, a localized concavity in the frontal facial silhouette, and nasal bridge-to-tip versus tip-to-lip slope calibration. Other accepted form, centerline, wrapped ear topology and small chin follow are frozen as targets. This is a new unaccepted candidate;145 is not a result of this new request.

Sparse original/reference cross-section diagnostics show the candidate half-width is short by about0.014H at height0.15H while the nose-tip maximum at0.28H is already close. Apply limited lower-mid contour support, not global face inflation. Lift upper nasal ramp slightly at0.32–0.36H, pull the bridge shoulder back around0.40–0.45H, preserve the tip and lower lip system. Measure resulting profile after subdivision instead of treating requested steepness as an unspecified coordinate slope.

Use a slightly fuller lower ear and deeper but bounded cup, keeping the corrected posterior sweep and contained return. Independently authored pigment has a lighter, broader upper arch and a restrained curled inner fold. Do not load reference texture or geometry. Refine eyebrow center/thickness toward the healthy sparse reference ribbon, excluding unpacked broken triangles. Replace detached outer lash triangles with a contiguous tapered four-corner wing; make the pupil smaller/rounder while keeping the accepted catchlight and shallow recessed eye. Check native views before any success claim.


## Revision147: decouple side-jaw fairing from frontal width
146 improves the measured nasal ramp but the frontal low-cheek transition still has a local inward step. Comparing136/145 at the same sections identifies side-boundary fairing as an additional cause: it reduced lateral width by0.006–0.008H around heights0.15–0.18H while fixing a SIDE silhouette corner. That side-view task only needs depth/height smoothing. Preserve lateral x during fairing, with a separately explicit lateral relaxation parameter default0, while retaining depth/height smoothing. Reduce the compensatory width increase at0.12H and add a gentle0.16H support station. Do not undo the accepted side-jaw smoothness or inflate the whole face. Verify actual frontal and exact-side curves after generation.

The147 ear pigment also connects the upper arch and inner fold with a short, bounded, independently authored curved fork; this avoids the previous lone C-stroke appearance while preserving light lobe color. It is procedural color, not imported reference pixels.


## Revision148: structure-first owner review
Inspect white and wireframe before rendered pigment. Preserve the accepted independent eyebrow, frontal silhouette, nasal ramp, center division and 1.5 mm maximum chin follow. Both ears change symmetrically. Their lower attachment should lean anteriorly, the hollow sheet should wrap farther inward, and its independent pigment/UV should describe a broad inner fold instead of a straight faint line. Keep root connectivity, no sealed solid ear and no crossing return wedges.

The posterior lower face boundary must follow a continuous inward收束 curve from chin to ear; this is a boundary placement issue, not a painted line or global head narrowing. Independent read-only diagnostics show excess lateral extent at the rear boundary around0.10–0.24H even though the front silhouette is accepted. Increase only the far-back return inset with bounded smooth profiles. Root-depth transitions are separately calibrated around the ear attachment.

The three indicated eye-adjacent shapes are the upper lash/liner, upper-lid fold, and lateral outer liner; the upper independent brow is frozen. Add attached tapered peaks to the lash ribbon and a short tapered eyelid fold, retaining meaningful geometric layers in white mode. The lower aperture should be a rounder arc, with corner positions and upper span preserved; the user's green line is approximate, not geometry to trace.

Read-only inspection identifies a shallow annular iris region, a separate small recessed pupil patch, a planar white catchlight, plus a lower iris sheen patch sharing eleven exact boundary positions with the main iris. That lower patch is a contiguous region split by authoring/UV structure, not evidence of an extra free-floating layer. The white pocket patches likewise share fourteen and nine boundary positions. Raw mesh components are split and cannot establish an exact global count of three/four layers. Replace the single square-grid iris with a low-depth radial annular sheet surrounding an independently recessed pupil, with a separate catchlight and pupil accent. Do not make a protruding sphere or a deep solid cylinder. Original colors only; no reference vertices, UVs, pixels, adjacency, names or file paths in generator inputs.

Acceptance: inspect front/side/rear white and wire; demonstrate separate eye layers and retained shallow inset; no degenerates or new ear-root wedge; preserve frozen body/hair, mouth regression and actual source manifest. Then inspect original pigment/render, run aggregate tests and durable preservation. This is an unaccepted candidate until actual runtime checks and owner's visual review.

Implementation checkpoint148 isolates eye layers and ears first. Rear-boundary diagnostics are preserved; large existing-band inset violates positive lateral ordering and would narrow the accepted front silhouette, so it is NOT applied in this candidate. A connected posterior return strip must be evaluated separately before final batch closure.


## Revision149: posterior return without shrinking the accepted front
A stronger deformation of the existing side band fails positive lateral ordering and changes the accepted frontal width. Instead extend only the actual lower posterior open boundary with a connected inward return strip. Original exterior vertices stay fixed, outer rim crease preserves their boundary behavior through subdivision, and new vertices remain inside the old lateral envelope. A short posterior offset gives correct back-facing winding without a coplanar overlap. The end fades back into the existing rim below the ear root; no independent floating outline. Inspect white back/side and full mouth motion to detect inversion or attachment gaps.

Native148 white/wire confirms separated radial iris, recessed pupil and catchlight, but frontal lash peaks and lid-fold width remain visually weak. Increase attached ribbon thickness and peak span modestly, enlarge lateral liner, and soften crease pigment. Do not change the accepted independent eyebrow.

149 failed the existing front-normal assertion for the enlarged outer lash wing; no model was delivered.150 replaces the potentially self-crossing four-corner wing with an attached three-corner taper and preserves explicit bilateral winding. The posterior return fades to zero before the ear-root interval, preserving the actual shared root chain instead of shifting its endpoint. All generated outputs remain creation-only.

151 corrects a native150 eye-stack occlusion found after structure checks: the inherited upper skin lid extended0.028H into the aperture while the lash ribbon was only1.1mm thick, exposing skin crescents above the iris. Keep the aperture contour fixed, shorten hidden skin coverage to0.010H and use a2.3mm attached lash band to cover its edge. This is an actual layer overlap correction, not an eye-wide scale change. Verify front and oblique views before completion.

152 follows actual151 front-view verification: the inherited rim-only lash depth buries upward tapered peaks in the adjacent skin. Place each ink vertex just above the actual local skin or old rim, whichever is forward, using its actual adjusted height. Preserve the aperture. Replace the overly regular eyelid-fold arch with four explicit normalized height controls for a tapered, gently angular plateau. This correction must pass native front/side/white/wire inspection rather than assuming code changes are visible.

153 final scoped calibration after152 actual front view: the peaks are now visible but the upper ink ribbon remains thinner than the measured reference upper-liner region (height span about0.046H). Use a4mm band and4.5mm maximum attached peak while retaining the unchanged aperture and independent brow. Sharpen the independently drawn inner-ear fold core without copying pixels. Reduce the lowest rear inset after sparse section analysis showed an inward overshoot at0.06–0.10H, preserving the successful mid-jaw correction and unchanged original exterior vertices.

154 is required by actual153 oblique WHITE-mode inspection: per-vertex max-projection onto skin creates a warped upper ribbon with visible slivers despite frontal render improvement. Replace this heuristic for the upper liner and its attached wing with one shallow sloped semantic plane, following independently measured upper-liner placement. Keep lower/outer lateral liner at the validated rim depth. This yields coherent sheet geometry rather than using pigment to conceal folds. Verify oblique white, exact side and render before closing the batch.


## Candidate155: diagnose before repair after owner rejected154
The six marked views identify actual geometry relationships, not a general request for extra detail. Preserve accepted independent brows and mouth follow. The current ear lower root remains posterior because the generator conflates the open rear face boundary with the anterior ear attachment. Read-only shared-position diagnostics show distinct anterior attachment and posterior return chains in the reference, meeting near the lower lobe. Its back is a broad wrapping sheet, not our narrow return strip. Current return-depth cap0.20 is an obsolete recipe assumption, not a safety requirement.

First repair the ear anterior root profile and fuller return with original semantic controls. The lower root must lead forward relative to the middle attachment. Maintain one connected root chain, monotonic surface progression, a hollow free back edge and no third face at a root edge. Broaden the reverse-facing sheet while keeping it separated from the front basin. Verify side, rear and oblique white/wire before judging pigment. Revisit the posterior jaw-to-ear return transition after this structural correction.

The eye review remains OPEN: current three equal pointed teeth do not match the tapered multi-direction upper liner. The pink fold and lateral purple strip require independent silhouettes, and lower aperture and actual iris/pupil/highlight layers need matching white/wire views. Do not declare the whole review resolved by technical tests or by this first ear-only candidate.

155 read-only reference adjacency correction: the two iris color fragments weld into ONE continuous disk with a single outer boundary, a central recessed vertex and a narrow peripheral band. Earlier148–154 incorrectly inferred an annulus and put the pupil behind a hole. Correct the original procedural iris to a shallow continuous disk, then place the separate pupil sheet in front of its central depression. Use original32-segment geometry, no imported reference indices or vertex coordinates. Inspect fan/bevel topology in white/wire and depth ordering from the side. The earlier 3/4-layer guess remains unverified as a global layer count. Upper lash peaks must lean in different directions with unequal heights, not three identical upward teeth; keep editable silhouette controls.

156:155 side WHITE still lacks the reference rolled outer ear rim. Section diagnostics show a steep root, a broad middle basin, and a short steep outer return; the previous single sinusoidal sweep instead spreads curvature across the whole ear. Replace with independent original normalized outward/depth profiles, add an explicit outer support row and preserve monotonicity. Rear wrap must extend medially beyond the anterior attachment width while staying posterior: a width fraction below1 incorrectly prevents this. Restore shallow anterior back-return depth and increase medial wrap, rather than pulling the entire back sheet toward the front.155 continuous iris topology is retained.

157 eye-adjacent silhouette correction: the owner circled the graphic upper eyelash band, pink lid fold and purple lateral edge, not the independent eyebrow. Replace repeated parameterized teeth on an arch with an original editable simple polygon: unequal directional tips, tapered inner corner, broader outer flare and a separate shallow purple outer wing. Keep a coherent sloped thin-sheet depth, not a projected warped ribbon. Pure tests verify bilateral winding and no outline self-intersections; white and wire must verify the actual sheets before rendered review.

The156 side white view confirms the new rim is readable, but the lower posterior transition still has a separate notch. Connect the first lower back-return edge to the adjacent existing facial boundary edge with one original rear-facing bridge triangle per ear. This closes the missing lower wrap transition without adding a third face to the anterior ear-root chain or moving the accepted front outline. Verify the new shared lower seam and rear silhouette.

158:157 front render shows an overlong blunt inner lash extension; shorten the inner semantic outline and keep unequal tapered tips. Measured candidate ear sections remain too wide around the lower/middle lobe; slightly reduce outward span and increase upper/lower asymmetry. The rear wrap is too medial at upper heights after applying a constant width fraction. Use a separate height-varying wrap profile: fuller lower wrap and progressively shallower medial return toward the top. Preserve the now-readable steep outer rim and lower rear bridge. Original pigment is sharpened only after white geometry review.

159 closes the remaining marked rear contour discrepancy observed in158 rear render: the innermost posterior chin row is almost horizontal, leaving a flat U instead of the reference rounded V-like收束. Lower only the center of the last posterior chin row with a smooth bounded profile, preserve its lateral anchor and all earlier/front rows, and reduce excess rear-return lift. This is a geometric rear-boundary change, never a painted line. Preserve1.5mm mouth follow and verify side white does not create a new spur.

161: actual160 front preview rejected the broad lateral-wrap experiment because it introduced cheek indentations. Restore159 lateral geometry and strict frontal winding; do not keep160 just because its pure tests and native import passed. Diagnose the local depth profile instead: the0.20–0.26H posterior-depth valley sits behind the forward lower ear root, creating the visible oblique bulge. Fair only this local depth valley into the unchanged lower root, preserving all lateral width stations and the anterior face. Check actual front silhouette as well as side/oblique before accepting this hypothesis.160 remains rejected experiment history.

162:161 preserves the front but the left45 silhouette retains an overly vertical shoulder immediately below the ear. Continue the local depth fairing down to0.15–0.20H, allowing a continuous diagonal收束 into the unchanged lower ear anchor. Keep lateral widths and all161 facial-feature geometry. Judge actual side and oblique silhouette together, not a single sparse rear-boundary number.

163 scoped mouth diagnosis:19actual10-degree baseline views show insufficient lower-lip roll and excessive upper/lower edge depth separation. Restore a small continuous lower-lip bulge, bring lip aperture edge depths closer while retaining slight upper prominence, and reduce the lifted smile-like corners. Preserve eye geometry, pupil disks, accepted independent brow and all other162 controls. This first experiment changes only mouth relief; chin underside and other owner requests remain pending and must not be called solved. Compare neutral0/±10...±90, then mouth morph clearance, before accepting.

164 follows the actual19-angle close review of163: lower-lip central depth is now slightly excessive and its broad flat lateral envelope still lacks a rounded side roll. Lower the mound center and give upper/lower lips separate Gaussian lateral widths rather than one super-Gaussian plateau. Keep the aperture width and tiny neutral gap; only change relief footprint. Separately fair the posterior jaw boundary across a taller local band with more bounded iterations; preserve zero lateral relaxation to protect the accepted front silhouette. Inspect60-degree lip outline and50–90-degree posterior chin turn in actual white/wire and rendered views. All earlier ear and eye-accent work remains pending.

165:164 failed the existing local jaw displacement bound and produced no valid model. Retain the bound: uniformly scale the proposed posterior-boundary fairing displacement field to at most0.018H instead of allowing the8-iteration request to exceed0.02H. Uniform scaling preserves relative smoothing weights; report proposed/applied displacement. Do not remove or loosen the assertion. Also correct the separately diagnosed mirrored ear-bridge winding: preserve directed root/rim order rather than a world-y normal heuristic. Recheck opposite edge directions and actual black triangle appearance; other ear richness and three accents remain pending.

166:165 passed native validation and removed the measured ear-bridge winding conflicts, with the prior side black triangle absent in the inspected view. Its posterior chin edge still has a visibly straight run and hard corner. The posterior return currently assigns crease1.0 to its open rim: test a localized0.35crease there while retaining facial center/eye crease groups and jaw corner-normal controls. Preserve actual mouth geometry from165. Render-only mouth seam color should be authored procedurally around the existing UV seam with narrow fading coverage; it does not replace the lip surface, cavity or shader outline. This addresses the broken gray subpixel neutral seam separately from geometric relief.

167 diagnoses a geometric chain conflation rather than merely a crease: front ear attachment was advanced, while the existing posterior jaw strip still has only a uniform0.007H return. Independently deepen that posterior strip with a bounded height-weighted profile, leaving front face/ear-root sampling and accepted eyes unchanged. Shorten only the last bottom return depth modestly and let the inner posterior ear sheet retain more of the outer bowl depth so the open edge can rise smoothly toward it. Do not globally change lateral widths or repeat the rejected160 front U-turn. Verify back opening, side silhouette, directed seams and oral clearance.

168:167 improves mid-jaw posterior depth but exposes an unacceptable notch under the ear in actual90-degree render. Do not accept167 visually. Read-only patch bounds identify why: the lower pinna outside sheet must extend much farther posteriorly than its anterior root; the current sine-tip membrane collapses that lower depth too quickly. Introduce original asymmetric lower-arc fullness and a bounded lower-posterior sweep while keeping the fitted anterior roots fixed. Coordinate the below-ear posterior strip using a smoother weight profile. Validate winding and no front/side notch; do not hide the notch with shading.

169:168 still exposes an ear-underface notch in actual side view. The last posterior return control interval goes directly from0.233H to0.271H while its inset/offset fades tozero at0.26H, so subdivision retracts the last open-boundary neighbor too far anteriorly. Add one meaningful whole quad-ring support level at1.6045m between1.599 and1.608, keeping eye and mouth patch anchors selected by height.3.5–5.5mm spacing is intentional ear-root curvature support, not duplicate near-coincident edges. Extend the posterior inset taper to the existing anterior root height instead of ending between unsupported rows. Reassess the accepted eye aperture and pupil geometry,front silhouette and neutral mouth before keeping the new support. Do not accept a mere numerical depth improvement if the notch remains.

170: Actual169 side-angle review shows the ear-underface notch substantially reduced but a lower posterior jaw angular transition remains. The current fairing occurs before adding the posterior return strip, so the newly created exposed boundary has not been faired. Add a separate bounded relaxation of that final posterior boundary before subdivision and ear-root fitting, preserving x positions and all accepted front ocular geometry. Limit this additional adjustment to0.008H (1.888mm), below0.27H and behind -0.12H; record it independently. Keep mouth geometry, eye aperture, iris, pupil and existing center/side creases unchanged. Inspect white/wire and every10degree render; procedural tests alone do not accept appearance.

171: Refine the three explicitly rejected eye-adjacent accents as thinner curved shapes while preserving the independent brow and accepted aperture/iris/pupil geometry. Replace the block-like upper lash with an original arched tapered outline, refine the outer corner accent, and narrow the upper fold. Enrich original ear pigment with coherent fork/helix/lower-lobe transitions rather than random strokes, keeping the hollow sheet topology and bridge winding. The169 19-angle comparison and sparse diagnostic also show the lip seam slightly too shallow, upper relief too low and lower peak slightly too high: deepen neutral seam by0.002H, raise upper relief by0.004H and lower lower-lip relief center by0.005H. These are bounded semantic adjustments; preserve narrow rest gap and1.5mm chin follow. Review geometry before rendered style and do not call pure tests aesthetic acceptance.

171 additional170 side-view finding: the remaining lower-posterior silhouette knee is produced by the center of the last underside station, which is intentionally excluded from lateral boundary fairing. Reduce that station’s center drop from0.018H to0.006H (2.832mm upward change), leaving anterior chin and mouth-follow amplitude fixed. This targets the actual silhouette source rather than raising the local fairing displacement limit. Verify both profiles and front contour.

172:171 removes the posterior chin knee and improves the three eye accents, but the exact10degree contact comparison still shows flatter lip definition near±60degrees. Read-only sparse lateral surface diagnostics identify the cause: center lip depth is within about0.5mm, while shoulders at0.02–0.04H lateral remain about0.8–1.8mm too recessed. Do not push the center lip outward indiscriminately. Broaden the original upper/lower Gaussian lip relief and add a small center-faded shoulder support, bounded by0.003H. Keep neutral seam, independent brow, accepted eye aperture/pupil, ear topology, nose and chin motion unchanged. Validate the new local surface field and repeat native angular inspection. Reference vertices are diagnostic only, not production data.

173 controlled local experiment:172 improves measured lip geometry but the±60degree rendered ridge remains weak. Reference read-only normal diagnostics show separate lip/cheek normal islands; do not copy their custom normals or assume extraction artifacts are desired. Test geometry-derived corner-angle weighting in the mouth neighborhood instead of area-dominated averaging, with smooth bounded blend and the existing center partition intact. Outside the localized mouth region preserve normals. Compare actual outline and white surface before retaining. Separately, rear white/wire review shows the pinna return as a wide flat paddle instead of an angled wrap. Keep its outside contour/root fixed; narrow the lateral return cap from1.35 to1.15 of span and increase its anterior fold-depth fraction from0.12 to0.26, retaining open basin and correct directed bridge. Inspect both sides and the opening curve; revert any regression.

174:173 corner-angle normal ablation produced no useful60degree outline improvement; revert that experiment and retain geometry-derived area normals. Lateral samples explain why matching a few depths is not sufficient: the lower-lip shoulder needs a controlled rolling edge whose tangent becomes locally steeper before joining the cheek. Replace broad Gaussian lateral relief with an original quartic falloff: lower half-width0.052H, upper0.075H, preserving center position within the bounded submillimeter correction. Increase lower/upper relief modestly to0.0112H/0.031H. Keep the small shoulder support and narrow neutral seam. No drawn mesh outline or copied reference normals. Verify the actual continuous lip ridge across±50–70degrees, not just scalar depth or shader settings.

175 topology correction: Read-only triangle-facing diagnostics locate a small folded surface triangle at the mouth corner (front-facing skin has a locally reversed projected winding). The mouth patch maps an asymmetric outer rectangle to evenly spaced inner angles, so index spacing does not match the geometric direction of each boundary vertex. Replace that index-based correspondence with monotonically ordered polar directions computed from the actual authoring boundary. Use the same angles for aperture placement and depth offsets; keep shared boundaries, oral attachment, neutral gap and small motion. Validate all projected patch quads and both mirrors, then actual generated skin winding and angular renders. This is an original procedural topology fix, not a reference mesh transfer.
