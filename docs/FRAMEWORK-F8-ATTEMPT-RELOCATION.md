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

## 2026-10-01 增量：最终候选逐区重新确认

本增量关闭“已有编号反馈 → 准确最终候选输出 → 用户明确接受/发布”的源码工作流。上文 2026-09-26 的检查点对应及未完成边界作为历史记录保留；其中“未增加最终发布时的再次对应门禁”不再代表当前源码。F8.5、F8 和 F9 整体仍未完成。

### 当前可用流程

生产发布面板读取原编号帧和本次最终尝试的不可变 output 编号帧。每个原区域必须按顺序对应一个最终区域，或填写非空的无对应原因，再由用户重新勾选逐区确认；既有反馈检查点映射不能替代这次授权。支持同轮尝试来源、已发布编号帧的后续反馈，以及延后任务继承的原始编号帧来源，保留原版本、原轮次及原始坐标，不改写历史图像或将二维框冒充三维命中。

最终确认绑定反馈来源摘要和准确的目标帧、尝试、output 摘要及文件哈希。Core 在新建发布、未完成操作重试和提交边界重新校验来源与目标；缺少新确认、映射不完整、无对应原因为空、来源跨范围、目标过期或 PNG 损坏时阻止发布。持久发布日志冻结完整确认，响应丢失后重开只查询，必须由用户明确重试原请求；未完成操作仍可中止，终态回执可重放，旧 schema 1 日志不被静默升级为新授权。

逐区映射、无对应原因和接受说明可恢复，但整页重开不会恢复文件归属、替换授权或最终区域确认。更换来源摘要或目标帧清空映射与确认；取消确认或修改映射立即禁用发布。图片读取失败时保留草稿、禁止授权，恢复图片也不会自动确认或提交。

浏览器正向验证发现并修复一处实际门禁问题：草稿与回执的对象属性顺序不同，原始 `JSON.stringify` 比较会误拒合法确认。现在经同一严格 schema 规范化后比较，仍要求当前完整映射与已验证回执一致，没有放宽未确认、过期或图片损坏保护。

### 实现位置

- [Core 来源继承](../native/core/src/object_publication_feedback.rs)与[最终确认校验](../native/core/src/object_publication_relocation.rs)：接入既有发布准备、文件日志和最终事务。
- [共享契约](../src/shared/preview-feedback-relocation.ts)与 [Desktop schema](../native/desktop/src/object_feedback_relocation_catalog.rs)：扩展既有发布参数及原帧读取，沿用生产 API。
- [最终对应编辑器](../src/ui/object-tasks/FinalRelocationEditor.tsx)、[发布批准](../src/ui/object-tasks/PublicationApproval.tsx)及[草稿/回执校验](../src/ui/object-tasks/final-relocation-draft.ts)：生产界面、草稿恢复和即时撤销授权。

### 本次验证与证据

证据目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-final-relocation-20261001`。以下结果来自本次实现后的有效日志；后续仅修改临时浏览器证明和文档，不重复运行未受影响的应用检查。

1. Core 发布过滤器 **37 项通过**；覆盖新的最终确认拒绝/无写入、准确发布来源继承、过期或损坏目标、未完成操作重开/原请求重试/中止及旧日志回放。见 [Core 最终日志](../../AI/GameEditor/linshi/beaver-final-relocation-20261001/core-publication-final.log)。
2. Desktop 发布 **2 项通过**；共享夹具的候选审阅和阶段推进消费者分别 **1 项通过**。测试夹具改为合法 PNG 和明确 `workspaceWrite` 沙箱回执，生产图像/沙箱校验没有放宽。见 `desktop-publication-final.log`、`desktop-candidate-adjacent.log`、`desktop-advance-adjacent.log`。
3. 前端完整定向批次 **45 项通过**；修复属性顺序门禁后，受影响的最终对应、发布批准和原请求/跨窗口批次 **21 项通过**。两批有重叠，不合计为 66 项。见 `frontend-tests.log`、`frontend-final-gate.log`。
4. TypeScript、定向 Rustfmt、Prettier、有效行及差异检查通过。有效行结果为 **1224 sources、17 个未改动历史超限文件、0 项违规**，没有更新结构基线。

```powershell
rtk proxy cargo test --locked -p beaver-core --lib publication
rtk proxy cargo test --locked -p beaver-desktop publication
rtk proxy cargo test --locked -p beaver-desktop candidate_review_routes_final_output_and_replays_without_dispatch
rtk proxy cargo test --locked -p beaver-desktop approved_successor_launches_once_and_reopens_with_immutable_approval
rtk proxy npx tsx --test tests/final-relocation.test.ts tests/final-relocation-workflow.test.ts tests/object-publication.test.ts tests/object-publication-approval.test.ts tests/object-publication-windows.test.ts
rtk proxy npm run typecheck
rtk proxy npm run check:effective-lines
```

Chromium 运行真实生产 `ObjectPublicationPanel` / `ObjectPublication`、localStorage 和 Web Locks，但 API 为模拟响应、PNG 由 canvas 生成，不是原生或真实引擎验收。[浏览器结果](../../AI/GameEditor/linshi/beaver-final-relocation-20261001/browser/proof.json)记录三组通过项：合法逐区确认与重开撤销授权；响应丢失后零自动提交并显式重试完全相同原请求；完整已保存草稿在损坏 PNG 时仍阻止发布，图片恢复后映射保留但不恢复授权。截图及原始日志保存在同目录。

`retry-browser.log` 的早期目标帧预期错误，以及损坏图证明的错误文案/不可用下拉选项断言失败，均作为脚本失败证据保留，不计入通过项；只修正临时脚本，未清空草稿或放宽产品保护。

### 尚未完成的边界

此增量覆盖已有编号存档的人工二维对应，不覆盖独立 PNG 对应、完整三维旧拓扑迁移或任意已发布版本间的通用迁移。未编译新 EXE，未验证 native WebView、真实模型/引擎和完整发布验收；旧 `.42` EXE 不包含本次门禁。原生编号反馈验收中的真实候选仍 `awaitingAcceptance` / `awaitingGate`，发布数为 0；本轮未代替用户接受或发布它。F7、F8.5、F8、F9 及整体完成条件保持未勾选。

## 2026-10-01 增量：独立冻结 PNG 最终确认

本增量补齐“独立冻结 PNG 的编号反馈 → 准确最终 output 编号帧 → 明确确认与发布”的源码工作流。上一个增量中的“独立 PNG 未覆盖”是该批次的历史边界，不再代表当前源码；完整三维旧拓扑、通用版本迁移和原生最终发布验收仍未完成。

### 当前行为与保存边界

生产面板直接读取反馈绑定的原始冻结 PNG，不伪造编号帧，也不接受任意替换图片。读取绑定原项目、原轮次、原尝试、output 检查点、路径和 SHA-256；继承的延后反馈继续使用原轮次。Core 共享已有冻结文件读取和完整 PNG 解码校验，核对真实尺寸、来源对象及资源限制。发布来源摘要绑定完整原反馈与原图 SHA-256，区域数来自原反馈。

原图成功读取和解码后，界面显示原 PNG、编号叠层与局部缩略图。每个原区域必须逐一对应准确最终候选的编号区域，或填写非空无对应原因，再由用户重新确认。最终目标继续严格绑定 attempt、run、output 摘要、路径和哈希；本次没有放宽原有目标校验。

原图缺失、损坏、尺寸错误或响应身份不符时，禁用对应输入、最终确认及发布，完整已保存的映射和说明保留。点击“重新读取原始冻结图片”只重读同一原请求；读取恢复后映射可用，但授权不恢复。整页重开同样保留映射、无对应原因和接受说明，清除文件授权与最终确认，不自动提交。

Core 在发布预览、新建日志、pending 原请求重试和提交边界重新验证原 PNG。日志建立后发现来源损坏，保留未完成操作供重开、原请求重试或中止；终态回执、列表和中止不新增对 live PNG 的依赖。沿用既有 journal schema，未改 `prepare::build` 或冻结日志 integrity，不静默升级旧授权。

### 实现与回归

- [原始 PNG 来源身份](../src/ui/object-tasks/final-relocation-source.ts)与[原图读取组件](../src/ui/object-tasks/FrozenFeedbackImage.tsx)：生产原图展示、readiness 和失败后的原请求重读。
- [Core 发布反馈](../native/core/src/object_publication_feedback.rs)、[冻结文件读取](../native/core/src/object_attempt_file.rs)及[原图验证](../native/core/src/object_rework_image.rs)：复用权威 attempt，避免事务内重复读取 store。
- [PNG 发布测试](../native/core/src/object_publication_png_tests.rs)、[PNG 恢复测试](../native/core/src/object_publication_png_recovery_tests.rs)及[前端 PNG 测试](../tests/final-relocation-png.test.ts)：完整映射、无写入拒绝、来源故障、延后继承、pending 恢复、终态回放和不可变原请求。

证据目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-final-relocation-png-20261001`。应用门禁通过后没有再修改生产代码；收尾复核有效日志，仅补跑错误过滤器遗漏的相邻回归，其余应用检查不重复。

| 有效检查                             | 结果                                                  | 日志                                                            |
| ------------------------------------ | ----------------------------------------------------- | --------------------------------------------------------------- |
| Core publication                     | 40 项通过，含 4 项 PNG 工作流回归                     | `core-publication.log`                                          |
| Core attempt-file                    | 3 项通过                                              | `core-attempt-file.log`                                         |
| Core 相邻返工图片与交付              | 5 项通过                                              | `core-rework-image-verified.log`                                |
| Desktop publication                  | 2 项通过                                              | `desktop-publication.log`                                       |
| 前端定向批次                         | 52 项通过，0 失败                                     | `frontend-focused.log`                                          |
| TypeScript / 定向 Rustfmt / Prettier | 通过                                                  | `typecheck.log`、`prettier-check.log`；Rustfmt 复用前段有效结果 |
| 有效行检查                           | 1229 sources、17 unchanged legacy files、0 violations | `effective-lines.log`、`effective-code-lines.json`              |

各测试批次有重叠，不合计为不重复测试数。原 `core-rework-image.log` 实际执行 0 项，保留但不计覆盖；正确过滤器 `rtk proxy cargo test --locked -p beaver-core --lib object_candidate_rework::image::` 执行 5 项通过。未更新结构 baseline。机器汇总见 [gate-proof.json](../../AI/GameEditor/linshi/beaver-final-relocation-png-20261001/gate-proof.json)。

### Chromium 证明与未覆盖项

[浏览器汇总](../../AI/GameEditor/linshi/beaver-final-relocation-png-20261001/browser/proof.json)的三组结果均通过：完整对应后仍需显式授权，取消或重开立即失效；四类原 PNG 故障（missing、decode、dimensions、identity）均阻断且保留草稿，显式重读准确原请求后不恢复授权；发布响应丢失后重开发送 0 次，显式重试的完整 JSON 与原请求完全相同，模拟终态重开不再发送。

浏览器使用真实生产 `ObjectPublicationPanel` / `ObjectPublication`、localStorage 和 Web Locks，但 API 为模拟响应、PNG 由 canvas 生成；不能称为 native WebView 或真实发布验收。人工复核 [合法确认截图](../../AI/GameEditor/linshi/beaver-final-relocation-png-20261001/browser/approval-ready.png)与[身份错误阻断截图](../../AI/GameEditor/linshi/beaver-final-relocation-png-20261001/browser/source-identity-blocked.png)，原图编号、最终对应和禁用状态可见。命名浏览器会话与本轮服务器已关闭，端口 55886 无监听，见 `browser/cleanup-proof.json`。

本轮没有调用外部模型、上传任意图片、查询或批准真实宿主候选，也没有编译新 EXE、改版本、提交或推送。旧 `.42` EXE 不包含本轮源码；前段记载的真实候选状态不是本次实时查询结果。F7、F8.5、F8、F9 与整体完成条件保持未勾选。
