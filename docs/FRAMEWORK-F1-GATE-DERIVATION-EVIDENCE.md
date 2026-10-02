# F1：首个细修待验收的安全派生

日期：2026-10-02（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[纯 retry 链派生](FRAMEWORK-F1-RETRY-DERIVATION-EVIDENCE.md)；汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。本次继续会话 `01a0faf8-2db1-7230-a483-a624c87cd748` 的下一可用切片，不改写前批结果或失败日志。

## 可用流程与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。源项目必须离线；Ready preparation、generation 1、唯一真实首个 fine 等原约束不变。首次成功或经历同 fine Failed / Interrupted retry 后，最后一个 attempt 可为 AwaitingGate；候选输出、冻结输入、可见工作区和历史证据必须完整。此前 attempt 不能为 AwaitingGate，源中不能已有候选审阅、跨 fine advance、rework、处置或发布历史。

**完成条件：保留候选输出与完整历史，副本默认安全暂停；准备、组装、启用、登记、登记重试、关闭重开、旧请求重放及解除暂停均不执行。解除暂停后重新核验，再通过既有生产入口显式创建唯一新 attempt。** 复制历史不等于新的执行授权，旧 lease 不重构，新 attempt 不复用旧 thread/turn。

两条正式消费路径：

1. **多 fine 的阶段推进**：在制造计划中明确解除安全暂停，通过 `objectTask.verifyRecovery` 重新核验和 `objectTask.checkAttempt` 检查候选，再通过 `objectTask.advanceAttempt` 明确接受当前 fine 并启动紧邻的下一 fine。新输入等于 terminal output，原 fine 变为 accepted。
2. **单 fine 的最终候选返工**：解除暂停、重新核验并检查，通过 `objectTask.prepareCandidateReview` 新建最终候选审阅，填写反馈后调用 `objectTask.reworkCandidate`。审阅本身不改任务状态或启动工作，显式返工才创建同 fine 的唯一新 attempt。

候选审阅只适用于最终 fine，不能拿非最终 fine 的候选审阅代替阶段推进。普通 `objectTask.resumeRecovery` 不能把 AwaitingGate 当作 Failed / Interrupted 重试；Core 精确验证 `OBJECT_RECOVERY_RESUME_TERMINAL_REQUIRED`。`canDispose` 在 fresh verify 后为 true，`canResume` 仍为 false，不新增执行旁路。

## 生产责任与状态边界

生产改动限制在四个既有 Core 模块，Desktop advance/rework/review API 及执行器无需新增路径：

| 文件                                                     | 责任                                                                                       | 有效行 |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------ | -----: |
| `native/core/src/object_recovery_derivation.rs`          | 完整同 fine 链校验，仅 terminal 可为 AwaitingGate，仍要求全部输出与相同 preparation        |    100 |
| `native/core/src/object_recovery_derivation_snapshot.rs` | 按历史 attempt 分别重建 medium/run/fine 与 queue 状态，锚定历史前缀、revision 和不可变命令 |    158 |
| `native/core/src/project_derivation_queue_validation.rs` | awaitingAcceptance 必须通过 stopped-owner 全链校验，检查追加安全暂停的 revision 空间       |    143 |
| `native/core/src/project_derivation_queue_records.rs`    | 目标事务向 awaitingAcceptance 追加安全暂停，已暂停不重复增加 revision                      |    201 |

历史 Failed / Interrupted 快照使用 failed，terminal AwaitingGate 使用 awaitingAcceptance，不能拿当前 queue 状态验证早期失败记录。claim/enqueue 字段、历史顺序、完成回执、对象与 control 锚点、blob 和可见 workspace 校验均保留。源恢复回执仍要求 `advance`、`rework`、`next`、`fresh_checks` 为空；本次是在派生副本中创建新的显式动作，不支持复制已有跨阶段/返工历史。

## 正式消费者与回归覆盖

Core 新增五项 gate 测试，覆盖 Empty / Latest / Pinned 三种基准、三次 attempt 历史转换、旧 verification/resume/check replay 无 lease、安全暂停、解除暂停不 claim、fresh verify、普通 retry 拒绝、显式 advance 输入和唯一后继、重开幂等，以及首次即 AwaitingGate 与再次派生不叠加暂停。负控逐一篡改历史或 terminal 的 medium/run/fine/queue 状态，并检查缺失输出、可见工作区漂移、候选 blob 丢失和非末尾 gate；inspect/prepare 拒绝，源 inventory 不变且目标不创建。

Desktop 新增两条正式 API 测试：

- `migration_derivation_gate_advances_only_after_explicit_new_approval`
- `migration_derivation_gate_final_candidate_rework_launches_once_with_feedback`

两者通过 migration dispatcher 完成 inspect → prepare → assemble → activate → register，再使用真实 `objectTask.*` dispatcher 与活动 Scheduler。源包含一次停止和一次成功 retry；副本完整保留两次 attempt，旧请求、登记重试及关闭重开后的新 Scheduler 均为零启动。解除暂停、fresh verify/check 和候选审阅仍不启动；相同显式动作并发只允许同一回执或精确 `OBJECT_RECOVERY_WRITER_ACTIVE`，最终只启动一次。随后重放、再次关闭重开也不重复执行。

Scheduler factory 启动当前 Rust 测试 EXE 的 `object_attempt_runtime::advance_tests::successful_rpc` 子测试，走实际 RPC/产物捕获链，不调用外部 AI、Godot 或 Blender。新 attempt 输入与 terminal output 相等，thread/turn 为空。单 fine 返工反馈 `Reduce movement speed` 不止停留在请求中：新产物 `result.txt` 实测为 `feedback received: Reduce movement speed`。宿主库、其他项目、源与 preparation inventory 隔离均有断言；其他项目关闭后比较 offline inventory，避免误判 WAL 活动。

`src/ui/ProjectDerivationPanel.tsx` 更新支持范围、安全暂停和两条后续流程说明，复用现有生产 UI 控制流。TypeScript 和阶段推进/候选审阅消费者测试通过；本次没有真实 WebView、UI smoke 或视觉验收。

## 模块结构

最终结构检查为 **1304 个源码、17 个未变动历史文件、0 个违规**。本轮 14 个 Rust 文件和 1 个 TSX 文件均不超过 250 有效行，没有更新 baseline 或新增 exception。除上述四个生产模块外：

| 文件                                                              | 责任                                    | 有效行 |
| ----------------------------------------------------------------- | --------------------------------------- | -----: |
| `native/core/src/project_derivation_execution_fixture.rs`         | 首次停止/待验收的正式构造夹具           |    200 |
| `native/core/src/project_derivation_retry_fixture.rs`             | 保留 retry 前驱并可选择 terminal 状态   |    182 |
| `native/core/src/project_derivation_execution_rejection_tests.rs` | 保留未完成/未支持边界负控               |    217 |
| `native/core/src/project_derivation_retry_tests.rs`               | 既有 retry 回归及 gate 测试接线         |    175 |
| `native/core/src/project_derivation_gate_tests.rs`                | gate 派生、安全暂停与唯一显式推进       |    192 |
| `native/core/src/project_derivation_gate_rejection_tests.rs`      | 历史状态、输出、工作区、blob 与前驱负控 |     63 |
| `native/desktop/src/migration_derivation_execution_fixture.rs`    | 可选后续 fine，保留旧夹具行为           |    163 |
| `native/desktop/src/migration_derivation_retry_tests.rs`          | 既有 retry 回归及 gate 测试接线         |    205 |
| `native/desktop/src/migration_derivation_gate_fixture.rs`         | 真实测试 RPC、候选历史及显式动作准备    |    223 |
| `native/desktop/src/migration_derivation_gate_tests.rs`           | 正式派生到推进/返工、唯一启动与隔离     |    199 |
| `src/ui/ProjectDerivationPanel.tsx`                               | 正式派生入口的支持与后续操作说明        |    165 |

## 本轮实际验证

证据根目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-awaiting-gate-derivation-20261002`。`focused-results.json` 保存后续定向批的命令、exit code、耗时和日志；`rustfmt-final-result.json` 单独记录修正日志捕获后的格式检查。Cargo 使用正常 PowerShell 工具链环境，保留现有 `LIB`；测试命令把 TEMP/TMP 指向本批独立 `temp`，`environment.json` 保留启动前环境。

| 检查                                                                                                                 | 实际结果                                                                 | 日志                                                   |
| -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------ |
| `rtk proxy cargo test --locked -p beaver-core project_derivation`                                                    | 首轮 101 passed / 1 failed；失败项修正后单独复测，不声称整组重跑 102/102 | `core-derivation.log`、`core-derivation-failed-01.log` |
| `rtk proxy cargo test --locked -p beaver-core project_derivation_gate_anchors_each_historical_status_to_its_attempt` | 1 passed / 0 failed，含新增未消费 verifier 四类状态负控                  | `core-gate-negative-final.log`                         |
| `rtk proxy cargo test --locked -p beaver-core object_recovery`                                                       | 20 passed / 0 failed                                                     | `core-recovery.log`                                    |
| `rtk proxy cargo test --locked -p beaver-desktop migration_derivation`                                               | 10 passed / 0 failed                                                     | `desktop-derivation.log`                               |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::advance_tests`                              | 2 passed / 0 failed                                                      | `desktop-advance.log`                                  |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::candidate_tests`                            | 1 passed / 0 failed                                                      | `desktop-candidate.log`                                |
| `rtk proxy npx tsx --test tests/object-stage-advance.test.ts tests/object-candidate-review.test.ts`                  | 9 passed / 0 failed                                                      | `ui-consumers.log`                                     |
| `rtk npm run typecheck`                                                                                              | exit 0                                                                   | `typecheck.log`                                        |
| 14 个 Rust 文件 `rtk proxy rustfmt --check --edition 2021 --config skip_children=true`                               | exit 0；仅补跑空输出日志捕获                                             | `rustfmt-final.log`                                    |
| `rtk proxy npx prettier --check src/ui/ProjectDerivationPanel.tsx`                                                   | exit 0                                                                   | `prettier-ui.log`                                      |
| `rtk npm run check:effective-lines`                                                                                  | 1304 sources、17 unchanged legacy files、0 violations                    | `effective-lines.log`、`effective-code-lines.json`     |

数量包含既有回归，不是项目完成百分比。Rust 测试同时编译受影响的 Core/Desktop 消费者；不等同于完整原生构建或实际安装包验收。通过后仅验证文档和交付完整性，不重复未失效的功能检查。

保留失败和诊断证据：

1. `core-derivation-failed-01.log`：已被 retry 消费的 `retry-verify-0` 的 fine 状态被篡改后，生产 reader 先将其与不可变 `saved.started.fine` 比较，实际拒绝为 `OBJECT_RECOVERY_RESUME_RECEIPT_MISMATCH`，不是后续 task snapshot 错误。仅修正这一个精确预期，并增加未消费 `paused-verify` 的四类状态负控，确保真正覆盖 `DERIVATION_RECOVERY_TASK_SNAPSHOT_MISMATCH` / `DERIVATION_RECOVERY_QUEUE_SNAPSHOT_MISMATCH`。未改成泛 `is_err()`，未放宽生产条件。
2. `desktop-derivation-compile-failed-01.log`：测试夹具 `let store = runtime.store().lock().unwrap();` 触发 `E0716: temporary value dropped while borrowed`。增加 store handle 局部绑定保持其生命期，随后 Desktop 派生 10 项通过。
3. `run-focused-checks.ps1` 的 Rustfmt 检查返回 0 且无输出，日志 writer 却将 null 传入 `WriteAllLines`。原批结果保留，已单独复跑格式检查并安全记录命令、文件数和 exit code，没有因此重跑功能测试。另一次最早 PowerShell 调用把 Cargo stderr 的 `Compiling` 当成终止异常，不作为编译失败结论。

既有 unused/dead-code 警告保留。`source-manifest.json` 和 `delivery-integrity.json` 记录源码/文档 SHA-256、严格 UTF-8 无 BOM、本地链接、结构报告一致性、1850 文件前置快照比较及进程核对；无关 dirty 文件及旧 retry 证据保持原样。本轮未启动 Godot/Blender，没有按应用名称清理用户进程。

## 未支持范围与交付边界

本切片闭合 **首个 fine 的 terminal AwaitingGate 安全派生，再显式推进或最终候选返工**。源中已有候选审阅、跨 fine advance、rework、非末尾 AwaitingGate、Running、仅领取而无停止 attempt、并列首个 fine、含 failed / objectHeld 的 reorder 历史、处置/导入/发布、Running AI 规划及其他未知记录仍在准备前整体拒绝，不丢弃未知数据。派生副本执行了新 advance/rework 后，不意味着该新历史现在也能再次派生。

完整对象数据派生、F1.4、F1 和整体开发仍未完成。未运行真实 HTTP/MCP transport、WebView、UI 视觉、外部 AI/Godot/Blender 或全量 native acceptance；未构建新 EXE、递增版本、提交或推送。源码版本保持 `0.1.19.44`，旧用户 Beaver 实例未替换，因此不声称新编译包已经交付。
