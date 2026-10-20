# 调研驱动眉形几何 v1（开发中）

2026-10-20 的图像研究复核 315 个候选，保留 302 个可用视觉样本。样本含第三方参考渲染且存在相邻分类，不能视为行业标准或官方资产核验。本实现原创构造二维轮廓并生成薄实体眉毛，不复制原始模型、图片或纹理。

## 本轮可观察完成条件

- 六部位选择中的眉部选项实际改变网格，而非仅改变标签。
- 6 个常用入口、4 个特殊构形及圆豆子型对应 11 个新预设；4 个历史预设保留，共 15 个选择。新运行时含 9 个几何分支，这是实现目录，不是九类互斥研究结论。
- 每侧仍只有一个 Tapered brow 对象，离散符号作为其中多个闭合网格岛，保留 NormalRegion 和眨眼部件顺序。
- 长度、宽度、弧高、峰位、端点圆钝、细节数量、卷旋圈数等必须有界并固化到组合摘要。改变眉形不能替换眼型、鼻、嘴、耳、脸。
- 旧默认 source adaptation 为精确恒等。不得覆写保留795/796或已有新模型目录。
- 实际 Beaver 生成、SDF、MiDot 图像检查及源参数回执绑定完成后，才登记相应原生样例；未测样例不继承其他结果。

## 设计和来源

常用入口为细长近直、宽楔、细浅弧、中宽弧、折峰、短叶；圆豆另外保留。特殊方向为卷旋、连续多尖簇、细毛缘、离散符号组。宽度与曲率是连续参数，不强制每个参考图归一个硬类。

- [Hawkins 官方模型展示](https://oppw4-20.bn-ent.net/character/hawkins/)支持离散符号组的视觉方向，不证明原模型拓扑。
- [Sanji 官方模型展示](https://oppw4-20.bn-ent.net/character/sanji/)支持卷旋方向。
- [龟仙人等官方游戏画面](https://dbsz.bn-ent.net/character/)用于粗毛缘等差异观察。
- [原神照片模式图像资料](https://genshin-impact.fandom.com/wiki/Photo_Mode/Gallery)提供短片、细带及多尖外轮廓案例；这是第三方整理来源。
- [参考档案 FAQ](https://sites.google.com/view/referencearchive/faq)不能独自证明所有参考模型都来自游戏发行商。

只保存来源链接和研究结论，不把参考图片或第三方网格加入本仓库。当前仅复用现有眉色/柔和度材质，描边和内部分色不在本轮几何交付范围。

## 代码与参数

- brow_design.py：严格版本与有界输入校验。
- brow_shapes.py：本地米制轮廓、回卷带、毛缘与离散三角。
- brow_mesh.py：真实 Blender 三角化、薄实体、镜像绕序、闭合边、正体积和贴合采样。
- brow_source.py：只替换 build_eyes 中的眉部尾段；不改保留源文件。
- presets.json：11 份研究预设。face_components 的 --brow-controls 可传入 JSON 文件覆盖有界数值，但不能借此修改 kind、schema 或其他部位。

曲面贴合先检查帽面顶点、边中点和重心相对实际皮肤深度函数的间隔；必要时共享边细分，最多3轮。不把名义顶点偏移冒称连续全局最小间隙。仍需实际渲染判断穿插与观感。

NativeJob 仍限64个输出和1MiB代码。新运行时通过请求内模块载入，源码冻结在既有 semantic_bundle.py，不增加输出槽，也不把大段代码藏进保留检查点规避源文件限制。

## 验证状态

实现、针对性测试及原生验证进行中。首次862暴露 Blender 三角化返回索引而非向量的 API 假设错误，已修正；863为成功生成的原型，但早于贴合采样和完整绑定加强，不作为最终验收。当前原生状态以随后提交的证据文件为准。


阶段检查（2026-10-20 UTC）：864 细直眉原生生成和绑定 SDF 成功，已实际查看正面、左 45 度渲染及左 45 度白模；完整姿态检查仍进行中。865 宽楔眉搭配杏仁眼原生生成成功，其 SDF 与渲染仍待验证。其他新预设保持 native_validation_pending，不继承 864 的结果。

本地通过：brow_library 8 项、face_components 58 项（含 CRLF 源码指纹回归）、facial_surface bundle 8 项；另有既有眉部、源中性和睫毛眨眼专项回归。已将新几何测试加入 CI。当前环境无 cargo 和 prettier，尝试官方有效行数与格式命令未能启动，因此不声称这些门禁通过；未调整或放宽基线。

864 已补齐 84 张眼部和 18 张嘴部源绑定截图，完整链校验与接触图复核通过，见 native-evidence.json。测试没有证明连续全局无碰撞或最终审美。原生链还发现既有 round-open 配方省略 lateral_aperture_follow / lateral_blink_follow，而生成器补入精确默认值 0 / 0.85；验证器现仅允许这两个精确默认值，非默认漂移仍拒绝，新增正反回归。

首轮远端 CI：Head generation example、Development Quality Reports、Dependency Security 均成功；正式行数 0 violations。质量报告仍记录未修改文件 workflows_upgrade_tests.rs 的既有 rustfmt finding，Prettier 没有执行，不能声称零告警。
