# F3.3 提交草稿锁定与下一份草稿

日期：2026-09-25。范围：已提交计划只读保留、项目重开恢复、显式创建下一份可编辑草稿。

## 可用流程

对象任务草稿提交成功后保留原计划内容并显示锁定状态，编辑、保存、再次提交和 Codex 规划入口禁用。刷新或重新打开项目后仍能查看提交内容；已提交任务的修改继续使用现有定义修订入口。

用户点击“开始下一份草稿”后，编辑区清空并恢复编辑。界面明确说明已提交任务与历史不会删除；“重新加载提交状态”可恢复刷新失败或远端变化后的状态。创建下一份草稿不启动调度、不修改任务定义。

## 持久化与重试边界

- Core 在提交任务、对象、run 和回执的同一事务内递增草稿 revision，写入 `committedRequestId`；原提交计划及其 plan revision 保留。保存和规划采纳路径拒绝锁定草稿。
- `objectTask.unlockDraft` 通过已打开项目的 Runtime 路由，校验草稿 revision 与当前计划 revision，原子写入空白可编辑草稿和独立请求回执。
- 相同请求精确重放原回执；同 request ID 更换参数会失败。重放不会覆盖之后的编辑，UI 在回执后重新读取当前草稿与快照。其他窗口若已再次提交，UI 展示最新锁定状态。
- 提交响应丢失时，对象生成入口接受与原请求匹配的锁定草稿，继续重放原提交请求，避免重复生成任务。
- 提交已确认但刷新失败时，UI 保留只读内容和错误提示；重新加载后可继续。解锁失败保留锁定状态和相同请求身份供重试。
- 旧数据缺少可选锁定字段时保持兼容读取；本批不追溯回填历史版本已提交草稿的锁定状态。

## 实现位置

Core：`native/core/src/object_task_commit.rs`、`object_task_drafts.rs`、新增 `object_task_draft_unlock.rs` 与规划存储边界。Desktop：`object_task_catalog.rs`、`data_dispatch_object_tasks.rs` 和业务通知。共享协议：`src/shared/object-tasks.ts`。生产 UI：`object-task-workspace.ts`、新增 `object-task-workspace-state.ts`、`ObjectTaskDraftEditor.tsx`、`ObjectTaskPlanningPanel.tsx`；生成入口：`object-generation-session.ts`。

## 本批验证

- Core 26 项通过：`object_tasks::tests::draft` 5 项（含新锁定/重开/解锁回执回归 2 项）、`commit` 4 项、`revisions` 3 项、`object_task_planning` 14 项。命令为 `rtk proxy cargo test --locked -p beaver-core --lib <filter> -j 1`。
- Desktop 任务分发 8 项、业务效果 1 项通过：`rtk proxy cargo test --locked -p beaver-desktop --bin Beaver data_dispatch_object_tasks::tests -j 1` 与 `business_effects::tests`。分发测试同步补齐此前独立执行/发布路由的排除名单；首次旧断言失败已修复并复验。
- 前端 24 项通过：`npx tsx --test tests/object-task-workspace.test.ts tests/object-task-draft-lock.test.ts tests/object-task-rebase.test.ts tests/object-task-baseline.test.ts tests/object-generation.test.ts`。覆盖重开只读、控件门控、显式解锁、响应丢失重试、远端后续编辑和生成提交恢复。
- `npm run typecheck`、本批源文件 Prettier/Rustfmt 检查、`git diff --check`、UTF-8 无 BOM 检查通过。
- `npm run check:effective-lines` 通过：1045 sources、17 unchanged legacy、0 violations；workspace 477、草稿编辑器 483、Desktop 分发测试 375、解锁事务 81 effective lines。未改结构基线。

日志保存在 `output/f3-draft-lock-*.log`，结构报告为 `output/effective-code-lines.json`。最初两个文件名式 Cargo 过滤器匹配 0 项，只作为编译结果，不计入上述测试数；Desktop 的 `--lib` 调用无目标，已改为真实 `--bin Beaver` 测试。

## 剩余边界

F3.3 保持未勾选：真实 Codex 标题建议、稳定插入锚点和正式交互验收仍待完成。本批完成提交后的草稿锁定及显式下一份草稿；不新增提交前审批状态。未运行浏览器或原生完整验收，未生成、发布新 EXE，未改变版本号。
