从609基形执行610五次平滑，下弧变形系数0.35，上弧1.0；渐消宽17mm及全部拓扑、限幅、单调验证保持。此为上翘形态与平滑程度的分段控制。Beaver原生生成后同角度测量。

## 613 ear side silhouette
User requests side-slope comparison of ears. Current612 upper posterior ear arc bulges 2–4px too far back while the lower arc withdraws 4–7px too soon in matched left90 view. Keep nose612 and eye/face geometry fixed. Apply a bounded bilateral posterior-depth fairing to the ear membrane only: modestly trim upper bow, fill lower sweep, smoothly fade at shared roots and preserve hollow wrap. Compare rendered/white/wire silhouettes at matching scale and side/45/back angles before acceptance.

614: actual613 improves silhouette RMSE3.26→1.69px but adds a middle-ear plateau at590–600px. Narrow the upper half-support of the lower sweep from27mm to15mm, retaining lower support and root pinning. Verify slope recovery before acceptance.


615 marked correction: user green line identifies the upper-ear junction, not the already calibrated posterior rim. Reference skull edge descends below the ear crown before returning upward; current ear root and crown join almost level. Introduce a shallow local 3.5mm downward transition at the shared upper root, taper through neighboring skin and membrane, preserve the separate outer crown and no geometric opening. Both ears match. Recompute shared-root reports; verify matched side closeups and no new intersections.

Native safety rejection: initial13mm lateral/depth support folds the narrow upper bridge. Widen lateral/depth support to26mm for a shared smooth transition; maintain3.5mm center and18mm height support, verify actual crown retention.

616: actual615 lowered the crown5px along with the root, failing the target notch depth (3px vs11px). Keep26mm lateral support but narrow depth support to17mm to preserve crown while lowering shared upper root. Retain face orientation and intersection checks.

617: actual616 notch5px remains too shallow against11px reference; crown still3px lower. Separate root and crown controls: root depression4.8mm and local posterior crown restoration2.2mm centered9mm behind root; crown restoration fades fully at shared root. Preserve connected topology, verify normals and all new intersections before visual acceptance.


618：只修主上睫毛的内低外升下缘与浅折面，避免中央下垂宽带。减少中段上缘隆起、截面鼓包，保留两端尖叶与外翼共享根。617耳鼻、独立眉毛、眼球不动。以逐5度渲染/白模/线框及曲线斜率测量决定是否保留；后续侧弧带仍待修。

619：外侧弧带继续沿眼缘向下、向内延伸，终点语义站由0.70移至0.56；上下根保持连续，伴随小层间隙由0.003H减至0.0012H，仍保留薄层和独立尖齿。原生碰撞及接缝检查不得放宽。

620：上带内中段下缘取消局部隆起，保持缓升弧；侧带中段截面少量转向正面，使连续弧带及尖齿可读，根部两端方向保持。偏置0.35、按sin²沿纵向渐入渐出，伴随小层一起变换；碰撞规则不变。

原生复核发现既有耳根预条件错误依赖“两边同时出现初始反转”。现在由实际受限面找到唯一镜像拓扑桥面，一并保持一致约束；仍要求两组、唯一镜像、全部最终面不反转、作用域不越界。不得把本修复用于改变已接受耳型，需与617实际坐标比对。

622：仅再生成眼缘睫毛。在最终组合阶段显式锁定用户已接受的自有617非眼区域，使用自有可编辑检查点作近邻对应（超过1mm即失败），保持新眼部顶点不变；再计算实际法线和检查连接。不使用参考角色顶点，不把自动再生成的耳部漂移交付给用户。此版本恢复依赖需包含自有617runtime blend及报告。

623：外侧细层沿主带边线连续贴合，横向分离从0.0012H降为0，仍保留自身薄片与法向微小层次、末端分叉和两个尖齿。以白模/侧面验证，避免正面可见的细长亮缝；实际穿插和分层不接受仅凭正面判断。

625：624前伸减少使90度小叶尖端后退过多，弃用，保留623前伸。侧边细层不能只调初始间距，必须在主带完成贴合后沿实际主带边界重新定位：内缘重叠0.08mm、向前微起0.2mm，保持高度、截面及尖齿，随后再走完整避障检查。禁止仅以一个角度接受。

用户“子睫毛独立并嵌入小尖叶”映射为 side_leaf 模块的两个独立薄片子结构，依附最终侧带基准线，不再用主侧带横截面宽度突起表达。渲染、白模、线框分别核验：连续主曲线、独立体积、实际根部连接。

652：651正面开口缩小但斜角仍长缝。上翼重叠边跟随主睫毛下缘的深度，根/自由端渐隐保留，底边不动。仍为独立薄片层叠，非整块封孔；通过真实0–180度三模式检查。

653：保留652上外分叉修正。对照用户线框，侧边独立子叶仅用根/肩/尖/肩/根五截面构成，避免密集微折。尖端沿伸出距离向下轻倾，折面高度0.45mm，根部仍嵌入主带。只改这四片子叶，不调整主侧带、眼型、耳鼻。多角度验证后判断，不凭网格数量验收。

654：653白模子叶过平且竖向拖长，未达交付标准。保留五语义截面，根部跨度收至70%，折高1mm，向后倾角系数降至0.15、向下倾0.15，目标是短而清楚的独立折片。原根距/净距约束不变。

655：654正面短子叶更清楚，但90度投影太薄。保持所有轮廓和根部，子叶自由伸出方向深度系数0.15→0.7，增加侧视截面。由原生贴合检查保障净距，不放宽根/表面约束；须对照90度白模。
