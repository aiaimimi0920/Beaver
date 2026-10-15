# 664 已确认模型与 SDF 源码检查点

2026-10-15，用户确认 664 当前效果可以，并明确要求将代码提交到 Beaver 和 NPRCharacterFrame。这里保存最新源码、配置、回归测试和生成流程，不含银狼参考模型、第三方贴图、凭据、运行数据库、导入缓存或私有截图。

## 复用顺序
1. 已有授权的 Aster Beaver 项目需保留私有基线 head_recovery_49 的模型/纹理/定义，以及 head_recovery_617 的 runtime blend 和 generation_report.json。它们是几何冻结输入，不是公开源码自带的资产。请从自己的恢复包恢复；缺失时不得用参考模型替代。
2. 保留 authoring/head_generation_guide.md，包含去发侧脸轮廓检查要求。
3. `python bundle.py --output <request.json>` 生成 head_recovery_662 几何请求，在现有 Beaver 的 blender.start 中执行。该命令自身不启动 Blender。
4. 使用 ../../sdf360/bundle.py 生成 664 SDF 请求。首次创建无需 --workspace；覆盖已有输出时提供 --workspace 以读取当前 SHA256。通过 Beaver 原生执行并轮询终态。
5. 安装 head-review 示例中的完整 showcase_extensions 和 showcase 目录，经 Beaver 打开 head_recovery_664/aster_definition.tres。参考模型是可选的授权本地资源，未配置时仍能单独检查候选模型。

## 已完成范围
最新几何包含鼻尖与相邻曲线、耳部包裹/连接、眼眶与分层睫毛的迭代。662 为保持几何的底座，663 引入 SDF，664修复细节法线生成孤立阴影的问题。独立小面片和眼眶仍保留可配置的风格参数。

已验证：356 项几何/配方纯测试；664 的9项距离场/方向测试、6个原生输出校验、250张实际光照图、10项明暗方向图像断言，以及左右45°实际观察。±180°首尾图一致，几何身份未变。用户确认本轮效果，不代表完整装扮、所有表情、所有硬件或最终产品发布验收。

## 限制
SDF 为单主光、连续凸形艺术化光照代理；不含通用投射阴影或多灯合成。框架原生验证的 assetDependencyHashesVerified 为 false；6个新输出另行验证 SHA。参考视频的贴图未下载或重新分发。

## 测试
`python -m unittest discover -s . -p 'test_*.py'`

baseline_fixtures.json 保存历史冻结输入的样式值、源文件 SHA 和 AST 合同，保持原有比较强度，避免测试依赖工作区外的历史目录。pipeline.py 等源码仍使用明确版本的原始几何恢复路径，不是任意角色适配器。
