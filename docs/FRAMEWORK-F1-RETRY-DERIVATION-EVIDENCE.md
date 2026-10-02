# F1：同一首个细修的纯 retry 链独立派生

日期：2026-10-02（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[首次停止执行派生](FRAMEWORK-F1-EXECUTION-DERIVATION-EVIDENCE.md)；汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。本次恢复会话 `01a0faf8-2db1-7230-a483-a624c87cd748` 中尚未交付的 retry 切片，完成定向检查与交付收尾。旧批次文档和失败日志不改写。

## 可用流程与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。支持同一个首个 fine 的多次“停止 → 核验 → 显式 retry → 再停止”，不再把所有 recovery / successor 历史一律拒绝。要求 Ready、无 preparation error、generation 1、唯一真实首个 fine，链内所有 attempt 均为 Failed / Interrupted，输出已捕获，冻结内容、工作区与历史证据完整。

**完成条件：完整保留停止链及恢复证据，派生副本默认安全暂停；准备、组装、启用、登记、登记重试、关闭重开、旧请求重放和解除暂停均不自动执行。只有针对当前副本重新核验，再提交新的显式恢复请求，才创建唯一的新 attempt。** 新输入等于 terminal output，不复用旧 thread/turn；旧请求只能重放回执，不重新构造 lease。

使用已有制造计划暂停控制和任务恢复界面：显式解除安全暂停后，通过 `objectTask.verifyRecovery` 重新核验，再通过 `objectTask.resumeRecovery` 恢复。暂停状态下生成的 verifier 在解除暂停后已经过期，复制的历史 verifier 也不能授权新恢复。相同恢复请求重复或并发提交不能重复启动。

## 停止链、回执与身份边界

- 新增支持 `object_recovery_verification`、`object_recovery_head`、`object_recovery_resume`、`object_recovery_resume_head`、`object_recovery_attempt_successor` 五类实体。全实体扫描与生产 storage reader 共同核对业务键、完成状态、nested verifier、head、generation 和 successor；pending、孤立、缺口、重复或损坏的记录在准备前拒绝。
- attempt 链复用生产 reader，按真实 successor 排序，只接受同一 fine 的 plain retry。`advance`、`rework`、`next` 和 `fresh_checks` 均必须为空；不存在重构旧 worker lease 的路径。
- 每份历史 verifier 的 Records 必须精确对应一个链前缀，medium/run/fine 的状态及 revision 必须符合生产推进；queue 的不可变 claim/enqueue 字段精确一致。历史 queue position 保留为证据，不误当目标当前规范化排序。
- control 由不可变暂停命令回执锚定，object 与 accepted pointer 按历史 command revision 核对；允许两次 retry 之间的合法 metadata 变化，不用当前对象内容代替历史快照。baseline sources 必须完整、有序且对应冻结闭包。
- 历史 interrupt 条目逐项精确匹配并保持唯一有序；允许 verifier 生成后再产生 completed interrupt，不错误要求旧快照包含未来回执。
- typed 转换 operation、target、Records、attempt/history、head/edge 与业务键，保留 generation、revision、状态、历史 thread/turn、检查和报告。`resume.started` 保留原始 Running 初始快照，不替换为停止后的 attempt。映射后的 baseline sources、interrupts 和 frozen versions 重新规范排序，并重算身份相关 digest；没有全局字符串替换。
- inventory 校验所有历史 attempt 引用的 blob，只把可见工作区与 terminal output 比较。历史计划投影只还原 planned root，不把每次 retry 当作新的初始计划。再次派生已暂停副本不重复增加 control revision。

## 正式消费者与回归覆盖

Core 新增五项测试，fixture 通过正式 API 构造三个 stopped attempts，包含 paused verifier、取消调度后的 Blocked resume、两次真正 retry、不同线程/轮次、trace、检查、正式 interrupt、合法对象元数据更新，以及 terminal verifier 之后新增的 completed interrupt。

正向覆盖 Empty / Latest / Pinned 三种基准、多对象 reference closure、历史 typed 转换、旧核验与恢复请求 replay 无 lease、检查/trace 可读、安全暂停、解除暂停仍不能 claim、fresh verify/resume 创建第 4 个 attempt，且源与准备副本 inventory 不变。负控覆盖历史前缀和顺序、task/queue/object/control/baseline/interrupt 快照、pending 和孤立回执、错误 head/edge/业务键、非 retry 用途，以及仅由旧 attempt 引用的 blob 丢失。

Desktop 新增 `migration_derivation_retry_history_requires_fresh_explicit_resume_and_launches_once`，使用正式 migration dispatcher、`objectTask.*` API 与活动 Scheduler。计数 factory 返回 `simulated launch failure`，不调用外部 AI、Godot 或 Blender。验证 inspect → prepare → assemble → activate → register、三个 stopped attempts 保留、全部旧 verify/resume replay、登记重试，以及 shutdown/drop/关闭重开后的全新 Scheduler 均为零启动。

复制的旧 terminal verifier、解除暂停前的 verifier 都不能授权新恢复；fresh verify 后两次并发提交同一 resume，只允许同回执或精确 `OBJECT_RECOVERY_WRITER_ACTIVE`，最终仅启动一次。第 4 个 attempt 输入等于旧 terminal output，无旧 thread/turn；后续新旧请求 replay 不再启动。宿主库、其他项目、源项目和准备副本隔离均有断言，其他项目以 offline inventory 比较，避免把 WAL 活动误判为修改。

`src/ui/ProjectDerivationPanel.tsx` 更新支持范围与恢复说明，复用既有正式 migration/recovery UI，不新增恢复旁路。本轮 TypeScript 和 UI Prettier 通过，但没有重跑 UI smoke、真实 WebView 或视觉验收。

## 模块责任与结构结果

最终结构报告为 **1300 个源码、17 个未变动历史文件、0 个违规**。本切片 19 个 Rust 文件及一个 TSX 文件均符合规则；没有修改 baseline 或新增 exception。唯一超过 250 行的本切片模块是 retry 负控测试，283 有效行，保持“损坏历史在派生前拒绝”的单一职责。

| 文件                                                              | 责任                                                  | 有效行 |
| ----------------------------------------------------------------- | ----------------------------------------------------- | -----: |
| `native/core/src/object_recovery_derivation.rs`                   | 停止链、真实首个 fine 与领取基准只读校验              |     99 |
| `native/core/src/object_recovery_derivation_receipts.rs`          | recovery 回执、head、generation 与 successor 闭集校验 |    183 |
| `native/core/src/object_recovery_derivation_snapshot.rs`          | 历史 Records 前缀和不可变证据锚定                     |    149 |
| `native/core/src/object_recovery_derivation_rewrite.rs`           | 回执、请求、head/edge 和业务键 typed 转换             |    111 |
| `native/core/src/object_recovery_derivation_records.rs`           | Records 与嵌套 attempt、历史集合转换                  |     52 |
| `native/core/src/object_recovery_resume.rs`                       | 显式恢复授权及派生子模块接线                          |    172 |
| `native/core/src/project_derivation_execution_inventory.rs`       | 全链 blob 与 terminal 工作区核验                      |     76 |
| `native/core/src/project_derivation_execution_records.rs`         | 执行记录转换与身份相关 digest                         |    145 |
| `native/core/src/project_derivation_execution_validation.rs`      | 执行实体校验、回执接线与 root 计划投影                |    112 |
| `native/core/src/project_derivation_identity_keys.rs`             | recovery 业务键纳入统一身份映射                       |    169 |
| `native/core/src/project_derivation_queue_validation.rs`          | 执行/recovery 支持范围接线与队列校验                  |    141 |
| `native/core/src/project_derivation_retry_fixture.rs`             | 正式 API 的 stopped retry 历史夹具                    |    184 |
| `native/core/src/project_derivation_retry_tests.rs`               | 全链派生、回执重放与 fresh 显式恢复                   |    173 |
| `native/core/src/project_derivation_retry_rejection_tests.rs`     | 历史损坏、孤立记录、非 retry 与缺失内容负控           |    283 |
| `native/core/src/project_derivation_execution_rejection_tests.rs` | 未完成/未支持执行边界和 typed schema 负控             |    227 |
| `native/core/src/project_derivation_execution_tests.rs`           | 首次停止派生回归及 retry 测试接线                     |    236 |
| `native/desktop/src/migration_derivation_execution_tests.rs`      | 首次停止的正式消费者与 retry 测试接线                 |    207 |
| `native/desktop/src/migration_derivation_retry_fixture.rs`        | 正式 recovery API 历史构造与 replay                   |    121 |
| `native/desktop/src/migration_derivation_retry_tests.rs`          | 活动 Scheduler 零自动启动与唯一显式恢复               |    203 |
| `src/ui/ProjectDerivationPanel.tsx`                               | 正式派生入口的支持与恢复说明                          |    159 |

## 本轮实际验证

证据根目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-retry-derivation-20261002-continued`。最终结果保存在 `final-corrected-20261002-041305/results.json`；命令、exit code、非零测试数、耗时和日志路径均已记录。`environment.json` 记录正常 PowerShell 环境，保留现有 `LIB`，TEMP/TMP 指向本批独立 temp 目录。

| 检查                                                                                     | 最终结果                                              | 日志                                                                   |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------- |
| `rtk proxy cargo test --locked -p beaver-core project_derivation`                        | 97 passed，0 failed                                   | `final-corrected-20261002-041305/03-core-derivation.log`               |
| `rtk proxy cargo test --locked -p beaver-core object_recovery`                           | 20 passed，0 failed                                   | `final-corrected-20261002-041305/04-core-recovery.log`                 |
| `rtk proxy cargo test --locked -p beaver-desktop migration_derivation`                   | 8 passed，0 failed                                    | `final-corrected-20261002-041305/05-desktop-derivation.log`            |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::recovery_tests` | 4 passed，0 failed                                    | `final-corrected-20261002-041305/06-desktop-recovery.log`              |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::resume_tests`   | 1 passed，0 failed                                    | `final-corrected-20261002-041305/07-desktop-resume.log`                |
| `rtk npm run typecheck`                                                                  | exit 0                                                | `final-corrected-20261002-041305/08-typecheck.log`                     |
| 19 个 Rust 文件 `rtk proxy rustfmt --check --edition 2021 --config skip_children=true`   | exit 0                                                | `final-corrected-20261002-041305/01-rustfmt.log`                       |
| `rtk proxy npx prettier --check src/ui/ProjectDerivationPanel.tsx`                       | exit 0，检查后文件未变                                | `final-20261002-040932/02-prettier.log`                                |
| `rtk npm run check:effective-lines`                                                      | 1300 sources，17 unchanged legacy files，0 violations | `final-corrected-20261002-041305/09-effective-lines.log` 与同目录 JSON |

这些是本次续接实际运行的结果，包含既有回归，不是完成百分比。Cargo 测试同时编译受影响 Core/Desktop 测试消费者；本轮没有另行运行完整原生构建。上一轮的 `core-retry-03.log`（5 passed）与 `desktop-retry-01.log`（1 passed）保留，但不作为本轮最终批的替代。

保留失败证据并区分测试输入错误与生产缺陷：

1. `core-retry-01.log` 为 3 passed / 2 failed：负控把真实 u64 `queue.enqueuedAt` 改成字符串，被 schema 提前拒绝；改为合法类型的错误数值，继续测试快照比较。
2. `core-retry-02.log` 为 4 passed / 1 failed：metadata 负控实际报 `DERIVATION_OBJECT_RECEIPT_PROJECTION_MISMATCH`，原错误预期不准确；修正后 `core-retry-03.log` 为 5 passed。
3. 本轮 `final-20261002-040932/03-core-derivation.log` 为 96 passed / 1 failed。已支持的 recovery verification/resume 是严格 typed wrapper，顶层 `projectId` 实际报 `unknown field`，不是 `missing field`。仅修正这两处精确错误预期，并将测试名调整为 unfinished/unsupported boundaries；保留源 inventory 不变和目标不创建断言，未修改或放宽生产校验。初批其余功能检查尚未开始；修正后派生和后续定向检查全部通过。

原日志保持原有编码，不改写失败记录；本轮新增文本为 UTF-8 无 BOM。既有 unused/dead-code 警告保留。`source-manifest.json` 与 `delivery-integrity.json` 记录本切片源码/文档 hash、严格编码、本地文档链接、结构报告匹配、1840 文件前置快照比较及测试进程核对；无关 dirty 文件和既有历史文档保留，未重置工作区。文档收尾后只验证文档与完整性，不重复未失效的功能检查。

## 未支持范围与交付边界

本切片已经闭合 **同一首个 fine 的纯 retry 停止链**，但不覆盖仅领取而没有停止 attempt、Running / AwaitingGate、跨 fine 的 advance、rework、并列首个 fine、含 failed / objectHeld 的 reorder 历史、处置/导入/发布、Running AI 规划和其他未知记录。这些仍在准备前整体拒绝。完整对象数据派生、F1.4、F1 及整体开发保持未完成。

未进行真实 HTTP/MCP transport、WebView、UI 视觉、真实 AI/Godot/Blender 或全量原生验收；未构建新 EXE、递增版本、提交或推送。源码版本保持 `0.1.19.44`，既有运行中的 `0.1.19.43` 用户实例未关闭或替换，因此不声称新编译包已经交付。
