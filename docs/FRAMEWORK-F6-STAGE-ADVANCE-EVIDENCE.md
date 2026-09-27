# F5/F6 人工接受与后继细任务推进

日期：2026-09-25。范围：显式接受成功细任务 A，启动同一 medium 的直接后继 B。当前切片完成开发与定向验证；框架总计划尚未完成。

## 可使用的流程

在生产任务的执行记录中，成功尝试停在 AwaitingGate。用户先运行冻结内容技术检查，再执行恢复核验；填写人工验收说明，预览下一细任务的标题、定义 revision、提示词及验收要求，最后明确确认启动。

Core 在项目数据库中保存完整请求、人工说明、原技术报告 ID、下一 fine 的冻结定义及重新检查结果。提交前复核内容完整性、代码结构、工作区、当前记录、暂停状态、依赖与直接后继顺序。成功事务同时接受 A、创建 B 的尝试和历史边；B 从 A 的冻结输出继续，同一 medium 保留对象写入权。对象版本仍未发布。

B 失败或中断后使用既有显式恢复流程重试 B，不重复接受 A。未知响应保留原请求并允许精确重试；重开项目只读取持久状态，不自动启动。取消后继续保留已接受 A 和不可变尝试历史。最后一个 fine 不开放此推进按钮，显示整体验收与发布尚未开放。

## 实现边界

- Core：object_stage_advance.rs 负责人工批准和直接后继资格；object_recovery_resume.rs / object_recovery_resume_store.rs 复用持久恢复日志、锁外重验和事务提交；object_recovery_attempt_history.rs 按持久历史边验证跨 fine 与同 fine 重试。
- Desktop：objectTask.advanceAttempt 使用既有 Scheduler 槽位、项目路由和重放机制；与 objectTask.resumeRecovery 分开校验请求用途。测试通过公开 Scheduler 和实际测试 RPC 子进程生成首次成功输出，没有开放内部 Lease API。
- UI：ObjectStageAdvancePanel、object-stage-advance 控制器及恢复确认区贯通生产执行窗口。关闭窗口使迟到结果失效；同步关闭监听不会继续发请求或刷新，也不会递归通知。恢复请求中的说明保留精确字节，避免读取时 trim 导致请求冲突。
- 存储：继续使用项目本地恢复记录，增加可选 advance/next/freshChecks，普通失败重试仍使用原路径。没有全局迁移、版本变更或发布。

## 本次验证

日志目录：output/framework-stage-advance-20260925/。

| 命令                                                                                                                                                                                                                                    | 实际结果                                   |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| cargo test --locked -p beaver-core --lib object_stage_advance                                                                                                                                                                           | 4 passed                                   |
| cargo test --locked -p beaver-core --lib object_recovery                                                                                                                                                                                | 20 passed                                  |
| cargo test --locked -p beaver-core --lib object_attempt_check                                                                                                                                                                           | 3 passed                                   |
| cargo test --locked -p beaver-desktop object_attempt_runtime                                                                                                                                                                            | 10 passed，包括测试 RPC fixture            |
| npm run typecheck                                                                                                                                                                                                                       | 通过                                       |
| npx tsx --test tests/object-stage-advance.test.ts tests/object-task-resume*.test.ts tests/object-task-recovery*.test.ts tests/object-task-execution*.test.ts tests/object-recovery-contract.test.ts tests/object-attempt-checks.test.ts | 64 passed                                  |
| 定向 rustfmt --edition 2021 --config skip_children=true                                                                                                                                                                                 | 通过                                       |
| 定向 npx prettier --write                                                                                                                                                                                                               | 通过                                       |
| npm run check:effective-lines                                                                                                                                                                                                           | 985 sources，0 violations，未修改 baseline |
| git diff --check                                                                                                                                                                                                                        | 通过                                       |

新回归覆盖 A 接受后 B 失败重试、取消和重开历史；无报告、错误后继、过期 revision、空说明；文件和前后继定义在检查期间变化；pending 批准崩溃、精确重放、提交后崩溃不重启；Desktop 并发同请求只启动一次、异项目拒绝和项目隔离；生产 UI 显式确认、丢响应、重开恢复、错误后继回执、UTF-8 字节限制和同步关闭。

验证期间修复了 UI 可选 position 的类型错误、取消监听递归以及 Desktop 测试越过私有执行 API 的编译失败。最后所列检查均通过。编译保留既有 validation/service.rs 未读取字段及既有测试/game_runtime 的 unused_mut 警告，没有修改无关代码来消除警告。

结构结果：批准模块 170 effective lines，Core 推进测试 235，Desktop 推进测试 210，历史校验 129，恢复事务模块 268，UI 推进控制器 47、面板 88。268 行事务模块保持单一恢复日志与原子提交职责；未新增超限例外。结构 JSON 与各命令日志保存在上述目录。

## 尚未完成

F5.1/F5.4/F6.6 保持未勾选。本次仅关闭人工接受后继流程，未实现 AI 评价、可配置阶段策略、当前阶段重做和下游失效。末项成功后的整体待验收、候选/引用闭包、可恢复文件物化、版本漂移冲突与发布后释放对象权属于 F7，继续未完成。

后续优先沿末项整体验收与 F7 发布推进用户流程，同时保留技术失败不可绕过、模型不能批准自身人工门槛、同对象写入权和历史查询约束。受管交互 Godot/Blender 会话、完整工具回调/取证、测试页关联和其余迁移工作按总计划继续推进。

本次没有运行完整原生验收、真实模型或引擎端到端流程，没有构建或重启 Beaver.exe，没有提交或推送，也没有宣称发布完成。UI 证据来自生产控制器和服务端渲染测试，尚无浏览器视觉验收。
