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
