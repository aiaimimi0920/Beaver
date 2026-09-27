# F7 最终候选反馈返工记录

日期：2026-09-25。本次完成“审阅最终候选、填写反馈、明确确认并返工最后细任务”的纵向流程，贯通 Core、项目持久化、Desktop API、Scheduler 和生产执行窗口。F5.4、F7.1 及框架总计划保持未完成。

## 可使用的流程

最后细任务成功并生成整体候选审阅后，用户验证当前恢复状态，在审阅记录下填写反馈，再准备返工。确认面板展示审阅 ID、原尝试、细任务、反馈、所有权保留规则及模型执行成本提示。明确确认后，系统创建同一细任务的新尝试，以前次输出为输入，并把反馈及审阅来源加入冻结提示词，再交给 Scheduler 执行。

原尝试、审阅记录、检查点和此前已接受阶段保持不变。返工保留对象所有权，不接受最终结果、不物化项目文件、不发布版本、不释放队列。新结果仍需重新检查和审阅。

丢失响应时保留原请求并精确重试；完成的请求返回不可变回执，不再次取得执行租约。待处理日志在重开后仍可恢复，重开本身不启动模型。同一请求 ID 改变反馈会返回冲突。

## 实现与持久化边界

- Core 的 object_candidate_rework.rs 校验明确的 rework 审批：reviewRequestId、attemptId、fineTaskId 和 feedback。只允许返工已审阅且当前成功、处于 AwaitingGate 的最后细任务；rework 与 advance 互斥。
- 反馈必须非空且不超过 4,000 UTF-8 字节；加入来源和反馈后的冻结提示词不超过 20,000 UTF-8 字节。新尝试保留 fine 身份，输入衔接原输出，不修改前序接受状态。
- object_candidate_review.rs 与 object_recovery_resume_store.rs 在锁外文件验证前后核对冻结恢复记录、当前目录及阶段来源。工作副本漂移或验证期间目录变化会阻止提交。返工不要求新的技术检查成功，以允许修复已有问题；文件完整性及恢复边界仍需通过。
- 沿用持久恢复日志、单次执行租约和不可变回执。object_recovery_attempt_history.rs 只为明确返工允许从 AwaitingGate 建立同一 fine 的历史边，保留 revision、冻结定义及输入约束。
- Desktop 新增 objectTask.reworkCandidate，校验方法与请求用途一致，复用项目路由和 Scheduler。共享 TypeScript 契约校验用途互斥及回执中的 fine/attempt 身份。
- 生产 ObjectTaskExecutionDialog 接入 ObjectCandidateReworkPanel 和既有恢复确认面板。提交前核对项目、当前 revision、完整 attempt target 及末项身份；含糊响应保留请求，避免用户重试造成重复执行。

## 本次验证

日志保存在 output/f7-rework-*.log。本轮完成边界的结果如下；文档收尾重新核对了这些日志。

| 检查                                                                                                                                                           | 实际结果                                                              |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| cargo test --locked -j 1 -p beaver-core --lib object_candidate                                                                                                 | 5 passed                                                              |
| cargo test --locked -j 1 -p beaver-core --lib object_recovery                                                                                                  | 20 passed                                                             |
| cargo test --locked -j 1 -p beaver-core --lib object_stage_advance                                                                                             | 4 passed                                                              |
| cargo test --locked -j 1 -p beaver-core --lib object_attempt_check                                                                                             | 3 passed                                                              |
| cargo test --locked -j 1 -p beaver-desktop --bin Beaver object_attempt_runtime                                                                                 | 11 passed                                                             |
| npm run typecheck                                                                                                                                              | 通过                                                                  |
| npx tsx --test tests/object-candidate-rework.test.ts tests/object-candidate-review.test.ts tests/object-stage-advance.test.ts tests/object-task-resume.test.ts | 15 passed                                                             |
| npx tsx --test tests/object-recovery-contract.test.ts tests/object-task-resume-execution.test.ts tests/object-task-recovery-ui.test.ts                         | 9 passed                                                              |
| 定向 Prettier 格式化和检查                                                                                                                                     | 通过                                                                  |
| 定向 rustfmt --edition 2021 --config skip_children=true --check                                                                                                | 通过                                                                  |
| npm run check:effective-lines                                                                                                                                  | 997 sources，17 unchanged legacy files，0 violations；未放宽 baseline |
| git diff --check                                                                                                                                               | 通过                                                                  |
| 本次涉及源码及测试 UTF-8 BOM 检查                                                                                                                              | 无 BOM                                                                |

Core 新增 3 个返工回归，覆盖反馈和输出衔接、已接受前序不变、不可变历史、所有权、关闭重开、精确重放、后续取消、待处理日志、反馈冲突、单次租约、缺少审阅、工作副本漂移及锁外验证期间目录变化。

Desktop 通过公开 API、实际 Scheduler 和确定性 RPC 子进程执行返工。夹具读取实际 turn/start 输入，只有收到 Reduce movement speed 才写出 feedback received: Reduce movement speed；测试读取工作副本 result.txt 验证该结果，证明反馈已进入执行进程。此证据不包含真实模型效果评价。

生产 UI 控制器回归覆盖旧尝试/审阅拒绝、空反馈、确认前不启动、反馈和成本提示、丢失响应精确重试、待处理请求重开不启动、保存的路由和载荷、错误 fine 回执、UTF-8 字节上限及用途冲突。

最初未限定 --lib 的 Core 命令并行编译多个集成目标，因 Windows 页面文件不足而失败（failed to mmap，os error 1455）。收窄为 --lib -j 1 后以上检查通过；最初失败日志已由定向成功重跑覆盖。未改动产品逻辑来绕过该环境问题。

## 剩余工作与验收范围

本轮只覆盖最终 fine 的明确反馈返工。F5.4 的任意阶段及下游范围重做、下游过期标记仍待实现。反馈提交尚未构成完整的反馈审阅与关闭策略。

F7.1 仍需最终人工接受、对象整体决定、所需阶段及有效报告策略、固定引用闭包、文件归属和未处置反馈的完整检查。审阅仍保留 FINAL_FINE_OWNER_ACCEPTANCE_REQUIRED、FEEDBACK_REVIEW_UNAVAILABLE、PUBLICATION_NOT_IMPLEMENTED 阻塞项。

F7.2 仍需持久文件日志、受管路径物化、崩溃恢复及一起更新接受版本、任务状态和队列释放的最终事务；后续发布和导入闭环继续按计划推进。返工成功不代表上述条件满足。

本轮未构建或启动 Beaver.exe，未运行真实模型、Godot/Blender、浏览器视觉或正式发布验收；未提交、推送、发布或修改版本。定向开发检查通过后未扩展为全量验收。
