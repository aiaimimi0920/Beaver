# F7 整体候选审阅记录

日期：2026-09-25。本次完成最后细任务成功后的持久化候选审阅流程，贯通 Core、Desktop API 与生产执行记录界面。F7.1 和框架总计划保持未完成。

## 可使用的流程

最后一个未取消的细任务成功后，用户先完成冻结输出的技术检查，再点击“生成整体候选审阅记录”。界面展示原始制作基线、领取时和审阅时的接受版本、对象 revision、阶段定义及验收要求、固定引用版本、完整文件清单、前后 SHA-256、登记归属、本次技术复查和未满足的发布条件。

审阅只生成证据，保留对象占用。它不会接受最后细任务、更新接受版本、物化项目文件、启动后继或释放队列。历史审阅显示生成当时的状态；再次检查需创建新的审阅请求。

请求丢失响应后可精确重试。关闭窗口会忽略迟到响应，重开只查询历史，不自动生成审阅。取消并删除工作副本后，已保存的审阅仍可查询和重放；此时创建新的审阅会被拒绝。

## 实现与边界

- Core 的 object_candidate_review.rs 校验项目、完整 attempt Target、当前持有对象的 run、成功等待状态、指定的通过报告、最终 fine 及此前 fine 的接受状态。锁外复查冻结 blob，事务写入前重新比较来源记录；请求 ID 不允许绑定不同参数。
- object_candidate_summary.rs 从原始 run 的 FrozenBaseline 计算完整文件并集，保留未变更文件以供归属和引用审阅。比较对象 revision 与接受版本漂移；引用变更、缺失或冲突归属、技术失败形成阻塞项。差异比较不使用上一 fine 的输入作为原始基线。
- 项目数据库保存冻结来源和不可变报告；重放和查询会复算派生摘要并检查一致性。该机制用于记录一致性检查，未提供防数据库篡改的密码学认证。
- Desktop 提供 objectTask.prepareCandidateReview 和 objectTask.candidateReviews，使用项目路由、严格请求解码和 spawn_blocking。生产 ObjectTaskExecutionDialog 在成功末项展示 ObjectCandidateReviewPanel。
- 现有技术检查仅覆盖冻结内容完整性和代码结构。报告无条件保留最后 fine 人工接受、反馈处置接入和发布实现三个阻塞项，不能作为发布许可。

## 本次验证

日志位于 output/f7-candidate-*.log。

| 检查                                                                                                                                                             | 实际结果                                   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| cargo test --locked -p beaver-core --lib object_candidate_review                                                                                                 | 2 passed                                   |
| cargo test --locked -p beaver-desktop object_attempt_runtime                                                                                                     | 11 passed                                  |
| npm run typecheck                                                                                                                                                | 通过                                       |
| npx tsx --test tests/object-candidate-review.test.ts tests/object-stage-advance.test.ts tests/object-task-execution*.test.ts tests/object-attempt-checks.test.ts | 39 passed                                  |
| 定向 rustfmt --edition 2021 --config skip_children=true                                                                                                          | 通过                                       |
| 定向 Prettier 格式化及新增关闭重开测试的格式检查                                                                                                                 | 通过                                       |
| npm run check:effective-lines                                                                                                                                    | 993 sources，0 violations；未放宽 baseline |
| git diff --check                                                                                                                                                 | 通过；存在既有 LF/CRLF 提示                |
| 本次涉及的 17 个源码文件 UTF-8 / BOM 检查                                                                                                                        | 均为有效 UTF-8，无 BOM                     |

Core 回归覆盖非末项拒绝、前序已接受状态、原始空基线比较、归属阻塞、无任务或队列副作用、取消删除后重开重放、冻结 blob 损坏、revision 漂移、请求冲突和跨项目拒绝。Desktop 使用真实公开 Scheduler 和测试 RPC 子进程，验证成功输出、查询、重放、严格参数和状态不变。

UI 回归覆盖生产执行窗口入口和服务端渲染、丢失响应精确重试、历史报告展示、重开只查询、关闭后迟到响应、同步关闭阻止派发、旧读取仍在途时立即重开、外来回执和无效最终阶段契约。独立只读复核提出的 busy 卡住疑点未得到源码支持：cancel 已同步清除 busy；新增关闭立即重开回归通过。

核心审阅存储模块 187 effective lines，摘要模块 161，Core 回归 205，UI 控制器 129，UI 回归 199。编译保留既有未读字段和 unused_mut 警告，未修改无关实现。

## 剩余工作

F7.1 仍需最后 fine 和对象整体的人工决定、所需阶段及可配置报告策略、完整固定引用与文件归属处理、未处置反馈接入。当前报告提供冻结清单和阻塞提示；没有提供内容级文本 diff 或反馈操作。

F7.2 仍需持久文件日志、受管路径物化、崩溃恢复，以及原子更新接受版本、任务状态和队列释放的最终数据库事务。F7.3 的真实历史基准发布、漂移冲突和各故障点验证尚未完成。固定引用变化和历史已接受版本漂移的完整发布集成用例留待该工作流实现。

本次未运行真实模型、Godot/Blender、浏览器视觉或完整原生验收；未构建或重启 Beaver.exe，未发布、提交、推送或修改版本。下一切片应在持久审阅来源上接入明确的最终决定和可恢复发布边界，保留所有权及历史查询约束。
