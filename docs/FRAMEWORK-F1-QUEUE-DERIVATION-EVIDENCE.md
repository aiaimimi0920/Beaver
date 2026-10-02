# F1：从未领取执行的队列计划独立派生

日期：2026-10-02（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[规划完成声明派生](FRAMEWORK-F1-DECLARATION-DERIVATION-EVIDENCE.md)；汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。前批报告及失败日志保持原样。

## 可用范围与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。新增以下七类记录的严格校验和身份转换：`object_task_queue`、`object_task_queue_order_revision`、`object_task_queue_reorder_receipt`、`object_task_dispatch_control`、`object_task_dispatch_receipt`、`object_task_coarse_dispatch_control`、`object_task_coarse_dispatch_receipt`。

支持已入队但从未领取执行的制造计划，保留源队列最终顺序、定义修订、执行前取消、medium/coarse 暂停状态及不可变请求历史。副本中仍为 queued 的 medium 全部默认暂停；源 medium 已暂停时不叠加 control revision。取消项不变回待执行项，coarse control 保留原状态。

打开、关闭重开、登记重试及重放旧排序/暂停/恢复请求，都不能解除副本的新暂停、领取任务或生成执行准备。用户在现有制造计划派发控制中发出新的正式 `objectTask.setPaused` 恢复请求后，生产 `object_run_preparation::claim_next` 才能按正常资格领取；依赖、coarse 暂停和对象队首约束仍生效。本批没有新增 execution gate，也没有改 Scheduler。

已领取或曾领取的队列（owner、claimToken、非零 generation、claimed 状态）、已有 execution preparation/attempt、Running AI 规划、导入/发布及其他未知记录仍不支持派生。源检查和准备在创建准备目录前整体拒绝，不静默丢弃数据；每项负控核对目标不存在且源 inventory 不变。F1.4、F1 和整体开发不因此完成。

## 身份、顺序和历史边界

- 队列项、medium/coarse control 的 project/task/object/run 使用既有独立身份映射；receipt key 复用生产 `SHA256(JSON((projectId, requestId)))` 算法。排序、medium、coarse 请求分别使用 `object_queue_request`、`object_dispatch_request`、`object_coarse_dispatch_request` namespace。
- composition 先复制并转换原历史，再在同一目标事务内规范队列顺序和追加安全暂停回执，最后校验目标。安全暂停复用生产 `object_task_dispatch::set_paused_in`，独立请求 namespace 为 `object_derivation_safety_pause`，不改写或删除旧 receipt。失败不发布部分目标，也不写源。
- 生产入队允许相同 position，最终排序为 `(position, task_id)`。新 task ID 可能反转原 tie-breaker；因此按源 `queue::list_in` 的完整顺序给目标写连续 position，保留语义顺序而非原数字 position。回归主动选择会反转映射 ID 字典序的 provenance，证明队首不靠偶然保持。
- 旧 reorder receipt 缺少完整历史 hashing 输入，不能声称重新计算历史 snapshot fingerprint。历史 version 使用 provenance-scoped CAS alias：`SHA256(JSON(("beaver-derived-queue-history-token-v1", sourceProjectId, targetProjectId, derivationRequestId, originalToken)))`。相同原 token 得到相同 alias，保留相邻 receipt 的链式相等关系；当前 `queueView.version` 仍使用正常生产算法。
- 历史重放返回转换后的不可变 receipt，不重做排序或写回旧控制状态；同 request ID 更改输入仍触发 `REQUEST_CONFLICT`。旧 receipt 中的标题、状态和 blockers 保留其历史语义，不拿当前定义/暂停状态伪造历史快照。
- dispatch control revision 必须连续、无重复，与最终 head/paused 一致；历史 expectedTaskRevision 不倒退，允许连续控制命令使用相同 task revision。上限使用生产 `9_007_199_254_740_991`，不误用 task 的 `i64::MAX`。
- reorder receipt 严格检查 schema、身份、token、项唯一性、取消状态、anchors、blockers 的允许值/唯一性/生产顺序和每对象 head 关系；当前队列另经正式 `queueView` 消费者校验。缺失、孤立或损坏的控制/排序历史不放行。

## 实际消费者与回归

Core 覆盖准备、组装、启用、登记、历史 replay/conflict、关闭重开、新请求恢复以及再次派生；直接 claim 和 preparation claim 均在默认暂停时返回 None，各 run 无 preparation。新的恢复请求将示例 control revision 从 3 推进到 4，随后真实 production preparation claim 领取预期 build，generation 为 1；源及 preparation inventory 保持不变。再次派生已暂停副本不重复增加暂停 revision。

Desktop 使用正式 migration dispatcher 与 `data_dispatch::call_object_tasks_for_test`，覆盖 snapshot/queueView、精确映射 receipt 读取、三个正式命令的历史重放和冲突、登记重试、关闭重开、显式恢复后的 production preparation claim，以及项目库/宿主库归属。既有 legacy running/queued/waitingChildren 夹具保留。UI 只更新支持范围和默认暂停说明；现有 `object-task-dispatch.ts` 消费 `objectTask.setPaused`，生成新 request ID 并携带当前 task/control revision。本批未重跑 UI 交互或真实 WebView，不把 API 集成等同视觉验收。

## 模块责任与有效行数

取自本批 `effective-code-lines.json`，非物理行估算。相关 22 个 Rust 文件及一个 UI 文件均无结构违规；两份既有负控模块分别为 293/343 有效行，保持单一负控责任，没有新增 exception 或修改 baseline。

| 文件                                                             | 责任                                          | 有效行 |
| ---------------------------------------------------------------- | --------------------------------------------- | -----: |
| `native/core/src/project_derivation_queue_records.rs`            | 七类 typed 身份转换、顺序规范及事务内安全暂停 |    196 |
| `native/core/src/project_derivation_queue_validation.rs`         | never-claimed 队列与排序历史校验              |    137 |
| `native/core/src/project_derivation_dispatch_validation.rs`      | medium/coarse control 与完整命令链校验        |    178 |
| `native/core/src/object_task_dispatch.rs`                        | 正常运行命令与共享 transaction-local 暂停     |    180 |
| `native/core/src/project_derivation_database.rs`                 | 单一目标事务编排与最终校验                    |    204 |
| `native/core/src/project_derivation_queue_fixture.rs`            | 正式计划、排队、排序及控制历史夹具            |    121 |
| `native/core/src/project_derivation_queue_tests.rs`              | 历史重放、重开、显式恢复及再次派生            |    192 |
| `native/core/src/project_derivation_queue_order_tests.rs`        | tied position 与历史 CAS 链负偶然回归         |    124 |
| `native/core/src/project_derivation_queue_rejection_tests.rs`    | schema、所有权、状态与排序历史负控            |    221 |
| `native/core/src/project_derivation_dispatch_rejection_tests.rs` | 控制链缺口、重复、head、孤立及倒退负控        |    142 |
| `native/desktop/src/migration_derivation_queue_fixture.rs`       | 正式 API 历史夹具                             |     98 |
| `native/desktop/src/migration_derivation_queue_tests.rs`         | 迁移/任务正式消费者与恢复边界                 |    152 |
| `src/ui/ProjectDerivationPanel.tsx`                              | 正式派生入口支持范围说明                      |    160 |

## 实际验证与失败记录

证据目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-queue-derivation-20261001-234701`。`checks.json` 保存原批结果，`checks-continuation.json` 保存 Desktop 修复后的续跑结果；`changed-rust-final.json` 保存 22 个定向格式检查输入。沿用同一正常 PowerShell/MSVC 环境并保留 `LIB`，TEMP/TMP 指向本证据目录的 `temp`。

| 检查                                                                                      | 结果                                                  | 日志                                               |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------- |
| `rtk proxy cargo test --locked -p beaver-core project_derivation -- --nocapture`          | 84 passed，0 failed                                   | `core-derivation-complete.log`                     |
| `rtk proxy cargo test --locked -p beaver-core object_task_queue -- --nocapture`           | 18 passed，0 failed                                   | `core-queue-dependencies.log`                      |
| `rtk proxy cargo test --locked -p beaver-core queue_reorder -- --nocapture`               | 6 passed，0 failed                                    | `core-reorder-dependencies.log`                    |
| `rtk proxy cargo test --locked -p beaver-core object_task_dispatch -- --nocapture`        | 10 passed，0 failed                                   | `core-medium-dispatch-dependencies.log`            |
| `rtk proxy cargo test --locked -p beaver-core object_task_coarse_dispatch -- --nocapture` | 5 passed，0 failed                                    | `core-coarse-dispatch-dependencies.log`            |
| `rtk proxy cargo test --locked -p beaver-desktop migration_ -- --nocapture`               | 13 passed，0 failed                                   | `desktop-migration-fixed.log`                      |
| `rtk proxy npm run typecheck`                                                             | 通过                                                  | `typescript.log`                                   |
| 相关 22 个 Rust 文件定向 `rustfmt --check --config skip_children=true`                    | 通过                                                  | `rust-format-check.log`                            |
| `rtk proxy npx prettier --check src/ui/ProjectDerivationPanel.tsx`                        | 通过                                                  | `ui-prettier-check.log`                            |
| `rtk proxy npm run check:effective-lines`                                                 | 1281 sources、17 unchanged legacy files、0 violations | `effective-lines.log`、`effective-code-lines.json` |

各 filter 确认实际运行了非零测试。测试数量包括既有回归，不是完成百分比；本轮新增 Core 队列测试 9 项、Desktop 消费者集成 1 项。Core 已通过结果没有因仅调整 Desktop 测试方法可见性而重复运行。

保留三次开发失败的原始证据，不放宽断言求绿：

1. `core-queue-initial.log`：8 passed、1 failed。缺字段负控误写 receipt 顶层 `projectId`，先触发 unknown field；改为 `{}`，继续保留 `missing field` 断言。
2. `core-derivation-final.log`：83 passed、1 failed。旧规划负控硬改按随机对象 ID 排序的 `/context/objects/0/id`，偶然选中无冻结版本对象，实际报 `DERIVATION_OBJECT_RECEIPT_OWNER_MISSING`；现按真实 parent ID 精确定位，继续保留 `IMPORT_VERSION_IDENTITY_MISMATCH` 断言，未改生产校验。
3. `desktop-migration-complete.log`：编译报 `error[E0624]: method task_api is private`。共享测试方法改为 `pub(super)`，不扩大生产 API；随后 Desktop 13 项通过。

仓库已有 unused mut/dead fields 警告保留；PowerShell 的 native stderr 包装不是失败依据，结论来自实际 exit code 与 `test result`。文档格式、本地链接、严格 UTF-8/BOM、source manifest、限定 diff 与本次 owned 测试进程核对保存于 `source-manifest.json` 和 `delivery-integrity.json`。既有用户 Beaver 和其他项目 Godot 进程不当作本轮泄漏进程清理。

## 尚未完成

下一实质边界是已领取任务及执行准备/attempt 的独立派生，需要一并定义旧执行所有权的终止、历史证据保留和副本显式继续语义；Running AI 规划与导入/发布操作也未支持。不可用更多 eligibility helper 或重复测试替代这些工作流闭合。

本批未重跑 UI smoke、前端构建、HTTP/MCP 传输、原生 WebView、真实工具绑定或全流程验收；未构建新 EXE、递增版本、提交或推送。版本仍为 `0.1.19.44`，新编译包尚未交付；F1.4、F1 及整体任务保持未完成。
