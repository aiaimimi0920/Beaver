# F1：首次停止执行历史的独立派生与显式恢复

日期：2026-10-02（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[从未领取执行的队列派生](FRAMEWORK-F1-QUEUE-DERIVATION-EVIDENCE.md)；汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。既有批次的报告、失败日志和其他工作区改动保持原样。

## 可用范围与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。新增 `object_run_preparation`、`object_attempt`、`object_attempt_trace`、`object_attempt_check_report`、`object_attempt_interrupt_receipt` 五类执行记录的严格校验与身份转换。

本切片支持 Ready、无 preparation error、generation 为 1 的执行准备，以及其唯一真实首个 fine 的首次 Failed / Interrupted attempt；输出已捕获，冻结内容、工作区和历史证据完整，且没有 recovery / successor 历史。frozen medium/run 保持领取时的 queued 状态，fine 保持开始前的 planned 状态。当前终态与 revision 不重置，历史 thread/turn 保留为证据。

**完成条件：派生副本默认安全暂停；准备、组装、启用、登记、关闭重开、登记重试及旧请求重放均不能自动执行。解除暂停也不是恢复执行。只有新的 `objectTask.verifyRecovery` 核验当前状态，再显式 `objectTask.resumeRecovery`，才能创建唯一的新 lease / attempt。** 新尝试使用旧输出作为输入，thread/turn 为空，不复用旧线程自动续跑；相同恢复请求重试只重放回执，不再启动。

仅领取但没有停止 attempt、Running / AwaitingGate、已恢复的多次 attempt / retry / successor / advance、并列首个 fine、含 failed / objectHeld 的 reorder 历史、处置/导入/发布、Running AI 规划及其他未知记录仍不支持。源检查与准备在目标目录创建前整体拒绝，不静默丢弃数据。F1.4、F1 及整体开发保持未完成。

## 身份、冻结内容与暂停边界

- 统一映射 project/task/object/run/attempt/check/interrupt/owner/claim 身份及对应记录键；owner/claim 仅作为历史身份转换，不构造旧 lease。generation、revisions、内容 hash、相对内容路径、时间、检查结果和历史 thread/turn 不被伪造为新的运行记录。run 工作区目录及其引用按新身份同步转换。
- 全实体扫描 preparation、attempt、check、interrupt、trace，包括未被 head 引用的记录；拒绝孤立证据和不完整链。检查报告复用生产 `verify_report`，trace 复用生产 `validate_trace`；interrupt result 必须等于真实生产终态 view。
- 映射 frozen version 身份后，重新按 `(object_id, version_id)` canonical sort，再复用生产算法计算 content digest；check 的 attempt digest 按转换后的完整 attempt 重算并重新验证。没有放宽 recovery 的冻结内容比较。
- claim-point acceptance 从 committed receipt history 的 revision 恢复，不拿当前 accepted version 代替领取时基准。停止后的合法 metadata/capture/acceptance 变化可派生，但损坏的领取基准和内容仍拒绝。
- 生产 `start()` 与派生校验共享 `select_fine`，要求真实首个 fine；同首位置并列时拒绝，不通过改 position 或依赖映射后 ID 字典序改变历史选择。
- source-lock inventory 检查 blob 存在性及摘要、捕获输出与可见工作区文件一致性、大小写路径 alias，以及未知或歧义 workspace namespace。沿用生产 checkpoint 的可见文件排除规则，不重新解释不透明用户文件。
- 组装目标事务在保留转换后的历史后，为 queued / failed medium 复用生产暂停业务追加独立安全暂停回执。已暂停不叠加 control revision，不覆盖旧 receipt；旧 `objectTask.setPaused` 暂停/解除暂停回执只重放历史，不解除副本新暂停。

## 实际消费者与回归

Core 的闭环回归覆盖 Failed + empty、Failed + latest accepted、Interrupted + pinned 三种生产基准，准备、组装、启用、读取、历史检查/中断/控制回执 replay、关闭重开、stale verification 拒绝、解除暂停仍不能 claim、fresh verify 后显式恢复及幂等重试。新 lease 的 attempt ID 不同，thread/turn 为空，输入等于旧输出，旧 attempt 保留；源和 preparation inventory 不变。另有停止后合法对象历史变化正控，以及未完成/已恢复边界、孤立/损坏证据、冻结基准、工作区和 blob 负控。

Desktop 新增 `migration_derivation_execution_active_consumer_never_launches_without_fresh_explicit_resume`，通过正式 migration dispatcher、对象任务 API 和真实活动 `Scheduler::start_with_objects` 验证。计数 factory 返回 `simulated launch failure`，不调用外部 AI、Godot 或 Blender。测试包括登记重试、旧控制/检查请求重放、shutdown/drop/关闭重开后全新的活动 Scheduler、解除暂停仍零启动、stale 核验不授权，以及 fresh 核验后的同请求并发恢复只创建一个新 attempt。并发第二响应只允许同一回执或精确 `OBJECT_RECOVERY_WRITER_ACTIVE`，不吞掉任意错误；之后重放不再启动。宿主库和其他项目无执行/恢复记录，源、preparation 和其他项目 offline inventory 均不变。

`src/ui/ProjectDerivationPanel.tsx` 更新支持范围及“任务恢复界面重新核验 → 显式恢复”的说明，复用既有正式 recovery UI 和 API，不新增另一套恢复入口。相关 TS 消费者回归通过；本批未进行真实 WebView、UI 视觉或 HTTP/MCP transport 验收，不把 API 集成等同视觉交付。

## 模块责任与有效行数

取自本批最终 `effective-code-lines.json`，不是物理行估算。34 个相关 Rust 文件及一个 UI 文件没有结构违规；既有 plan 负控模块为 299 有效行，保持单一负控责任。没有新增 exception 或更新 baseline。

| 文件                                                              | 责任                                             | 有效行 |
| ----------------------------------------------------------------- | ------------------------------------------------ | -----: |
| `native/core/src/object_recovery_derivation.rs`                   | 首次停止边界、领取基准及真实首个 fine 的只读校验 |     87 |
| `native/core/src/project_derivation_execution_validation.rs`      | 全执行实体及孤立证据校验、历史计划投影           |    103 |
| `native/core/src/project_derivation_execution_inventory.rs`       | source-lock inventory、blob 与捕获工作区一致性   |     67 |
| `native/core/src/project_derivation_execution_records.rs`         | typed 执行身份转换与身份相关 digest 重算         |    131 |
| `native/core/src/project_derivation_execution_fixture.rs`         | 正式领取、停止和历史证据夹具                     |    204 |
| `native/core/src/project_derivation_execution_tests.rs`           | 派生、历史重放、重开及显式恢复闭环               |    234 |
| `native/core/src/project_derivation_execution_rejection_tests.rs` | 未完成/恢复边界、孤立证据与冻结基准负控          |    217 |
| `native/core/src/project_derivation_execution_inventory_tests.rs` | workspace namespace、输出和 blob 负控            |     91 |
| `native/core/src/object_acceptance_history.rs`                    | 从 committed receipt revision 恢复历史接受状态   |    139 |
| `native/core/src/object_attempt_store.rs`                         | 生产 attempt 存储及共享 fine 选择                |    180 |
| `native/core/src/object_recovery_files.rs`                        | 冻结输入与工作区恢复校验                         |    178 |
| `native/core/src/project_derivation_copy.rs`                      | 离线源检查、准备与 inventory 校验编排            |    188 |
| `native/core/src/project_derivation_paths.rs`                     | namespace 与工作区路径归属                       |    103 |
| `native/core/src/project_derivation_queue_records.rs`             | 队列身份转换与事务内安全暂停                     |    198 |
| `native/core/src/project_derivation_copy_tests.rs`                | 离线检查及 typed execution schema 负控           |    227 |
| `native/core/src/project_derivation_files_tests.rs`               | 文件所有权、alias 和目标冲突负控                 |    203 |
| `native/core/src/project_derivation_object_rejection_tests.rs`    | 未支持记录与对象历史负控                         |    140 |
| `native/core/src/project_derivation_plan_rejection_tests.rs`      | 计划身份、引用、历史及未支持记录负控             |    299 |
| `native/desktop/src/migration_derivation_execution_fixture.rs`    | 活动 Scheduler 与正式恢复 API 夹具               |    152 |
| `native/desktop/src/migration_derivation_execution_tests.rs`      | 零自动启动、唯一显式恢复和跨库隔离               |    205 |
| `src/ui/ProjectDerivationPanel.tsx`                               | 正式派生入口支持范围及恢复要求                   |    159 |

## 实际验证与失败记录

证据目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-execution-derivation-20261002-012700`。`focused-final-results.json` 保留原批所有 exit code；`focused-corrections-results.json` 保存修正旧负控后的派生、34 文件 Rustfmt 和结构检查结果，`changed-rust.json` 保存格式检查输入。运行使用同一正常 PowerShell/MSVC 环境，保留 `LIB`，TEMP/TMP 指向本证据目录的 `tmp`。

| 检查                                                                                     | 最终结果                                              | 日志                                                         |
| ---------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------ |
| `rtk proxy cargo test --locked -p beaver-core project_derivation`                        | 92 passed，0 failed                                   | `corrected-core-project-derivation.log`                      |
| `rtk proxy cargo test --locked -p beaver-core object_attempt_store`                      | 5 passed，0 failed                                    | `final-core-object_attempt_store.log`                        |
| `rtk proxy cargo test --locked -p beaver-core object_attempt_check`                      | 3 passed，0 failed                                    | `final-core-object_attempt_check.log`                        |
| `rtk proxy cargo test --locked -p beaver-core object_attempt_trace`                      | 2 passed，0 failed                                    | `final-core-object_attempt_trace.log`                        |
| `rtk proxy cargo test --locked -p beaver-core object_recovery`                           | 20 passed，0 failed                                   | `final-core-object_recovery.log`                             |
| `rtk proxy cargo test --locked -p beaver-core object_run_baseline`                       | 5 passed，0 failed                                    | `final-core-object_run_baseline.log`                         |
| `rtk proxy cargo test --locked -p beaver-core object_version_acceptance`                 | 4 passed，0 failed                                    | `final-core-object_version_acceptance.log`                   |
| `rtk proxy cargo test --locked -p beaver-core object_task_queue`                         | 18 passed，0 failed                                   | `final-core-object_task_queue.log`                           |
| `rtk proxy cargo test --locked -p beaver-core object_task_dispatch`                      | 10 passed，0 failed                                   | `final-core-object_task_dispatch.log`                        |
| `rtk proxy cargo test --locked -p beaver-desktop migration_`                             | 14 passed，0 failed                                   | `final-desktop-migration.log`                                |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::recovery_tests` | 4 passed，0 failed                                    | `final-desktop-recovery_tests.log`                           |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::resume_tests`   | 1 passed，0 failed                                    | `final-desktop-resume_tests.log`                             |
| `rtk proxy npx tsx --test` 七个恢复消费者测试文件                                        | 29 tests，29 pass，0 fail                             | `final-typescript-consumers.log`                             |
| `rtk proxy npm run typecheck`                                                            | exit 0                                                | `final-typecheck.log`                                        |
| 34 个文件 `rtk proxy rustfmt --check --edition 2021 --config skip_children=true`         | exit 0                                                | `corrected-rustfmt.log`                                      |
| `rtk proxy npx prettier --check src/ui/ProjectDerivationPanel.tsx`                       | exit 0                                                | `final-prettier-ui.log`                                      |
| `rtk proxy npm run check:effective-lines`                                                | 1291 sources、17 unchanged legacy files、0 violations | `corrected-effective-lines.log`、`effective-code-lines.json` |

七个 TS 输入为 `object-recovery-contract`、`object-task-recovery-lifecycle`、`object-task-recovery-query`、`object-task-recovery-retry`、`object-task-recovery-ui`、`object-task-resume-execution`、`object-task-resume`，均位于 `tests/*.test.ts`；完整命令保存于结果 JSON。各 Cargo filter 已确认运行非零测试，测试数量包含既有回归，不是完成百分比。Core 直接依赖合计 67 项。仅修正四份 test-only 负控后，未重复不受影响的 Desktop、runtime、TS 或类型检查。

保留开发失败历史，不通过放宽断言或校验求绿：

1. `core-first.log`：`error[E0277]: the trait bound u64: ToSql is not satisfied`，修正 SQLite 测试参数类型。`core-first-retry.log` 为 0 passed / 8 failed，暴露夹具选择及 frozen metadata 前提问题，按正式对象/计划前提修正。
2. `core-fixture-inventory-fix.log` 为 4 passed / 4 failed，包括负控错误优先级和 `OBJECT_RECOVERY_BASELINE_CHANGED`。修正精确错误预期及身份映射后的 frozen versions canonical sort；生产 recovery 内容比较未放宽。随后 `core-closure-fixture-fix.log` 为 8 passed / 0 failed。
3. `desktop-first.log` 的 Null vs `failed` 来自测试读取错误的 attempts API shape；按真实嵌套 attempt 读取。`desktop-api-shape-fix.log` 暴露活动 SQLite inventory 的 `os error 33`，改为关闭其他项目 Runtime 后比较 offline inventory，再重新打开。`desktop-offline-inventory-fix.log` 为 1 passed / 0 failed；没有吞掉锁错误或跳过隔离断言。
4. `final-core-project-derivation.log` 保留原批 88 passed / 4 failed：`object_attempt` 已是 typed supported schema，不能再当作 unknown kind；顶层 `projectId` 应为 unknown field，而 `{}` 应为 missing field。copy 测试分开验证两者，object unknown-kind 改用真实未支持的 `object_publication`，plan malformed payload 改用 `{}` 并保留 `future_workflow` unknown-kind 负控。孤立工作区已在 inventory 阶段提前拒绝，files 测试改为 inspect/prepare 均验证 `ambiguous or unknown workspace identity`。全部保留目标不存在与源 inventory 不变断言；未改生产逻辑。定向重跑最终 92 passed / 0 failed。

前两份 Core 日志为原始 UTF-16LE，其他新增文本使用 UTF-8 无 BOM，不改写旧日志掩盖失败。仓库已有 unused mut/dead fields 警告保持原样；PowerShell 的 `RemoteException` native stderr 包装不是失败依据，使用 exit code 与 `test result`。文档格式、本地链接、严格 UTF-8/BOM、与最终结构报告匹配的 source hash、限定 diff 和本次 owned 测试进程核对保存于 `source-manifest.json`、`scoped.diff`、`delivery-integrity.json`；不终止已有用户 Beaver 或其他应用。

## 尚未完成

本切片闭合首次停止执行的安全派生与显式恢复，不覆盖仅领取/运行中/待验收、多次恢复及后继尝试、并列首个 fine、failed / objectHeld 排序历史、处置/导入/发布或 Running AI 规划。完整对象数据派生、F1.4 和 F1 继续保持未完成。

本批未重跑 UI smoke、前端构建、HTTP/MCP 传输、原生 WebView、真实工具绑定或全流程验收；未构建新 EXE、递增版本、提交或推送。版本仍为 `0.1.19.44`，新编译包尚未交付。文档收尾后仅做文档与交付完整性验证，不重复已经通过且未失效的功能检查。
