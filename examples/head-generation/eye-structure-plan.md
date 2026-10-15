# 规划映射

首先冻结生成规范，再把其语义转换为 face_style.json 中的 calibration 配置。通用几何函数只解释控制站、开口曲线、腔体扩展及虹膜凹度。已有口腔内向绕序、形变导出和冻结角色部件沿用已验证实现。

本轮校准优先落地眼眶空间、独立虹膜、中轴鼻唇及横向轮廓。旧配方的身体装配坐标仍作为兼容层存在，并不宣称整个生成器已完全泛化。后续需把耳、口型和材质细节的剩余固定值移入模板参数。

输入只包含本规范、稀疏语义参数和既有候选基础资产。不会读取外部参考网格、贴图或邻接表，也不使用角色名触发生成。参考素材仅供独立验收。


## Revision148: structure-first owner review
Inspect white and wireframe before rendered pigment. Preserve the accepted independent eyebrow, frontal silhouette, nasal ramp, center division and 1.5 mm maximum chin follow. Both ears change symmetrically. Their lower attachment should lean anteriorly, the hollow sheet should wrap farther inward, and its independent pigment/UV should describe a broad inner fold instead of a straight faint line. Keep root connectivity, no sealed solid ear and no crossing return wedges.

The posterior lower face boundary must follow a continuous inward收束 curve from chin to ear; this is a boundary placement issue, not a painted line or global head narrowing. Independent read-only diagnostics show excess lateral extent at the rear boundary around0.10–0.24H even though the front silhouette is accepted. Increase only the far-back return inset with bounded smooth profiles. Root-depth transitions are separately calibrated around the ear attachment.

The three indicated eye-adjacent shapes are the upper lash/liner, upper-lid fold, and lateral outer liner; the upper independent brow is frozen. Add attached tapered peaks to the lash ribbon and a short tapered eyelid fold, retaining meaningful geometric layers in white mode. The lower aperture should be a rounder arc, with corner positions and upper span preserved; the user's green line is approximate, not geometry to trace.

Read-only inspection identifies a shallow annular iris region, a separate small recessed pupil patch, a planar white catchlight, plus a lower iris sheen patch sharing eleven exact boundary positions with the main iris. That lower patch is a contiguous region split by authoring/UV structure, not evidence of an extra free-floating layer. The white pocket patches likewise share fourteen and nine boundary positions. Raw mesh components are split and cannot establish an exact global count of three/four layers. Replace the single square-grid iris with a low-depth radial annular sheet surrounding an independently recessed pupil, with a separate catchlight and pupil accent. Do not make a protruding sphere or a deep solid cylinder. Original colors only; no reference vertices, UVs, pixels, adjacency, names or file paths in generator inputs.

Acceptance: inspect front/side/rear white and wire; demonstrate separate eye layers and retained shallow inset; no degenerates or new ear-root wedge; preserve frozen body/hair, mouth regression and actual source manifest. Then inspect original pigment/render, run aggregate tests and durable preservation. This is an unaccepted candidate until actual runtime checks and owner's visual review.

155 read-only reference adjacency correction: the two iris color fragments weld into ONE continuous disk with a single outer boundary, a central recessed vertex and a narrow peripheral band. Earlier148–154 incorrectly inferred an annulus and put the pupil behind a hole. Correct the original procedural iris to a shallow continuous disk, then place the separate pupil sheet in front of its central depression. Use original32-segment geometry, no imported reference indices or vertex coordinates. Inspect fan/bevel topology in white/wire and depth ordering from the side. The earlier 3/4-layer guess remains unverified as a global layer count. Upper lash peaks must lean in different directions with unequal heights, not three identical upward teeth; keep editable silhouette controls.


## Candidate178 narrow eye-accent calibration
Freeze the previously reviewed head and internal eye geometry. Replace flat ink sheets with original semantic thin rolled bands: bounded longitudinal bow, front ridge and finite return, unequal connected peaks, a shaped lateral band with short attached accents, and an asymmetric upper-lid fold. Diagnose white/wire before pigment. Do not reproduce unpacking holes or read any reference asset in the generator. Independent eyebrow, aperture, iris, pupil, catchlight, ear, nose, mouth and chin remain fixed.


## Owner refinement 179: attached, layered eyelash sheets
The owner reviewed 178 and asked to continue refining the eyelash/eye-edge features. Keep the accepted aperture, iris, pupil, highlight, independent eyebrow, face, ears and mouth unchanged. The upper dark band must sit close to the actual upper eyelid surface in top and oblique views rather than hover in front of the eye. Its sculptural effect comes from a modest folded sheet and unequal swept tips, not a large global stand-off. The outer vertical accent needs a narrow curved primary strip and a finer separated companion branch, with a small intermediate tip and a long legible slit; do not fill the slit into a broad three-toothed plate. Add the small isolated downward lower-outer lash indicated in the red-box wire image. Do not reproduce reference unpacking fragments. Review wire and white first, then front/oblique/top render under equal settings.

Diagnosis: the previous planar ink depth is about 7.8–9.4 mm forward of the upper eyelid at five sample stations before adding bow/relief. The next original recipe binds the return side to the existing skin-lid surface and bounds local relief. No reference asset data enters generation.


## Evaluated-surface attachment refinement180
179 removes the gross planar stand-off, but actual white-mode review reveals local upper-band burial against the subdivided skin. Analytical depth alone is insufficient. Bind only the existing accent return rails to the evaluated generated skin and generated eyelid surfaces, preserving thin-sheet offsets, branching and all protected geometry. Record the maximum correction and sampled attachment clearance. No reference geometry is used.


## Attached-root folded free edge181
180 removes local skin intersections, but white/top views still have less readable broad folded-strip relief than the reference. Keep the upper rail attached, permit the lower free edge a bounded1.6mm forward flare, and retain only0.12mm local ridge relief. This is a folded sheet with an attached root, not a global translation of the band. The central tuft leans toward the outer corner. Actual multiview review remains required.


## Root orientation182
Actual181 white-mode comparison shows the flare was assigned to the wrong rail: the lower eyelid-facing boundary is the lash root, while the upper swept edge is free. Keep the lower root surface-attached and flare the upper free edge by the same bounded1.6mm, with the evaluated-skin collision correction retained. The accepted eye and face are unchanged. Verify top/oblique contact and a readable continuous folded face rather than an isolated thin bright lower rim.


## Continuous narrow sheets183
182 close render reveals that the integrated central tooth looks like a thick diamond, and the outer sheet has small disconnected-looking fragments. Preserve attached roots, separate the main upper sweep from one narrow curved overlapping tuft instead of swelling the entire main ribbon into the tuft. Subdivide only longitudinal gaps larger than the declared curvature sampling bound so the generated-surface fit cannot leave long flat chords cutting through skin. Keep thin return faces, an open lateral slit, and the isolated lower tuft. These are local eye-accent corrections; protected eye and head geometry remain unchanged.


## Oblique free-tip depth184
183 actual45-degree render exposed an overlong outer spear and a diamond-like side loop. The cause is evaluating the far-corner free tips and companion blade independently on rapidly changing cheek/orbital depth. Continue the local lid depth beyond the corner with a bounded support station; shorten the outer free tips, and give each lateral station a shared local support depth across its width/companion branch. Keep evaluated-skin collision checks, thin returns and the intentional slit. This does not change the accepted underlying eyelid or face.


## Thin-sheet finish185
184 removes the overlong oblique spear. Thin the return walls and ridge relief so intersecting layer roots do not read as beveled blocks. Keep visible depth through separate curved overlapping sheets, bounded1mm free-edge flare and the intentional side slit; the skin-facing roots remain attached. Preserve the lower isolated tuft and all accepted underlying head and eye geometry.


The thin-sheet geometric audit caught a real defect before execution: the five-rail cross-section sampled the curved front at its ridge but spanned the back with one unbroken chord. At thin thickness that back chord can cross the curved front and reverse signed volume. The corrected six-rail section samples the back at the same ridge station, so front and return have matched curvature. Keep the strict front-facing/closed-volume checks; do not reverse normals to hide a self-intersection.
