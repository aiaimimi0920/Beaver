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
