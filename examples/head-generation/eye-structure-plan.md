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
