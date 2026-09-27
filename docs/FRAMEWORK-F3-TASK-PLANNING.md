# F3.2 人工规划与未启动任务撤销实施记录

日期：2026-09-23。对应 [实际框架实现开发计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) 的 F3.2。本批接通人工草稿、未启动任务撤销和保留本地编辑的冲突恢复。当时 Codex 追问及提案、已提交任务的定义修订尚未接通，F3.2 保持未勾选；对象执行继续等待 F4。本批未构建或发布新的 Beaver.exe，未运行完整原生或游戏创作验收。

后续进度：已提交未启动任务的定义修订、复核、不可变历史及冲突恢复已在 [定义修订实施记录](FRAMEWORK-F3-TASK-REVISIONS.md) 中接通。本文保留人工规划与撤销批次的源码范围和原有 60 项测试证据；当前剩余 F3.2 能力为 Codex 追问及拆分提案。

2026-09-24 Codex 接入补充：规划 service 已接入桌面生命周期，提供 `objectTaskPlanning.start/get/answer/cancel/adopt` 路由；Codex worker 使用独立只读 app-server、项目 runtime 上下文和幂等回执，提案采纳只更新对象任务草稿，不创建执行任务。对象任务工作区新增规划面板，支持目标、验收要求、追问回答、取消和显式采纳。核心规划测试 14 项、桌面规划契约测试 1 项、对象任务桌面测试 12 项、前端对象任务测试 29 项、类型检查、定向格式检查和有效行检查通过；原生 release 构建产出 `target/release/Beaver.exe`（0.1.19.36 development）。正式原生验收仍受调用进程缺少合法 `BEAVER_API_TOKEN` 阻塞，F3.2 暂不勾选。

## 可观察结果与源码范围

原生“创作 / 任务”视图的“对象任务”面板现在支持编辑对象提案、粗/中/精任务提案及假设记录，保存到项目草稿，再显式提交规划。单个中修可以没有粗修父任务，精修可以追加到已有中修。已提交任务继续按真实身份、责任父任务、依赖和 run 展示；假设记录保留在计划历史中。

本批继续使用 `objectTask.saveDraft`、`objectTask.getDraft`、`objectTask.commit` 和一致性快照。前端工作区负责本地编辑、保存/提交和同步状态；`ObjectTaskDraftEditor.tsx` 与 `ObjectTaskAssumptionsEditor.tsx` 负责表单。`ObjectTasksPanel.tsx` 统一连接工作区、任务列表和撤销对话框，切换项目时重新建立该项目的状态。

撤销实现位于 `native/core/src/object_task_cancel.rs`；引用资格校验位于 `object_task_validation.rs`。桌面 API 继续复用原生 `objectTask.*` 分发，能力说明位于 `native/desktop/src/object_task_catalog.rs`。共享请求、回执和状态类型位于 `src/shared/object-tasks.ts`。前端撤销会话由 `object-task-cancellation.ts` 管理，`ObjectTaskCancellationDialog.tsx` 展示确认范围和结果，`ObjectTaskList.tsx` 展示任务与 run 的撤销状态。

## 撤销范围与事务

`objectTask.cancelPlanned` 沿 `parentTaskId` 收集责任后代，只撤销尚未启动的任务，并关闭这些中修拥有的 run。单独撤销精修时保留其上级中修和 run。已经撤销的记录保留原 revision；只增加本次实际改变的任务和 run revision，整个操作只推进一次计划 revision。

相同对象上的独立中修、前置依赖、依赖当前任务的其他任务、对象内容及假设历史均保留。列表明确标记已撤销依赖。撤销范围内出现已启动任务或 run，或任何待更新 revision 耗尽时，整次请求失败。任务、run、计划状态和请求回执在同一个事务中写入；回执写入失败会回滚这些变更。

请求和回执沿用已有字段，保留已保存请求的重放能力：

```text
request: projectId, taskId, requestId,
         expectedTaskRevision, expectedPlanRevision
receipt: projectId, taskId, requestId,
         previousTaskRevision, taskRevision,
         previousPlanRevision, planRevision
```

新任务不能再挂到已撤销父任务、run 或依赖上。在草稿中重新提出同 ID 的旧定义也不能掩盖其已撤销状态。单纯重提完全相同的已提交定义仍复用原记录；没有新记录时不推进计划 revision。

## 确认、重试与本地草稿

打开撤销对话框时固定所审阅的快照、目标任务、计划版本和责任范围，展示受影响的任务与 run。只为 `planned` 任务提供撤销按钮；只读列表调用方仍可省略操作入口。已撤销且没有工作基准的 run 显示“未执行（已撤销）”。

同一确认会话复用稳定的请求 ID 和预期 revision，重复点击不会并发提交。响应丢失后可以重试原请求；回执必须与项目、任务、请求及前后 revision 精确匹配。关闭对话框或切换项目后，旧响应不能覆盖新会话。成功回执在后续刷新失败时仍然保留；重新刷新不会再次发送撤销写入。

计划版本冲突时保留本地任务和假设内容，展示新的权威快照，并提供“保留本地内容并采用计划版本”操作。用户确认后重新检查远端；只有仍与刚才审阅的计划版本一致才采用。重新检查期间计划再次变化或本地内容被修改时，继续保留内容和冲突，要求再次确认。

工作区分别记录已保存草稿的计划基准和当前本地基准。即使正文完全相同，采用新计划版本后也会标记为待保存；后续刷新不会把旧的已保存基准误判为新的冲突。已有草稿和空白新草稿都可以保存采用后的基准。界面过滤已撤销父任务；草稿中原来选中的已撤销或缺失父任务仍显示为不可用，提示用户更换。

## 定向验证

本批通过 60 项定向测试：核心 20 项、桌面 9 项、前端 31 项。命令均从正常 PowerShell 工具链环境运行，原生构建保留 MSVC 环境。

```powershell
rtk proxy cargo test --locked -p beaver-core object_task --lib
rtk proxy cargo test --locked -p beaver-desktop object_task
rtk proxy npx tsx --test tests/object-tasks.test.ts tests/object-task-query.test.ts tests/object-task-workspace.test.ts tests/object-task-rebase.test.ts tests/object-task-cancellation.test.ts tests/object-task-cancellation-ui.test.ts
rtk npm run typecheck
rtk proxy cargo fmt --all -- --check
rtk npm run check:effective-lines
```

- [核心测试日志](../output/f32-continuation/core-object-task.log)：责任范围、精修独立撤销、已撤销记录 revision 保持、活动记录/版本耗尽拒绝、回执写入回滚、已撤销引用拒绝及原持久化回归。
- [桌面测试日志](../output/f32-continuation/desktop-object-task-rerun.log)：撤销重放、精修保留 run、中修连带范围、严格项目路由、错误保留和不唤醒调度器。
- [前端测试日志](../output/f32-continuation/frontend-object-task.log)：确认范围、重复提交、响应丢失重试、回执校验、关闭/项目切换隔离、刷新失败保留回执、草稿 rebase 竞态及界面静态渲染。

桌面测试首次失败来自按固定数组下标断言任务状态。快照按任务 ID 排序；修正为按 ID 查找精修并核对其与单任务查询结果一致后，9 项全部通过。该修正没有改变快照排序或撤销规则。Rust 格式检查发现两处相关模块声明顺序，运行官方 formatter 后复查通过。

本批前端格式检查命令：

```powershell
rtk proxy npx prettier --check `
  src/shared/object-tasks.ts `
  src/ui/object-tasks/object-task-workspace.ts `
  src/ui/object-tasks/object-task-cancellation.ts `
  src/ui/object-tasks/ObjectTaskCancellationDialog.tsx `
  src/ui/object-tasks/ObjectTasksPanel.tsx `
  src/ui/object-tasks/ObjectTaskList.tsx `
  src/ui/object-tasks/ObjectTaskDraftEditor.tsx `
  src/ui/object-tasks/object-tasks.css `
  tests/fixtures/object-tasks.ts `
  tests/object-task-workspace.test.ts `
  tests/object-task-rebase.test.ts `
  tests/object-task-cancellation.test.ts `
  tests/object-task-cancellation-ui.test.ts
rtk proxy npx prettier --check docs/FRAMEWORK-F3-TASK-PLANNING.md docs/FRAMEWORK-IMPLEMENTATION-PLAN.md docs/README.md
```

检查结果见 [类型日志](../output/f32-continuation/typecheck.log)、[前端格式日志](../output/f32-continuation/frontend-format-check.log)、[Rust 格式日志](../output/f32-continuation/rust-format-check-rerun.log)、[文档格式日志](../output/f32-continuation/docs-format-check.log) 和 [有效行日志](../output/f32-continuation/effective-lines-rerun.log)。本批只运行所列前端文件的定向格式检查，未重新运行全仓前端格式门槛。

结构检查扫描 815 个源文件，保留 17 个未变更历史文件，0 个违规；本批未更新历史基线。较大的本批文件为 `object-task-workspace.ts` 477 行、`ObjectTaskDraftEditor.tsx` 437 行、`object-task-cancellation.test.ts` 330 行、`object_task_dispatch_tests.rs` 272 行，均为有效行数。工作区负责草稿和同步状态，编辑器负责同一规划表单，撤销会话已独立拆出；测试分别覆盖会话协议和原生 API。源码、测试和本批文档保持 UTF-8 无 BOM。

本轮补充了稳定位置字段：草稿任务按编辑器数组顺序写入 `position`，新增、删除和首尾/相邻插入会重编号；原生快照按 `position` 和任务 ID 排序，旧记录缺失字段时兼容为 0。核心回归测试覆盖显式位置排序。该能力只解决持久化顺序，提交锁定、解锁草稿和真实 Codex 标题建议仍未接通。

## 后续入口

本批交接时的下一步包括 Codex 需求追问、合理假设、拆分提案和已提交未启动任务的定义 revision 修订。后续定义修订已经完成，见 [对应实施记录](FRAMEWORK-F3-TASK-REVISIONS.md)；当前继续接入 Codex 到同一草稿/确认流程。现有人工表单不表示这些模型能力已经接通。F3.3 的标题建议、锁定/解锁与插入位置，以及 F3.4 的完整泳道、对象迭代栈和制造导航继续待办。

对象任务仍不创建旧任务记录、不唤醒旧调度器。F4 完成对象队列、写入权、领取基准和共享工作区之前，对象执行保持关闭。本批撤销只覆盖未启动计划；运行中的取消、成果处置和发布恢复继续由 F4/F7 完成。F1 迁移收口、F2.4/F7 导入准备及正式提交边界保持原计划状态。
