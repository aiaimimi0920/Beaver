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


## Revision 204: continuous forward-facing lash sheet
The full 203 sweep still shows a rounded shelf following eyelid depth instead of a broad forward-facing thin wing. Keep the attachment rail fixed and blend only the free upper edge toward a shared anterior envelope, bounded to 5 mm. Preserve the independent eyebrow, aperture, iris and all other face structures. Shorten the high tuft slightly and preserve its attached taper. Inspect white and wireframe first at every 5 degrees; require root continuity, no skin/iris intersection and no new solid wedge. This is a hypothesis until native generation and visual review.


## Revision205: fix the sheet normal through geometry
204 front and 90-degree white views reveal the main front surface tilts downward: its lower edge was anchored and its upper edge projected anteriorly. This creates a dark inward-facing canopy under an upper light, unlike the broad lit upper plane in the reference. Change the structural prompt: attach the upper boundary near the lid; extend the lower free edge anteriorly and gently downward. Preserve genuine outward front normals and a matched thin return, never fake a lighting normal. Keep the separate upper swept tuft and lateral connection. All protected eye and face geometry stays unchanged. Compare full 5-degree capture again.


## Revision206: coherent evaluated attachment
205 fixes the main geometric normal and produces a broad lit white front surface, but the 90-degree view separates the tuft and main sheet. Current obstacle fitting can independently move a main station by 7 mm while the adjacent tuft moves under 0.5 mm. Before residual collision fitting, measure correction on the actual evaluated upper attachment rail, propagate it coherently to the main sheet and its fork/tuft/wing and fade it down the connected lateral strip. Preserve per-facet collision guards and the 0.8 mm join limit. This is a structural attachment correction, not an arbitrary global offset or fake normals.


## Revision207: tuft ahead of the actual canopy
The coherent root correction alone does not solve the side silhouette. After reversing the main sheet in205, the tuft apex still referenced the attached UPPER rail; the main free LOWER rail now projects farther forward and can hide the tuft. Rebase only the tuft apex against the actual main free-edge envelope, then add a smaller bounded 2.5 mm curl. Keep the root interpolation and original front outline, main geometric normals and all collision/connection gates. Inspect side and front before continuing full sweep review.


## Candidate 208: compact attached canopy after the complete 207 sweep
The 37 angles and three modes in observations207.json show that the current free canopy is too far forward, too steep, and remains visible behind the head long after the reference. Preserve the bright upward-facing top surface, fork count and accepted eye aperture. Reduce the free-edge depth opening and side-wing advance; keep the evaluated root attached. A root clearance correction must taper across the free canopy rather than translating already-clear free tips by the full root correction. Keep the existing per-facet collision verification, thin return thickness and connection guard. No obstacle or check is disabled. Verify 0–180 degrees again, including white and wire views, before any aesthetic completion claim.


## Candidate 210: independent lash trajectory
The 209 narrowed band made the small tuft appear detached in front without fixing the oblique profile; reject that experiment. Return to the continuous 208 band. The eyelid corner slope currently also tilts the entire graphic lash strip, making its outer root sit too high in side view and the inner fork too low. Give the graphic accents their own gentler vertical trajectory (25% of the anatomical eye-corner tilt), without moving the protected eye aperture. Raise the short inner fork into the main lower margin and shorten its nose-side reach. Preserve thin folded surfaces, evaluated collision tests and the attached lateral join.


## Candidate 211: readable fold and swept-tip curvature
The 210 side check is materially flatter than 208, but the large bright main plane still lacks the reference's separate top and front planes. Increase the thin ribbon's fold relief without increasing shell thickness. The swept tip is almost edge-on in profile; add bounded forward curvature only midway along its free portion, preserving root and apex positions. Keep the independent graphic trajectory and all frozen anatomical geometry unchanged.


## Candidate 212: reattach the outer secondary wing
The 211 side checks still show too little layered separation. Native clearance evidence also shows the secondary outer wing remains about7mm away from generated obstacles although only0.35mm clearance is required. Its support currently clamps to the eyelid corner even when the wing extends outside the eyelid. For that secondary wing only, blend the root depth toward the actual local skin surface, capped at8mm, while retaining native evaluated collision guards. The main band and accepted eye aperture remain unchanged. Tighten the side companion separation to avoid a large open loop and give the thin secondary strip a slightly broader silhouette.


## Candidate213 from the complete212 angular review
Use a narrower swept upper tip, rooted on the main ribbon's front fold rather than retreating to the skin-level upper rail as its height rises. Carry its base depth from that front fold continuously through the free portion. The outer secondary wing uses its own local root height and free-margin range; it must not inherit the main canopy's entire height difference and exaggerated forward pitch. Lower its outer tip to make a flatter lower fork. Preserve source anatomy and all collision/connection safety tests.


## Candidate214: connected secondary-wing root and physical tuft fold
213 exposes a new gap between the main strip and the secondary outer wing in the90-degree white/wire view. Keep the wing's close-to-skin outer support, but blend its inner root exactly from the main strip's three-dimensional point, tapering continuously toward its independent outer support. Add an explicit native surface-distance check for that root. The narrow upper tuft needs an actual folded cross-section: increase its fold relief without thickening the return, and remove the duplicate parent-ridge depth term that was applied twice to its base. Reject213 as a final candidate.


## Candidate215: controlled lateral depth trajectory
214 closes the secondary-wing root (native nearest distance about0.071mm) and exposes a real upper-tip fold. The lateral strip still makes a sharp side-view bracket, unlike the reference's gentle continuous line. Give the side band a monotonic top-to-bottom depth trajectory with only a small convex bow, preserving the exact shared top root and the same front-view x/height path. Native obstacle sampling may move it forward when necessary; never allow intersections. Restore a more readable inner fork length now that the independent lash trajectory is flatter.


## Candidate216: retain eye-following curvature while damping the side kink
215 straightened the side band but moved its middle too far toward the front of the eye, exposing a large side gap. Retain65% of the original skin-following depth and35% of the monotonic depth trajectory. This damps the acute bracket without flattening away the required curvature. The exact top join and bottom position remain invariant. Keep the213/214 secondary root and upper physical fold corrections. Recheck all angles and native aggregate validation before presenting for owner review.


## Candidate 240: 可见睫毛色带修复
239完整111张逐角审核仍未达美感验收。主条与短睫毛需从内侧墨黑平滑过渡到外侧低饱和紫色，以使两层的分叉可辨。239顶点色被单色色块烘焙覆盖；本轮将原创色带接入独立UV条带(.86–.89)，避开眼、耳、皮肤既有UV。保持239所有几何不变，用实际渲染核实色带生效后继续精修小叶空间曲率。不复制参考贴图。


## Candidate 241: 短睫毛弯叶走向
基于239全111图及240正侧白模，短睫毛应从宽根逐渐向眼外侧上扬，前半段低缓、末端上翘；避免正面直立针状和侧面厚直三角。保持根部与主条连续、受控薄片回折和左右对称，仅调整原创轮廓控制点及自由端前伸、弧腹前伸。240原创渐变色带沿用。眼型、瞳孔、独立眉毛和其余脸部冻结。通过Beaver生成后重新比较正面/45°/侧面与线框，不以纯测试通过代替美感验收。


## Candidate 242: 只读参考粗测后的短叶位置校准
241正侧白模显示叶尖仍偏高偏内，根部向内展开过多。只读归一化包围盒为候选x .2161–.2768、高-.0291–.0091、前深.1315–.1807；参考为x .2327–.2857、高-.0345–.0042、前深.1337–.1875。将这一差异解释成语义：根部回到原位且稍降，叶尖略向外/向前但略降低，宽根到弯尖连续。独立设计8点轮廓，未复制任何参考顶点/UV/法线或贴图。旧美学测试将叶尖限制在t<.5且仰角比>.25，和本轮实际参考不符，改为t<.65、仰角比>.15、尖高于根2mm；这些是造型范围，不是穿模/连接安全阈值。所有碰撞净距.35mm、连接.8mm、位移12mm以及冻结区域回归保持不变。


## Candidate 243: 外眼缘侧条跟随曲面
保持242上睫毛和短叶，单独修正侧睫毛的空间贴合：减少顶部与底部深度直线桥接的权重，让侧条更多沿各自宽度位置的眼缘曲面弯曲。上端仍与主条三维连接，下端收拢，薄片厚度、分叉轮廓、穿模净距和连接阈值不变。重点对比90–120°可见侧条和面部之间的狭缝，不能只凭正面通过。


244：基于243完整111图对比，短叶应以宽根渐收的上扬弧线露出主睫毛，避免正面仅一根尖刺、斜面过早形成侧向尖角。保持尖端位置和下边根部，增加上缘中段曲率与露出宽度。此轮仅修改原创短叶上缘控制点；不改眼型、独立眉毛或其他脸部。


## Candidate 245：主睫毛下缘与内侧分叉的弧线
244完整111图显示：主条上缘大形已接近，但下缘过平，内端短分叉几乎水平；参考内侧低落而中段收高，形成有张力的弯叶。先单独校准这一原因，不同时改小叶深度。保持主条贴附上根、外侧连接和已认可眼型，内半下缘自然下落，中段轻收高，再平滑接到外端；内侧第二尖沿同一方向下斜，避免两根平行横刺。保持244短叶、侧带、材质与所有安全阈值。新控制点是原创语义设计，非参考网格拷贝。每5°三模式截图须重新验证，不将测试通过视为美感通过。


## Candidate 246：连续主带与短分叉的真实附着面
245正面/45°/90°三模式实际查看后，内侧下降方向正确但弧肩过重，先回收40%的主条下缘改变量，保留内侧下斜尖。短叶的根不能永远锚定主条最后方上边缘：那会把根藏在后面、只留一个侧倒尖。语义为“小叶宽根落在主条靠根的折面上，宽根先顺主条包覆，向外逐渐收窄，再自由上翘”。因此根位于主条从上根向自由边20%的折面；前伸尖略增加1.2mm，同时减少中腹前凸1mm，避免整片向内倒。根部上沿有弯起的宽肩，不是细直三角。沿用碰撞和连接检查，冻结已认可眼型、虹膜、独立眉毛。


## Candidate 247：减少主睫毛折面的台阶感
246正面、45°和90°的渲染/白模/线框9张实际比较：宽根与内端弧线略改善，主条的灰色前折面仍像规则厚条。主条几何厚度保持0.18mm，仅将横截面拱脊幅度由3.6mm减为1.8mm，让表面呈薄弯片而非屋脊状台阶；主根位置、平面轮廓、副翼连接及短叶设计保持。此为隔离变量试验，须用真实白模/线框验证后才保留。安全净距和冻结区域不变。


251修复：短叶必须从同横向位置的主睫毛上缘开始舒展，重叠区域保持贴合；不能用整片最低高度计算抬升，避免根部整体前翘形成独立长刺。仅超过当地上缘的自由叶片逐步前伸和扭转，尖端和正面轮廓保持。250九图仍见侧面长刺及规则主带，未获审美通过。


252结构校正：251把所有处于主带高度范围的短叶都固定，侧面反而成为更细长针状，不采用。短叶是一片从靠内宽根向外上扬的连续叶片；沿叶片长度逐步舒展，不沿全局高度或主带高度阈值突然折起。保留尖端和原正面轮廓，根部贴主带，长向连续前弯，宽向轻扭转。147测试中的两个251重叠区假设不再作为最终结构约束；保留其历史并新增真正根部及长向连续性检查。


253短叶截面校正：252纯叶长进度使下缘也前伸成宽立板。保持近端根和尖端，长度推进与高度推进各半，且高度推进受长度根部权重约束。目标是下缘更收、上缘舒展的曲面薄叶，而非针或立板。新增原生短叶根到主带最近表面距离<0.8mm检查，不能仅靠皮肤无碰撞推断连接。


254窄弯叶校正：253全111图已实际复核。正面短叶宽肩，45°向内形成宽折片，90°近竖直三角板；参考短叶是窄而弯曲上扬的片。先固定根点、尖端、上边界和现有深度场，仅把自由下缘抬向上缘，逐渐收窄中部腹面，避免在侧转时暴露宽帆。根部连接与无穿模阈值不变。这是独立原创参数，不复制参考网格。暂不反转扭转方向，先隔离轮廓面积的影响。


255短叶上扬曲率：254缩窄轮廓后仍在侧面形成直楔。参考白模的弧度来自低根向尖端逐渐增加上扬斜率，而不是根部立即抬高并直线接尖。保持根点与尖端，重塑中间上下边界为低腹、末端上挑的窄叶；长向深度公式与宽向扭转保持，单独验证曲率。原有碰撞及连接门槛不变。


255原生失败修复：首轮被正面法线检查阻止，未导出合格模型。纯几何重现表明弯曲薄片正反面四边形选了不同物理对角线，导致薄片体积符号与前表面朝向矛盾。明确三角化并使背面4/5区对角线与正面匹配，保持薄片层间距、碰撞与法线检查；新增真实短叶左右两侧的正面朝向回归测试。不采用翻转守卫或放宽阈值。


256宽向扭转方向诊断：255实际0/45/90三模式显示单纯缩窄和降低叶腹会让正面分叉消失，不能修正侧面宽立片。保留255已验证的正反面配对三角化修复，撤回短叶轮廓到253完整审查基准；仅反转同幅度3mm宽向扭转，让靠外边退后、靠内边前伸，以旋转薄片朝向而非继续缩小正面轮廓。根点和尖端不动，仍需原生净距、正面法线和多视图验证。纯测试使用扭转幅度绝对值，保留原阈值和解析几何约束，允许有意义的双向参数。

257: Root-attached continuous swept leaf; reject reverse curl induced by local parent ridge. Keep clearance, mirrored winding, thin return and all frozen surfaces.


264短叶高度根因校正：263完整111图确认短叶在35–70度过早变成细线。只读包围盒显示候选短叶下界比参考高约0.019H，尖端、横宽和深度已接近；不能继续仅改深度或颜色。语义要求小叶从主片中段的贴合根生出，腹面向下覆盖主片一部分，再沿外上方自然弯尖；叶尖保持。原创八点轮廓降低根和腹面下界，所有厚度、净距、连接和冻结部件约束不变。实际生成后重点复查根部连接、白模侧面和全角图，不以测试代替审美。


Candidate294: restore289 smooth lower edge and short leaf. Shorten only the upper main strip outer endpoint relative to the independent lower wing, maintaining taper and continuous support clamping. Front/45 screenshots show insufficient separation between the two outer tips; this reduces the stacked blunt-step appearance without changing the accepted eye opening. Revalidate dependent wing joins and collision safety; no aesthetic acceptance implied.

314: Reject309–313 local-depth wrapping experiments: native face-shell collision guards failed, and neither extra length nor width subdivisions provided a valid candidate. Return to308 shared-section support and original topology. Isolate the visible side-band excess width: reduce only inward width smoothly by50% at mid-span, keep the original top-root and zero-width bottom, outer teeth and depth envelope. Review actual front/side/back before retaining; do not treat failed branch tests as completion.

316:315 tooth shortening removes the distinct pointed accents at45/100, so reject it and keep314 original tooth reaches. The broad outward side silhouette also retained307 outward middle guide despite that earlier shape failing. Restore the pre307 center curve(.16→1.005,.38→.985,.62→.9,.82→.75), while retaining308 robust shared-section support and314 narrower inner width. Roots, depth envelope, all upper accents and accepted face/eyes remain unchanged. Re-evaluate actual oblique silhouette before retaining.


Candidate 318: test the missing middle lower-sheet coverage detected by native coarse normalized probes. Lower only the two middle main lower-rail control stations by 0.007H and 0.003H, preserving upper attachment, endpoints and frozen face/eyes. Match outer leaf root to revised parent curve. Reject317 roll-only change. Validate clearances and actual multiangle images before acceptance.


Candidate319: replace point-fitted wavy middle lower rail with a continuous rising inner-to-outer lower boundary after the inner tip. Keep center coverage while reducing neighboring over-coverage. Upper root rail, tip positions, short-leaf outline, lateral accents and accepted face/eyes are unchanged. Parent wing root follows the new smooth boundary. Native safety and visual review required.


Candidate 320: after the complete 319 sweep and candidate-only ray attribution, reduce the hanging inner-third face of the main upper lash. Raise only the lower boundary at inner stations; keep the upper attachment, mid/outer rails, fold thickness, independent fork/tuft/wing, eye and face unchanged. This is a targeted silhouette hypothesis, not an accepted fix. Check front and 45 degrees for loss of fullness and 90/100 degrees for removal of the lower shelf. Revert if either worsens.


## Candidate 328: continuous outer-wing lower silhouette
Return to the provisionally useful 320 baseline. Short-leaf experiments 321–327 have not earned visual acceptance and are not compounded here. The 0/45-degree views show a rectangular hanging step under the outer secondary wing. Its lower guide drops about 3 mm immediately after its attached root. Replace only the four intermediate lower wing heights with a shallow smooth taper: preserve root, upper outline, spatial length, depth guide and pointed tip. The goal is an upturned secondary leaf with no rectangular under-step. Preserve accepted face and all physical clearance/connection checks; inspect the real outcome before acceptance.


## Iteration342: bounded geometric smoothing of the main crease
Return to328;337 margin shortening was insufficient. New native read-only diagnostics show reference and candidate central lash geometric plane tilts are similar (roughly55–60degrees), but reference shading normals in a central patch differ from its geometric normals by23–55degrees, while ours mostly differ by0–6degrees. This discrepancy contributes to the different white-model shading. The reference is unpacked and its extreme normals may be erroneous, so do not reproduce or transfer them. Instead derive a bounded, original normal blend from our own two front fold surfaces:45% toward their shared area-weighted normal only along the main-lash crest. Keep55% of the crease split. Exclude thin return/back faces, all other accents, face and eyes. Mesh positions/topology and collision checks remain unchanged. Compare white/render from all relevant angles to decide whether the hard plank impression is reduced. This is a shading trial, not a structural or aesthetic acceptance.


342 isolated normal diagnostic: Use ordinary own area-weighted shared vertex normals across the two main front folded faces (blend1.0 instead of0.45). Reference aggregate shows crest normal substantially smoother than polygon inclination; do not transfer any reference normals. Main front only, never returns/sidewalls, topology/positions unchanged versus328/338. The blend parameter test now permits its mathematical endpoint1.0, a deliberate design change; physical collision/thickness/manifold guards are untouched. Must compare white, wireframe and NPR render; smoothing cannot claim silhouette repair.


342: Reject uniform341 smoothing. Use semantic center-rounded / tips-crisp normal profile on the own main front faces only. Smooth transition from inner crisp segment through central round crest to outer crisp taper; no reference normal vectors copied. Geometry remains exactly328. Independent normal diagnostic only: cannot claim correction of outstanding leaf silhouette.


Candidate345: inner fork planar outline and fold uniformly scaled0.72 about fixed attached root, retaining sweep angle, with bounded3mm lower-edge return. Coarse diagnostic shows own nose-side supporting skin about4mm ahead of reference atx/H0.10, whereas fork atx/H0.14 already near depth target. Previous pure lateral contraction made a vertical hook; uniform outline contraction tests removing that collision without modifying protected face or eye. Short leaf restored342, axis-twist343/344 rejected. Validate physical clearance and0/45/90silhouette.


Candidate346: keep uniform root-centered inner fork angle, use0.78scale and1mm return instead of3450.72/3mm. Own face-only read-only rays found0.78tip remains in eye aperture while0.82tip contacts forward skin; full native clearance still required.345 removed hanging hook but concealed branch too much. Preserve all other342 geometry and normal profile.


Candidate350 structural correction: main ribbon crown must be measured from its FINAL attached upper/lower rails. Own editable349 diagnostics measured a 4–6mm crest protrusion above near-coplanar boundary rails despite nominal2.2mm relief. Free-margin projection changes the boundaries without equivalently updating the crown. Reconstruct the crown at controlled1.5mm profile relative to final rails; preserve matched return thickness, keep upper/lower boundary placement unchanged and transport the attached small tuft root with a fade to its unchanged tip. Retain all collision, thin-root connection, closed-volume and protected-face checks. This is a hypothesis to validate visually, not acceptance.


Candidate351: Preserve the350 final-rail crown fix. Remove the overly low diamond-like underside of the short upper split leaf by lifting its lowest base station to meet the adjacent lower guide. Keep its root, tip, upward sweep, depth guides and paired return thickness unchanged. The small leaf should read as a swept triangular split on the main lash, not a hanging diamond at30–55degrees. Retain collision and root-join guards.


Candidate354: Reject352/353 lower-margin projection reduction, which moved the lateral root forward and did not establish the desired layered silhouette. Return to351 surface-fitting behavior. Re-evaluate349 central22percent narrowing with350 final-rail crown reconstruction retained;349 alone was obscured by the inflated post-fit crown. Use the same tested central lower guide and matching outer-root station from349, unchanged outer wing rest, unchanged upper guide, unchanged thickness and root/collision gates. Compare white and rendered side silhouette before accepting; do not claim the earlier failed349 trial became useful without this new evidence.


## 第389版：减轻侧睫毛的纵深凹弯
保持第387版连续截面方向和已确认的眼型。本轮只把外侧睫毛根到末端的纵深弧度降至自身支撑拟合弧度的35%，保留根与尖端的位置，以及主片与伴随小片相同的位移场。目标是侧面形成柔顺的贴肤带状结构，减少中段向后陷入再折回的S形视觉。不得改脸、眼球、耳朵、嘴，不放宽穿模和连接阈值。必须以原生生成和多角度白模、线框、渲染验证。


## 第390版：主睫毛横截面由折面细化为弧面
389白模仍呈两片平面组成的屋脊状宽板。对自身已贴合的主睫毛截面，仅在下边缘到脊线、脊线到上边缘之间各插入一个弧面采样点；保留两条边缘和原脊线的位置，用连续抛物线斜率连接。前后回折层严格配对，保留原厚度和分叉连接；执行现有碰撞、封闭性和退化检查。其余五官保持不变。


## 第391版：消除主睫毛外段下边缘凹折
390多角度白模显示主睫毛与外翼连接处仍下凹再急升。自身轮廓参数在t=0.65的下边缘比t=0.55更低，形成局部低谷。本轮仅将该处下边缘从0.0205618927H升至0.0265H，使其位于相邻0.023H和0.034H之间；外翼起点同步采用同一高度，维持连接。其余弧面、尖端、五官和碰撞阈值不变，重新生成并全角度验证。

纯测试发现只升外翼起点会引入新台阶，因此将整个外翼同向平移相同高度（约1.40mm），保留外翼本身的宽度、渐尖和相对上扬；侧睫毛继续由原有连接拟合随外翼根部调整。历史“外段必须比中段低0.004H”和旧根高度常量是被本轮轮廓目标替代的设计预期，改为外段单调上升及共享根一致性断言。穿模、连接、退化、闭合阈值一律未修改。

整翼上移被分叉间隙测试拦截，未执行。最终方案以实际三维高度而非局部rise值为准：共享根升至0.0227H（约0.50mm），外翼前两个下边缘仅调整为0.019H/0.0195H，外翼上边缘和尖端不动。新增断言直接检查实际三维下沿在t=.55/.65/.86连续上升，保持原分叉间隙和根部台阶阈值。
