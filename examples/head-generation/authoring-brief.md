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
