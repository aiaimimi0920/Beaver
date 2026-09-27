# F8 候选尝试编号存档延后反馈

日期：2026-09-26。状态：已完成候选输出编号存档到发布后任务的子流程；全部 Beaver 开发计划及 F8/F9 仍未完成。

## 可使用的流程

在候选尝试输出的 Godot 场景预览中冻结画面、填写编号区域意见并保存。打开该候选的反馈面板，选择“排入后续中修”，刷新并选择编号存档，回看原图和区域意见，再填写反馈、标题与验收标准并保存。

编号存档与冻结 PNG 文件标注互斥。选择存档后切回“本轮返工”会保留引用，并明确禁止本轮提交；用户需要切回后续中修，或显式移除存档。草稿按完整审阅身份保存，关闭重开不自动提交。请求先写本地存储再发送，响应丢失后重试原请求，不能用新选择覆盖未确认的引用。

发布面板可展开回看原始存档及编号，展开前不读取图片。列表最多读取该尝试最近 8 个编号帧，已绑定引用额外精确读取，不依赖最近列表。存档损坏或身份不匹配会明确失败并保留引用，不替换为另一帧。

发布前须重新确认预览摘要及 deferred 处置。发布事务创建 planned 后续中任务和细任务，仍需用户入队。即使期间返工形成不同的已接受候选，后续任务的 feedbackImage 仍读取原反馈尝试的 PNG、编号意见、相机及来源；完成后保持 awaitingAcceptance。

## 实现与边界

- Core 共用存档完整性读取，再按原尝试项目、运行、output checkpoint、路径/hash 和完整输出快照摘要绑定。保存反馈前在事务内复核候选，拒绝同时指定 image 和 previewFrame。
- Desktop 新增只读 objectTask.attemptFrames，支持最近列表和指定引用；不启动引擎、不调度任务。deferCandidateFeedback 接受严格 runId/frameId 引用。
- 不可变发布反馈保存原引用。自动创建的后续任务通过原发布反馈授权读取；普通发布版本引用继续使用独立的精确版本绑定，不能借用尝试存档。
- 沿用 PNG 校验、编号区域及 1 MiB 模型传输限制。超限只返回来源和大小限制，历史二维区域仍须在新版本重新定位，不宣称三维命中。

## 本轮验证

所有 native 命令在现有 PowerShell 工具链中通过 rtk proxy 执行，保留 MSVC/LIB 环境。

| 检查 | 结果 | 日志 |
| --- | --- | --- |
| cargo test --locked -p beaver-core publication_deferred | 7 通过 | output/attempt-archive-core.log |
| cargo test --locked -p beaver-core publication_followup | 5 通过 | output/attempt-archive-adjacent.log |
| cargo test --locked -p beaver-desktop publication_routes_accept_and_retain_history_without_redispatch | 1 通过 | output/attempt-archive-desktop.log |
| tsx 定向 deferred、draft、frame picker、followup 测试 | 16 通过 | output/attempt-archive-ui.log |
| npm run typecheck | 退出码 0 | output/attempt-archive-types.log |
| 定向 rustfmt、Prettier | 通过 | output/attempt-archive-format-check.log |
| npm run check:effective-lines | 1157 个源文件，17 个未改动历史超限文件，0 项违规 | output/attempt-archive-structure.log |
| git diff --check | 通过 | output/attempt-archive-diff-check.log |

新增 Core 回归覆盖错误尝试、运行、checkpoint、hash 的原子拒绝，双图片来源拒绝，精确读取、原请求幂等和计划不变。跨尝试回归使用实际 worker/RPC 子进程：旧反馈、新候选、发布、项目重开、原 PNG 与全部存档来源交付及待验收。归档记录由夹具构造；没有重复运行 Godot 采集。

Desktop 测试覆盖新只读路由及缺失存档在保存前拒绝。前端测试覆盖引用随草稿与不确定请求恢复、冲突图片来源拒绝、精确尝试/checkpoint/reference 校验，以及生产编号组件的静态回看。初次编译发现测试私有函数访问及 TypeScript 数组索引可空问题，已修正，最终检查通过。既有 unused_mut 和 validation State 字段警告未在本轮处理。

本轮未调用外部模型，未执行浏览器交互、Beaver.exe 原生验收或外部发布；worker/RPC 夹具结果不能替代这些验收。独立只读检查未发现明确的身份越界或静默丢失；列表遇到坏归档整体报错的 fail-closed 行为保留。

## 下一步

尝试编号存档仍需接入本轮返工。Blender 预览、真实引擎命中、跨版本拓扑重新定位及 F9 全路径集成收口仍未完成。F8.1-F8.5 和 F9 保持未勾选。未变更版本，未提交、推送或发布。
