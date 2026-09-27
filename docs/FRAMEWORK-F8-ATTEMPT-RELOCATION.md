# F8.5 历史尝试编号帧对应证据

日期：2026-09-26。完成同一制作轮次内历史尝试编号帧到当前候选编号帧的人工对应子流程；F8.5、F8 和 F9 整体仍未完成。

## 可用流程

候选审阅可选择同一项目、对象及轮次内另一项已有输出的尝试，读取其不可变编号帧。用户需要逐区指定当前目标区域，或明确填写无对应区域的原因，再确认提交。本轮返工进入既有费用确认；延后反馈进入后续中修，发布时创建 planned 后续任务，不自动执行。

部分对应、文字和帧身份保存在反馈草稿中，重载不自动提交。更换目标帧、来源帧或来源尝试会清空已有对应决定及确认。来源或目标图片缺失、项目身份不符、PNG 损坏或尺寸不符时，界面阻止确认及提交，保留草稿供刷新恢复。

返工、延后反馈、发布记录及后续上下文保留来源尝试、原始来源帧、目标帧和逐区决定。原坐标始终属于原始存档；回看同时显示两张原 PNG 及编号意见。Core 校验完整有序的来源区域覆盖、目标区域边界、非空无对应原因、不同尝试及存档身份，拒绝越界、跨范围和过期目标。

延后反馈的对应关系只在保存反馈时的检查点确认。后续候选或发布版本可能变化，任务提示仍要求重新定位；本批未增加最终发布时的再次对应门禁。

## 实现位置

- [Core 对应校验](../native/core/src/object_feedback_relocation.rs)：供现有返工及延后反馈链调用，沿用不可变帧解析和图片交付。
- [共享契约](../src/shared/preview-feedback-relocation.ts)及[草稿逻辑](../src/ui/object-tasks/feedback-relocation-draft.ts)：来源身份、逐区决定及确认状态。
- [对应编辑器](../src/ui/object-tasks/FeedbackRelocationEditor.tsx)及[摘要](../src/ui/object-tasks/FeedbackRelocationSummary.tsx)：生产候选审阅、费用确认与发布回看。
- [Desktop schema](../native/desktop/src/object_feedback_relocation_catalog.rs)：扩展既有操作参数，没有新增独立端点。

## 功能验证

本次收尾复核前一会话保存的日志，相关源文件在最后一次行为验证后没有修改，复用已通过结果。

1. Core 新增回归 3 项通过：不完整、跨范围和过期请求不写入；延后反馈重开、发布及两张原图交付；返工回执恢复及普通重试继承对应关系。见 [Core 日志](../output/f8-attempt-relocation-20260926-201002/core-relocation.log)和[测试](../native/core/src/object_feedback_relocation_tests.rs)。新增双图测试使用直接 callback 调用，不代表新增双图 RPC 端到端覆盖。
2. Core 相邻回归 6 项通过，包含已有原始图片及编号帧的 worker/RPC 交付覆盖。使用下列三个过滤器，实际执行数分别为 1、1、4。见[有效相邻日志](../output/f8-attempt-relocation-20260926-201002/core-adjacent-verified.log)。早先 `core-adjacent.log` 的错误过滤器执行了 0 项，不计入证据。
3. Desktop `publication_routes_accept_and_retain_history_without_redispatch` 1 项通过，包含无目标及同尝试来源拒绝。见 [Desktop 日志](../output/f8-attempt-relocation-20260926-201002/desktop.log)。
4. 前端 15 项通过，覆盖对应契约、草稿、延后原请求、返工及帧选择器；`npm run typecheck` 通过。见[前端日志](../output/f8-attempt-relocation-20260926-201002/frontend.log)及[类型检查](../output/f8-attempt-relocation-20260926-201002/typecheck.log)。

```powershell
rtk proxy cargo test --locked -p beaver-core --lib feedback_relocation
rtk proxy cargo test --locked -p beaver-core --lib object_publication_deferred::rework_frames::
rtk proxy cargo test --locked -p beaver-core --lib object_publication_deferred::frames::
rtk proxy cargo test --locked -p beaver-core --lib object_publication_deferred::delivery::
rtk proxy cargo test --locked -p beaver-desktop publication_routes_accept_and_retain_history_without_redispatch
rtk proxy npx tsx --test tests/feedback-relocation.test.ts tests/object-candidate-feedback-draft.test.ts tests/object-publication-deferred.test.ts tests/object-candidate-rework.test.ts tests/publication-frame-picker.test.ts
rtk npm run typecheck
```

## 浏览器证据

[夹具](../output/playwright/f8-attempt-relocation/harness.tsx)使用生产候选返工、恢复确认、发布及任务执行组件，API 为模拟响应，图像为真实可解码的红色历史 PNG 和绿色当前 PNG。没有调用真实引擎或外部模型。

[主要流程日志](../output/f8-attempt-relocation-20260926-201002/browser.log)和[重开日志](../output/f8-attempt-relocation-20260926-201002/browser-reopen.log)记录以下通过项：

- 部分对应与文字重载恢复；未完整对应无法确认；更换三个身份入口会清空决定。
- 缺失、跨项目、损坏及尺寸错误图片阻止提交，刷新恢复后可继续。
- 本轮返工费用确认显示无对应原因，提交保留准确目标和映射。
- 延后提交成功但响应丢失，重载后锁定原请求；重试请求完全一致且仅写入一次。
- 发布及再次重载后回看两张不同的 400 × 200 原 PNG，无可见回放错误。
- 先开启对应再选择独立 PNG 时隔离冲突控件；清除 PNG 并关闭对应可恢复普通文字反馈。

截图：[部分草稿恢复](../output/playwright/f8-attempt-relocation/partial-restored.png)、[返工费用确认](../output/playwright/f8-attempt-relocation/rework-confirmation.png)、[发布回看](../output/playwright/f8-attempt-relocation/published-replay.png)、[双帧详情](../output/playwright/f8-attempt-relocation/replay-detail.png)。收尾时人工复核双帧详情，可见历史区域 1 对应当前区域 1，历史区域 2 标记无对应并保留原因；两张原图和编号均保留。

专用 Playwright 会话 `f8-relocation` 已关闭；收尾检查时本地夹具端口 `57449` 已无监听，无需结束其他进程。

## 最终检查与边界

本次新执行 `rtk proxy cargo fmt --all -- --check`、18 个相关前端源文件及测试的官方 Prettier 检查、定向 `git diff --check`，均通过。37 个相关文件为有效 UTF-8 且无 BOM。`rtk npm run check:effective-lines` 通过：1193 个源文件，17 个未改动的历史超限文件，0 项违规；未更新结构基线。日志保存在[本轮目录](../output/f8-attempt-relocation-20260926-201002/)，分别为 `final-rust-format.log`、`final-prettier.log`、`final-diff.log` 和 `final-effective-lines.log`。

本批仅覆盖同轮次不同尝试的输出编号存档，不覆盖已发布版本之间或独立 PNG 的对应。人工二维区域对应不提供自动三维拓扑迁移或精确命中保证。原图模型交付沿用现有 1 MiB 限制及来源回退语义。没有运行本轮原生 EXE、真实 Godot/Blender、外部模型或完整发布验收；F8.5、F8、F9 保持未勾选。
