# F4.4 显式重试执行交付证据

日期：2026-09-25。承接会话 `01a0d651-6653-7361-b6b7-a9595035347a` 的 Beaver 开发计划。此次完成失败或中断 fine 的显式重试工作流；F4.4 整体仍未完成。

## 可用流程

在生产执行记录中核验当前记录与工作区，确认新模型执行及可能产生的费用，然后启动新尝试。新尝试以原执行保存的输出检查点为输入，保留原执行和所有后续尝试的历史。原中断回执不会阻止用户中断新的执行。

`objectTask.resumeRecovery` 校验完整项目、任务、对象、run、owner、claim、revision 和核验代次。文件检查在数据库锁外完成，提交前再核对记录、暂停状态及取消标志。只有首次成功提交返回内部执行 lease；原请求重放只返回不可变回执。Scheduler 在扫描前保留槽位，接收方丢失响应不取消已提交 worker，也不产生第二次启动。

前端在响应不明、回执不匹配或尚未完成时保留完整原请求。项目重开可读取 pending 请求，由用户显式重试。已提交 Running 但没有活动 worker 的记录保持恢复待处理状态；读取、核验、重放或重开均不自动启动外部执行。AwaitingGate 不允许走本重试入口。

## 验证结果

- Core `cargo test --locked -p beaver-core object_task --lib`：146 passed，0 failed，见 [日志](../output/f4-resume-core-final.log)。包含 4 项新恢复重试回归：多次重试检查点与历史、pending 重开与精确请求、文件漂移/取消/记录变化、拒绝待验收执行。
- Desktop `cargo test --locked -p beaver-desktop object_attempt_runtime`：8 passed，0 failed，见 [日志](../output/f4-resume-desktop.log)。新用例覆盖并发请求、只启动一次、关闭 Scheduler 后回执重放、项目重开历史和项目存储隔离。
- Desktop `object_task`：20 passed，0 failed，见 [日志](../output/f4-resume-desktop-tasks.log)；`business_effects`：1 passed，0 failed，见 [日志](../output/f4-resume-effects.log)。
- 前端恢复、执行和契约定向回归：48 passed，0 failed，见 [日志](../output/f4-resume-ui.log)；新旧执行中断切换回归：1 passed，0 failed，见 [日志](../output/f4-resume-execution-ui.log)。包含确认前不发送、响应丢失后精确重试、pending 重开不自动调用、非法回执、刷新失败保留回执和 blocked 后重新核验。
- `npm run typecheck`、相关文件 `prettier --check`、`cargo fmt --all -- --check`、`git diff --check` 均通过。日志分别为 `output/f4-resume-typecheck-final.log`、`output/f4-resume-prettier-check.log`、`output/f4-resume-rustfmt.log`、`output/f4-resume-diff-check.log`。
- `npm run check:effective-lines` 通过：964 sources，17 unchanged legacy files，0 violations；见 [日志](../output/f4-resume-structure.log)。未修改结构 baseline。

初次 Core 回归有一项 Windows 文件锁错误（os error 33）；最终同范围运行 146 项全部通过，未为此放宽断言。初次 Desktop 命令误加 `--lib`，随后测试编译发现 String 参数缺少转换，修正后上述正式范围通过。保留日志，不将早期失败记为通过。

## 实现位置

- `native/core/src/object_recovery_resume.rs`、`object_recovery_resume_store.rs`：重试编排、持久请求、回执和事务提交。
- `native/core/src/object_recovery_attempt_history.rs`、`object_attempt_view.rs`：前驱链与历史执行读取。
- `native/core/src/scheduler_object_resume.rs`、`object_attempt_worker.rs`：槽位、项目 gate、取消及新 lease 执行。
- `native/desktop/src/object_attempt_runtime.rs`、`object_task_catalog.rs`：typed API 和严格业务目录。
- `src/ui/object-tasks/object-task-resume.ts`、`ObjectTaskResumePanel.tsx`：确认、原请求重试、创建回执与恢复入口。
- `tests/object-task-resume.test.ts`、`tests/object-task-resume-execution.test.ts`：客户端行为与新执行控制回归。

## 剩余边界

受管 Godot/Blender 会话及资源恢复仍是 F4.4 的下一入口。F5 门槛决策、后继 fine 推进、接受与发布未实现；本次重试不会跳过验收或释放对象所有权。没有生成新 EXE，没有运行完整原生验收、真实模型调用或浏览器视觉验收。此次结果仅为已实现切片的定向开发验证，不代表发布验收。
