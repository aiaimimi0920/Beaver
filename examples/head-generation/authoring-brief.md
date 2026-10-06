# 参数化风格脸生成规范：校准案例 A

创建柔和、下颏收窄的二次元脸。只重建 Face，既有 Body、Hair 不变。以下比例均相对于脸高 H；横向 x 对称，y 从颏尖向额顶为 0–1，z 向前，颏尖前表面为深度 0。

眼部：两眼中心横向 ±0.222H。开口横向半宽 0.110H，内外眼角高度分别 0.428H、0.452H。上弧最高约 0.489H，下弧最低约 0.344H。内眼角深度 -0.006H，外眼角 -0.071H。上缘向开口内覆盖约 0.028H，下缘仅约 0.003H。睑缘后方是真实空间，后腔比前开口宽约 1.22 倍、高约 1.43 倍，后退约 0.08H。不可形成向后收窄的漏斗或正面突出的白色眼球。

独立虹膜：中心 x=±0.218H、y=0.423H，横向半径 0.065H、纵向半径 0.082H。边缘中间深度 -0.029H，外侧逐渐后退；中央相对边缘曲面内凹 0.020H。虹膜保留独立几何，隐藏边界由眼睑遮挡。配色为灰紫，暗上缘、明亮下部和克制高光。

鼻与嘴：小而明确的楔形鼻尖，鼻尖高约 0.29H、前突约 0.157H；通过连续鼻梁和侧面过渡连接面部，不能成为贴上去的球。嘴中心约 0.157H，宽约 0.165H，静止时仅留小于 0.004H 的细缝，唇面深度约 0.08–0.10H。口腔向内扩展，内壁朝向口内，舌体有厚度。张嘴通过形变测试，默认值为 0。

轮廓与拓扑：额顶圆弧、颊部饱满、颏部短圆，额头中轴最前处约 0.107H。使用独立配置的少量宽度和剖面控制站，而非固定角色顶点。可见脸面采用有意义的四边面带；下颏不能所有线聚到一点；不增加无结构作用的极近重复线。运行时允许明确三角化，保留切线和口型形变。

验收：固定同屏比例下检查正面、45 度、侧面，白模、线框及灯光方向；核对眼眶前后关系、鼻唇中轴、闭口与张口。完成生成与技术检查不等于外观合格。该案例只验证模板能表达这一组参数，不能直接宣称适用于所有脸型。

## 第 2 次校准修订
眼眶深度约束只作用于开口附近的窄过渡带，不能压平整条眉骨或颧骨。横向剖面分成中央柔和转折、宽阔面颊和外缘回转三个部分，禁止用一个大范围平面把它们覆盖。耳朵以固定的归一化耳根平面构建，不能让每一条耳环跟随脸缘深度各自漂移形成侧面月牙。近闭合嘴缝应微弯，唇缘有克制的上下体积。

## 表面法线一致性修订
白模审查不能被与真实拓扑不一致的分析曲面法线掩盖。使用实际面片面积加权的平滑法线；对连续皮肤焊接后的顶点重算，眼睑、虹膜、口腔等有意义的边界维持独立。不得以修改灯光或诊断材质来隐藏缺陷。

## 曲面与布线修订
保留可编辑的低密度四边面控制笼，用一级四边面细分形成连续皮肤曲面。眼口开口边界作为结构边界保持，脸外缘允许自然圆滑；不得添加无作用的极近平行支撑线。评估细分前后眼口比例、鼻尖轮廓、下巴平滑性与张口形变。实际面片法线与细分后的网格保持一致。此项旨在消除采样不足的块状转折，不以平滑法线伪装错误轮廓。

## 稀疏语义剖面校准
控制脸的横截面时，采用下颏、下唇、鼻底、颧颊、额头、额顶等少数语义高度的曲线控制站，并以连续插值连接。每条横截面只包含中央、内颊、中颊、外颊及回转边界，不读取外部模型顶点。角色参数显式保存在 calibration.cross_sections 中。减少下颌两侧宽度并保留小圆颏；眉毛高度调整到约 0.58H。嘴角不能形成尖锐折痕，唇面体积需在白模中可读。

## 剖面连接与参数清理
各高度剖面按当前脸宽归一化后放样，禁止把较窄截面的越界值截成一条平带再向上混合。保留所有实际生效的参数，移除已被新结构替代、运行时不再读取的旧参数，避免自然语言映射到无效旋钮。

## 耳、颏与唇的联合校准
耳部上缘略向外、耳垂略向内，耳面朝前外侧而不是正面竖直贴片；耳根保持和脸颊连接。下颏最底部应有向后上方回转的圆弧厚度，避免侧面像刀尖。闭口缝保持不变，在其下方约 0.125H 高处形成柔和下唇体积，上唇峰约 0.175H，唇面起伏约 0.012H。眉线应细，厚度约 0.0035H。虹膜降低蓝紫饱和度，使用中性灰紫的浅下缘，不改眼眶深度规范。

## 闭口唇缝和材质边界修订
闭口缝中心约 0.15H；嘴缝所在曲面应比上、下唇峰稍后退，形成可读的唇间过渡，不能只在一条平直剖面上切缝。下唇峰约 0.13H、起伏约 0.006H，上唇峰约 0.175H、起伏约 0.012H；静态缝隙仍小于 0.004H，内部腔体与张嘴动作保留。耳部贴图坐标随耳形参数归一化，采样远离图集边界，避免耳尖串入相邻色块。眉厚折中为约 0.007H。

## 隐藏眼腔净距约束
眼腔隐藏侧壁必须在最终细分皮肤内侧保留约 0.004H 净距，避免极小间隙让轮廓线或侧壁从侧脸显露。开口边界和独立虹膜保持不动；后腔扩张比例是目标尺寸，有冲突时只收回隐藏壁。检查应针对最终曲面，而非仅检查未细分的解析轮廓。记录调整量，并通过原生侧面与45度视角核验，不能只凭数值宣布解决。

## 真正的下颏回转面
颏部不能仅用正面高度场的最低边界代替。前颏接缝高约 0.007H；随后向后 0.03H 的面带下落到最低处，再经向后 0.07H/高0.004H、向后0.14H/高0.015H、向后0.20H/高0.03H的控制站形成圆滑回转。使用连续四边面带，横向逐渐展开，不把多条边收束到单点。正面与下颏共用接缝并一起细分；分别检查正面朝向与下颏向下的绕序，保留闭口和张嘴形变。

## 下颏两侧闭合修订
回转面必须沿两侧连接到下颌边界，不得形成悬空底片或侧面开缝。回转环横向范围保持在对应高度下颌轮廓内侧。用连续四边面侧带连接；只允许每侧起始角一个局部三角闭合面，不形成扇状汇聚，更不把颏尖收束到单点。分别验证前脸、向下回转面与向外侧带的绕序。通过真实几何闭合修复，不改诊断材质或隐藏缺陷。


## Candidate70: asymmetric attached ear specification
The ear must read as a swept-back pinna attached to the facial side, not an independent elliptical dish. Calibrate a twelve-landmark semantic outline: narrow attached lobe below, wider upper helix above and behind, recessed concha, a broken inner antihelix ridge, and an anterior tragus transition. Use a thin closed back shell and meaningful quad bands; do not fan all faces into one vertex. Root-side points must overlap the facial side surface without a visible circular rim. Keep the successful69 chin, eye pocket clearance and mouth geometry unchanged. Reference mesh/UV/texture remain analysis-only and are never loaded by generation. These approximate landmarks are a case preset, not a universal anatomy claim.
Acceptance: neutral front, both sides, white45 and wire against the reference; no black atlas bleed or visible gap, ear not protruding forward, closed positive volume and finite/positive face areas. Recheck mouth0/0.5/1.

Back-shell correction: the back surface must follow every front semantic band at positive thickness. A single nonplanar polygon across the outer ear boundary can intersect the recessed concha and triangulate differently when reversed; it is prohibited. Mirror the front quad layout into the back shell and join only the outer boundary.

## Candidate71: root attachment depth
An ear root must join the posterior side boundary, not extend over the cheek. Sample root depth at the actual outer facial boundary; lateral overlap is a separate burial parameter and must not move the depth sample inward across the sharp cheek-return profile. Keep the closed conforming back shell from70. Verify absence of a lateral wing/visible root outline in exact side view.

## Candidate72: inferior ear-root transition
The ear lobe must transition into the lower lateral head, without a blue/background notch between the lobe and jaw. Extend the root attachment sector smoothly through the inferior lobe, retaining the posterior depth sample and a distinct outer helix. Do not restore the cheek wing. The outermost rear rim stays independent of the buried root sector. Keep all face and eye/mouth geometry unchanged.

## Candidate73: evaluated-surface ear-root attachment
A control-profile overlap alone is insufficient to certify the final subdivided root junction. After evaluating the facial shell, project only the anterior/inferior ear boundary onto its nearest skin surface and bury the seam0.002H inside. Propagate each boundary correction smoothly inward across radial bands and identically to the back shell, retaining ear thickness. Limit correction to0.06H and record actual movement. Do not move the posterior helix or use material changes to hide a gap. Reject nonfinite or degenerate geometry.

## Candidate74: readable brow and illustrative facial accents
Keep the successful recessed eyes, nose/lip geometry, closed chin and fitted pinnae. Bury the ear seam0.006H to clear the fixed renderer outline as well as the skin surface. Brows span x0.106–0.304H, rising from y0.568H to0.591H with gentle arch and tapered thickness up to0.018H; they must not be long horizontal lines. Add a restrained warm upper-lid crease in the procedural skin atlas, without extra parallel mesh rails. Use dark plum lash pigment, gray-violet iris planes, a soft outlined lower iris disc and a small cool accent near the pupil; reduce large white dots. Give the small nose plane a readable but restrained tint. All graphic features are independently authored procedural parameters, not sampled or copied textures.

## Candidate75: pigment calibration independent of structure
Retain structural74 geometry. Keep eyebrows short and gently rising but halve their visible thickness to0.009H and reduce arch to0.002H; avoid an exaggerated high curved brow. Iris lower planes are a lighter gray-violet with dark upper rim. Increase lower eyelid coverage to0.005H while retaining recessed placement. The nose-plane pigment occupies height0.345–0.425H above the geometric tip, with0.032H width; do not confuse pigment placement with nasal projection. Keep subtle tip blush lower. No lighting changes or reference texture copying.

## Candidate81: reconstruction after workspace reset
Restore the useful lip-volume and narrow neutral seam design from recorded experiments, using the verified75 recipe as the source of truth. This is a new reconstruction, not a claim to recover the exact lost76–80 files.
The relaxed mouth must be almost closed with a thin seam, with a real oral cavity and thick tongue behind it. Upper and lower lip rims must have controlled, distinct depth, tapering smoothly together at the corners. Opening must separate the lips across their width rather than creating a central triangular hole. Define upper/lower deformation relative to the same curved neutral seam used by both the skin aperture and the oral cavity. Changes in neutral gap size must not reclassify raised corners as upper lip.
Retain the eyes, ears, nose, chin and all Body/Hair data from75. Do not apply the ineffective boundary-crease experiment or the rejected over-wide lip shading experiment. No new dense support rails or reference mesh/UV/texture copying.
Acceptance: real Beaver generation, preserved Body/Hair fingerprints, finite valid geometry, neutral front/side/white45/wire review and MouthOpen0/0.5/1. The cavity must remain dark, tongue visible when appropriate, and all lip corners remain connected. Overall face quality remains work in progress even if these focused checks pass.

## Current reference-review scope
This candidate addresses only the lip profile and neutral-seam deformation. The full workflow must additionally validate both side views and the rear eye/oral-cavity structures. Upcoming independent revisions cover region-weighted center/perimeter edge crease, shallow non-pinched eye-pocket backs and simplified pinnae with original texture details. Do not report these unchanged75 features as fixed by81. Imported reference data can be defective; copy no mesh or original crease values that are not actually available.

## Candidate82: broad eye-pocket rear closure
The hidden back of each recessed eye pocket must be a broad shallow closure, not a pointed/conical funnel. Retain the independent concave iris, eyelid opening and front clearance. Terminate existing eye-pocket bands at a full-size planar rear boundary; remove the shrinking inner ring that creates a radial pinching pattern. Keep enough depth for the iris without letting the pocket protrude through side facial skin. No extra dense concentric support loops. Preserve81 lip profile and shared rest-seam morph.
Validate front, both sides and rear in white/wire/render modes. Report rear-cap planarity and degeneracy before and after clearance fitting; a topology-only change is not accepted until the native back view loses the old pinched appearance without front/side regressions. This is a focused eye-pocket change, not a claim that ears, nose or facial crease have been fixed.

## Candidate83: simple pinnae and truthful rear-cap shading
Use modest, simple ear silhouette and thickness geometry. Internal ear detail should primarily come from an original procedural ear atlas: a restrained concha tint, curved helix indication and short inner fold. Remove the over-modeled alternating relief rings. Keep root overlap, closed thickness, positive volume, no background notch and no forward cheek wing. Keep a sparse set of useful quad bands rather than a dense radial sculpture.
The planar ocular back caps introduced in82 must use their actual flat geometric corner normals; averaging their boundary normals with the pocket walls creates unwanted vertical bands. Smooth surfaces elsewhere retain area-weighted geometric normals. Do not apply artificial analytic face normals or turn the whole face into flat shading.
Preserve81 mouth dynamics, current nasal and facial shell shape. Native front/side/rear/white/wire inspection and mouth0/.5/1 are required. Nose and facial crease remain later focused changes.

## Candidate84: regional Blender edge Crease for face planes
Use explicit, configurable edge-Crease groups along the facial midline and the front-to-side cheek boundary. The nasal midline may retain a firmer ridge; reduce its strength toward the forehead and chin. The lateral front-face boundary should be readable without becoming a cut, detached seam, or a uniformly hard border. Preserve the meaningful control-cage edge flow and existing aperture boundaries; do not add dense support rails.
This recipe uses actual Blender crease_edge values before Catmull-Clark evaluation. Prove that the weights change evaluated geometry by comparing with the same cage with only the new crease groups disabled. Do not infer original reference weights, split the mirrored face, replace all normals with hard normals, or claim a stored attribute alone produced a visual change.
Keep83 eye rear closures, ear simplification and81 mouth dynamics. Inspect front/side/white45/wire and compare nose/cheek transitions; inspect symmetry and mouth0/.5/1. Nasal projection and oral rear volume remain separately tunable and are not declared solved by crease weights alone.

## Candidate85: distinct nasal base and narrow upper bridge
Use the verified center/perimeter edge-Crease mechanism, but do not expect crease strength alone to fix proportions. The lower nose needs broader side support without a bulbous tip; the upper bridge must fall into the inner cheek/eye region more narrowly. Give the nasal cross-section an explicit monotone peaked profile with a fuller basal variant, rather than a single circular bell. Blend basal fullness out above the tip. Preserve a small slightly projected tip and continuous side silhouette.
Add a sparse semantic upper-bridge cross-section while preserving outer cheek/head boundaries. Keep84 lips, eye pocket closures and simplified ears unchanged. Generator parameters must be semantic and independent of reference meshes/texture pixels. Judge native front, exact sides, white45 and a light sweep as well as sparse measurements; lower numerical error does not establish beauty or acceptance.

## 86：口腔后部体积
静止嘴缝及外部唇形保持85版。口腔从窄唇缝向内扩展为有足够上下空间的腔体，不能把后壁收成扁小的尖锥。内腔顶、底、宽度和向后距离应有独立的归一化控制；后壁应为宽阔、近似平面的封口，不在可见面皮上制造汇聚极点。舌体有厚度，保持既有开口时可见的状态。检查闭口、半开口、全开口及背面；增加内腔不能穿出下巴、鼻底或面颊。保持当前眼、鼻、耳和表面拓扑不变，以隔离这项调整。

## 87：独立虹膜上下比例
保持内嵌眼眶及四周眼睑关系，独立虹膜略向上对齐，并缩短竖向半径，避免虹膜像下垂的圆盘。虹膜仍是内凹曲面，不能通过整体前推形成凸眼球。将虹膜中心高度与竖向半径明确区分，固定眼眶、睫毛、贴图及其他脸部参数，验证正面和双侧的眼白留量、遮挡与视线观感。

## 88：虹膜与下眼睑协同遮挡
虹膜上移和缩短竖向比例必须与下眼睑协同，不能在下缘露出一条不自然的完整白色月牙或双重黑边。保持眼角高度，上提下眼睑弧线中央以轻微遮挡虹膜下缘；平滑地过渡到眼角，保留内嵌眼眶的后腔和侧面层次。单独匹配虹膜包围盒不能替代眼睑、眼白和虹膜的整体关系。

## 89：虹膜的均匀四边面布线
独立虹膜采用疏密适当、均匀分布的四边面网格，避免在椭圆上下端把一整排顶点压到极窄的距离内，也不采用所有边汇聚到中央点的扇形。轮廓仍为同一椭圆，中心、尺寸、内凹深度与眼睑遮挡关系保持88版。减少没有轮廓或形变作用的采样线，并检查投影四边面面积、最短边、曲面轮廓和贴图表现；不能靠修改材质掩盖布线损伤。

## 90：风格化口型与稳定下颏
静止仍为半闭合细缝。张嘴主要由唇缘上下分离及唇周局部组织完成，下颏尖与下颌外轮廓基本固定，禁止整张脸向下拉长。默认全开口较克制：上唇约上移0.013H、下唇约下移0.038H，嘴角只轻微下移0.0064H。下方权重在靠近下巴前快速衰减，底部0.07H范围固定，0.07–0.145H为平滑过渡。保留真实内腔和有厚度舌体，舌体随较小口型适量下移；检查中立、半开、全开以及侧面，不得折叠、穿插或产生三角裂缝。该幅度是本项目风格目标，不是渲染器限制。

## 91：耳根精确对接与浅空心耳壳
耳朵不再是覆盖面部的独立封闭厚块。以脸壳真实侧边界的一段作为耳根，沿其向外生成浅弯曲的椭圆半壳面片，耳根位置逐点一致，合并后边界须连续。保留空心/薄片感，内部耳廓主要由原创贴图表达；不增加多层环状凸凹和无功能后盖。轮廓上部宽圆、下部渐收，向后轻微倾斜，不能形成耳根缺口、明显分离描边或尖锥。验证根部接缝、面方向、非退化面和双侧轮廓。

## 92：完成本批轮廓、眉唇与后腔规范
头顶边界应是连续圆弧，向侧后方平滑下降，不在额顶和侧边形成突兀尖角。形状过渡分布在上半额头较宽范围，避免把转折挤在最后一两排；中线与脸侧必要折痕保留并向顶部渐隐。眉毛为贴面的深色细长弧带，眉头收尖、中段稍厚、外端渐尖，整体轻微上扬；厚度约0.012H、端点高差0.025H，不做悬浮细线。中立嘴缝略窄，半宽约0.065H，中心高度约0.150H，保留轻微嘴角上扬、上唇稍前和半闭合内腔。张口继续固定下巴。眼窝后部采用短过渡接简洁椭圆截面，后两截面等宽等高，去掉不必要的鼓包与收腰；口腔后部也用连续、尺寸一致的简洁椭圆段，前唇附件保持细缝，舌体和遮挡保留。

耳根的第一段面片应顺着脸侧切线向后转，再逐渐放平为浅弯耳壳；避免精确接缝虽焊上、却因切线突变鼓成结块。91白模显示根部仍有鼓起感，92将后扫轮廓改为平滑正弦转向并保留同一根部边。

## 93：头顶空间弧线与耳根跨度复核
92侧面显示仅调整顶部高度还不足以得到圆滑空间弧线：顶部深度不能在降低高度后再次套用较低面部截面，否则后缘会偏前而显得像斜直边。将头顶边界的高度和前后深度分别用少量语义控制点约束，在同一宽过渡区平滑连接额头。根部耳高区间略扩大到0.26–0.55H，以免实际采样截短耳壳。保留前述眉唇、简洁后腔、共享耳根和稳定下巴口型。

## 94：保持额头前轮廓
93侧面复核发现把整段额头深度混向顶部绝对深度会压凹前额。顶部空间弧线应只施加相对于原顶部深度的差值，并向下渐隐，不能把额头中线原有凸弧拉向头顶平面。保持头顶边界目标、原前额凸度及其他本批参数，复核侧面后才保留。


## 95：可读白色眼高光与有界浅眼槽
本轮用户实看94：眼睛缺少清晰白色高光，内槽过深，侧方位有穿插风险。保留独立内嵌虹膜，但“内嵌”不能等于越深越好。明确区分眼睑口沿、可见虹膜曲面、藏在后方的白色腔体。缩短后腔直段并减少虹膜内凹量，保持眼角和眼睑不变；不把整个眼球向外推，也不靠提亮遮掩深槽。
增加一块可参数化的原创白色小椭圆高光面，紧随虹膜曲面并仅留防重叠的小间隙。高光在正常展示比例下必须能看清，不依赖仅存在于贴图中的亚像素小点。高光位置、宽高、表面间距独立可调，面片低密度均匀四边面，不复用参考几何或纹理。参考只用于判断机制与稀疏比例。
验收：真实生成后比较正面、左右45度和左右侧面，渲染/白模/线框及多光照；白点不能被上睑完全遮掉，不能成为浮在眼前的凸片，浅后腔不能穿脸壳或封住眼睛。保留94闭嘴、口腔、固定下巴动态、眉唇与耳根结构，耳形及下颌收束在后续独立迭代处理。


## 96：耳轮廓及下颌至耳根的连续回收
94耳根虽已真实连接，但耳形和下颌至耳根收束不符合用户观察。耳朵改为上部较饱满、下部逐渐变窄的浅椭圆膜片，耳根共享实际面壳边界，不能把宽大的三角翼作为耳廓。横向断面采用四分之一椭圆的外展/后掠变化，减少根部突折，保留开放背面与原创耳纹。宽度和后掠分别用少量归一化控制点，不读取参考网格。
下巴到耳根需要前侧面和后侧回收共同形成连续轮廓。下颌后缘在低处应向内收，再平滑过渡到耳根；不将整段边界简单设成同一高度截面的最宽点。为靠后外侧带增加有界横向收窄与深度回转参数，渐消于上侧脸。前脸、鼻唇、眼睑开口、下巴尖和既有表情小幅运动保持。不能用尖点汇聚或高密度补线掩盖轮廓问题。
检查正面、左右侧、左右斜侧、背面和白模线框，验证耳根仍为共享流形边、局部面不翻转、轮廓连续；没有实际截图与模型检查不能宣称耳形已通过。

## 97：保持下巴底面与回收带的连续性
96试验在生成阶段被下巴底面朝向检查拦下，未形成可交付模型。下颌外侧回收不能直接牵动下巴最低一排及其原有底部闭合。新增高度0.020H至0.060H的平滑启用区间，最低底面完全保持94结构；往上逐渐进入下颌回收。保留朝向检查，不删除断言或翻转法线掩盖拓扑。

## 98：限制外侧收窄，保留正向参数次序
97已避免底面变形，但过强的横向回收使前侧参数带折返，被原有面朝向检查拦下。不能通过放松检查交付。将侧缘收窄限制为该高度半宽的最多10%，保持横向参数严格递增，结合有界后缘深度变化形成柔和收束。此版本不会声称精确复刻参考的全部向后包转；后续是否需要独立侧后拓扑，应由实际侧面效果判断。

## 99：下颌参数不能影响眼外侧与太阳穴
98完成实际生成并通过基础拓扑检查，但白模斜侧出现眼外侧/太阳穴的局部硬折，说明下颌深度控制向上作用过广。将其在高度0.27H至0.36H平滑淡出，眼眶和太阳穴保持已检查过的95形状。耳膜的新外展/后掠轮廓与下颌低处的有界收窄仍保留，不靠改材质掩盖硬折。

## 100：按真实眼眶补片边界隔离下颌控制
作者笼的眼眶补片最低边界在高度(1.614-1.544)/0.236，约0.297H；只在0.36H结束衰减仍会牵动这条边。下颌后缘深度控制改在0.22H至0.285H淡出，保证整个眼眶补片及上侧脸保持95形状。测试使用实际最低补片高度，不能仅挑更高的测试采样点。

## 101：用边界位移修正保留横断面曲率
100的白模回归仍暴露下颌上方压平凹痕。原因是将外侧带每一点直接向“后缘绝对深度”混合，连内侧已有曲率也被压向同一深度。改为计算目标后缘与原始后缘的深度差，再将这一个位移平滑施加到外侧带；保留现有横断面起伏，后缘目标仍可独立调整。添加同一位置不同初始深度间距保持的回归测试，不改材质、不放松朝向门槛。

## 102：耳外缘不继承耳根的波动
101已去除侧脸压平凹痕，但侧面耳廓外缘仍随耳根深度起伏呈S形。保持已焊接的耳根不动，为外耳缘设置少量归一化椭圆后掠深度控制点；横向从实际耳根平滑过渡到这一稳定外缘。这样既保留一体连接，也不把面壳边界的波动机械复制到整片耳膜。原创耳纹、眼部和下颌保持101，检查左右侧和斜侧实际轮廓。
