Only explicit brow/iris/lower-cover and nose pigment controls change from74. Treat pigment height separately from geometric nose height. Verify front render versus white view; do not claim geometry changed when only the atlas changed.

## Reconstructed81 mapping, written before implementation
- Useful76 parameters: upper lip peak height0.176H, influence width0.020H, relief0.024H; lower relief0.003H; mouth height0.155H.
- Useful79 neutral aperture: half-gap0.00065H, total0.3068mm atH0.236m; upper rim depth delta+0.008H, lower-0.004H, smoothly tapered at corners and across existing rings.
- Planned80 correction: one curved rest seam y(x)=mouth_height+corner_lift*clamp(x/half_width,-1,1)^2. Aperture, jaw classification/falloff and cavity normalization share this function. This was not executed before reset and requires fresh tests.
- Keep eye/mouth boundary crease1.0;77 changed no exported positions and is not treated as a useful geometry control.
- Do not restore78's rejected wider/deeper neutral dark band.

##82 eye-pocket mapping
Keep48 angular samples and the first3 expansion bands. Replace the old fourth rear band plus18-percent shrinking fifth ring with one full-sized terminal contour, width1.08 and height1.12 times aperture, at constant rear depth measured from eye-center rim. Rear cap has no added center pole; its planar polygon triangulation must not create geometric pinching. Preserve the evaluated-skin clearance algorithm and inspect its effect on the terminal boundary. No original reference vertices or topology are copied.

##83 mapping
Ear angular samples36→24, radial bands6→3, with relief only-0.004H at inner bands and0 at rim. Preserve the existing semantic outline, paired closed back surface, root fitting and UV isolation. Inner ear shape is moved to an independently authored analytic color function with bounded strengths rather than copied texture pixels.
Flag only the actual planar eye rear-cap polygon as flat. After joining and triangulating, custom corner normals use polygon normals on flat faces and area-weighted vertex normals on smooth faces. This preserves honest geometric shading and removes the cap's boundary-normal interpolation artifacts.

##84 edge-Crease mapping
Use a normalized-height strength profile on true front-surface center edges, with the strongest values near the nasal bridge/tip and tapered weights toward forehead/chin. Select the existing q=0.78 lateral control-cage chain for the cheek front/side boundary; reject non-longitudinal edges and preserve eye/mouth aperture creases. Record selected edge counts and evaluate matched vertex displacement with the new groups disabled as a non-reference ablation. No original source Crease metadata is available, so these are explicit case parameters subject to native visual validation.

##85 nasal mapping
Read-only reference/candidate ray measurements after84: tip-center depth is close (about0.8mm short at0.28H), lower-nose sides remain about2–3mm shallow, while upper-bridge sides at0.40H are about2.7–4.5mm too projected. These are sparse semantic observations, not copied vertex coordinates or original crease weights.
Use separate monotone nasal lateral profiles for tip/bridge and fuller base, blended across height0.25–0.28H. Increase lower-nose support width, make a small tip-height adjustment, and insert a narrow0.40H bridge cross-section while retaining outer-side values. Validate actual subdivided geometry; all numerical estimates are hypotheses until generated and viewed.

## 86 oral-volume mapping, before implementation
The native rear view shows the previous oral bag as a shallow horizontal almond; the reference has a taller, broader rear volume. Treat this as a semantic volume requirement, not permission to copy its triangulation or possible unpacking artifacts. Preserve the first two lip attachment rings. Parameterize subsequent width, backward depth and upper/lower expansion relative to H; taper less aggressively at the rear. The final cap uses a constant depth and geometric flat normals, avoiding nonplanar triangulation shading. Preserve neutral lip and jaw classification; validate volume, rear planarity and sampled skin containment before native mouth regression.

## 87 independent-iris alignment hypothesis
Read-only connected-component bounds of the isolated reference iris suggest center height approximately0.429H and vertical half-extent0.077H, versus current0.423H/0.082H. Use rounded semantic parameters center0.4295H and half-height0.077H, lateral center0.2195H. These are a calibration hypothesis, not direct vertex copying or acceptance. Keep pocket/lids/nasal/ear/oral source and texture unchanged. Compare against86; reject if new exposed edges, scleral slivers or protrusion appear.

## 88 paired lower-lid hypothesis
Candidate87 was not retained as an isolated improvement because the unchanged lower lid exposed a narrow white crescent below the raised iris. Reduce lower arch from0.094H to0.088H while preserving corner height/slope and iris87. This lifts the central lower contour by1.416mm and rebuilds its pocket/shell boundary through the same shared contour function. Test corner continuity and central iris overlap; verify side view for protrusion.

## 89 iris topology mapping before implementation
The old 13 by17 iris grid scales a whole row by sqrt(max(0.002,1-v²)); its extreme rows compress many vertices into tiny spans. Replace it with a9 by9 quad grid mapped continuously from a square to a disk: x=u*sqrt(1-v²/2), y=v*sqrt(1-u²/2), then apply the existing ellipse radii. This keeps32 boundary edges,81 vertices and64 quads per iris without a center fan or collapsed pole rows. Use unchanged analytic depth/UV mapping. Validate positive projected cell area, finite samples, boundary ellipse and minimum edge spacing; native front and oblique views decide retention.

## 90 bounded mouth morph
Read-only actual89 GLB measurement found front-chin downward motion up to15.11mm and lower-chin-region motion15.85mm. Preserve neutral geometry, replace broad lower-jaw falloff with explicit local mouth parameters and a smooth chin-preservation gate. Lower opening reduces to0.038H, upper0.0127H, corner down0.0064H; lower radial falloff0.09H and upper0.05H. Pin normalized height<=0.07H, blend to full support by0.145H. Tongue moves down0.025H, with existing small backward shift. Verify exported sparse morph displacement by region and native half/full opening; lower motion must not cross fixed chin vertices.

## 91 shared-root ear membrane
Reference audit finds exported ear components with exact coincident root points/edges against the main face, though index connectivity is split. Current overlapping closed shells do not reproduce that relation. Author a shallow, open-backed ear membrane from the actual evaluated facial boundary chain in the ear-height interval. Four lateral bands follow a rounded outward bulge with slight posterior sweep, rather than copying reference vertices. Reuse every root point; the existing runtime weld should make shared edges. Verify exact seam position and two-face adjacency after joining. Endpoint triangles remain local, not a dense central fan. Original procedural pinna pigment remains.

## 92 bounded feedback mapping
Reference top boundary falls substantially farther down at the side than the old q^4 warp. Apply a rounded semantic drop profile over a wider upper-head height interval starting0.58H; constrain its derivative so longitudinal rows remain ordered. Do not extend face creases into the crown. Reference eyebrow strip is thicker centrally with a near-linear lower edge: thickness0.012H, rise0.025H, no extra baseline bow. Neutral rendered mouth is wider than reference; reduce half-width to0.065H and shift its center to0.150H, keeping relative lip peaks and stable-chin dynamics. Sparse lip-depth rays through the open seam hit the rear cavity and are not valid surface-fit errors. Rear eye pockets use32 angular samples, front aperture plus two identical ellipse sections behind the minimum rim depth. Oral back uses4 rings with two equal rear sections at fixed depth planes. Keep root-connected91 ear membrane; validate all topology, containment and native views before retention.

耳根的第一段面片应顺着脸侧切线向后转，再逐渐放平为浅弯耳壳；避免精确接缝虽焊上、却因切线突变鼓成结块。91白模显示根部仍有鼓起感，92将后扫轮廓改为平滑正弦转向并保留同一根部边。

## 93：头顶空间弧线与耳根跨度复核
92侧面显示仅调整顶部高度还不足以得到圆滑空间弧线：顶部深度不能在降低高度后再次套用较低面部截面，否则后缘会偏前而显得像斜直边。将头顶边界的高度和前后深度分别用少量语义控制点约束，在同一宽过渡区平滑连接额头。根部耳高区间略扩大到0.26–0.55H，以免实际采样截短耳壳。保留前述眉唇、简洁后腔、共享耳根和稳定下巴口型。

## 94：保持额头前轮廓
93侧面复核发现把整段额头深度混向顶部绝对深度会压凹前额。顶部空间弧线应只施加相对于原顶部深度的差值，并向下渐隐，不能把额头中线原有凸弧拉向头顶平面。保持头顶边界目标、原前额凸度及其他本批参数，复核侧面后才保留。

## Candidate95 eye mapping
- Visible original white catchlight: explicit normalized offset(-0.030,+0.010)H from iris center, half-size(0.009,0.007)H, surface offset0.0012H,5x5 square-to-disk quad patch. White ocular palette, actual geometry, no reference pixels.
- Pocket transition setback0.004H, posterior straight length0.030H, width/height expansion1.04. Previous0.020/0.065/1.10 values are superseded. Rear remains behind iris; inspect actual clearances rather than claiming a shorter parameter proves no collision.
- Iris concavity0.014H and outward depth slope-0.31; rim depth and aperture stay fixed. No global forward translation.

## Candidate96 contour mapping
Ear outward/posterior profiles replace one symmetric sinusoidal bulge. Transverse x uses sin(pi*u/2), posterior uses1-cos(pi*u/2); rootu=0 remains exactly the evaluated shell. Lower head outer band q>.78 gains sparse width-inset and posterior-depth profiles, fading out between heights.52 and.60. This changes only the return band, not the front chin-tip or mouth operator. Measurements guide authored ratios rather than copied vertices.

## Final102 refinements
The ear transverse posterior displacement uses root_depth minus the authored outer-depth curve, so the outer rim stays smooth even when the attached root varies. Lower return depth is an additive boundary correction, not an interpolation toward absolute depth; the field fades below0.285H, beneath the actual eye patch minimum~0.297H. The unchanged topology gates rejected96/97, and native oblique white views rejected98–100 flattening. All these revisions are captured in the authored brief rather than manual Blender editing.


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
