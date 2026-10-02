# F1：非运行中 AI 规划会话的独立派生

日期：2026-10-01（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[手工制造计划派生](FRAMEWORK-F1-PLAN-DERIVATION-EVIDENCE.md)；汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。前批证据和计数不改写成本批结果。

## 可用范围与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。新增三类实体：`object_task_planning`、`object_task_planning_head` 和 `object_task_planning_receipt`。支持 `AwaitingInput`、`Proposed`、`Adopted`、`Cancelled`、`Failed`、`Interrupted`；`Running` 继续拒绝。

离线源经检查、准备、组装、显式启用及登记后，副本中的 Proposed 可显式采纳，再通过正式草稿 API 编辑、保存及提交；AwaitingInput 可显式回答，生成新轮次并清除旧 Codex thread/turn。读取、关闭重开和重放历史 request 均不启动 AI 或任务；只有新用户命令触发新的规划工作。源项目和准备副本的 inventory 不变，失败证据保留。

仍不支持规划完成声明、入队、执行准备/尝试、导入/发布操作及其他未知记录，准备前整体拒绝而不是静默丢弃。F1.4、F1 及整个 Beaver 开发不因此完成。

## 身份、历史和恢复边界

- session/start 及全部 planning receipt request 使用相同 `object_planning_request` namespace；round 使用 `object_planning_round`。对象、任务及版本沿用既有映射。
- draft/head key（尤其 `object-task-plan`）、decision/question/assumption/stage ID 保留语义，供正式消费者继续使用。
- thread/turn 保留为历史诊断，不作为副本执行恢复凭据；新 answer 显式创建 fresh round 并清除它们。生产 reopen 尚未接入 `planning::recover`，本批不扩大此接线。
- 仅 typed assumption `sourceDetail` 中的 `planning:<session_id>` 和 `planning:<session_id>/decision:<decision_id>` 重写 session 身份；goal、prompt、questions 和 answers 中的叙述字符串不全局替换。
- 冻结 context 按 `expectedPlanRevision` 校验任务起源、定义及取消时序，以及完整 task/run/assumption 集合；相同 assumption ID 的不同修订按 `(id, plan_revision)` 区分。
- start/adopt/cancel/answer 请求、session、head 和 revision 相互核对；answer 回执按轮次恰好消费 User decisions，拒绝借用 Automatic、重复消费和缺失回执，同时允许跨轮复用同一 question ID 及相同答案。
- 保留 opaque question/choice 扩展。非 Adopted 的 MAX draft revision 可保存并派生，显式 adopt 仍按正常业务规则拒绝 revision exhausted。
- 不交付完整历史详情 UI，也不声称运行中的规划可恢复或真实 Codex 提供方已验收。

## 模块责任与有效行数

来自本批 `effective-code-lines.json`，不是物理行估算。251–500 行模块保持单一责任；没有新增超限 exception 或改写 baseline。

| 文件（Core 前缀 `native/core/src/`） | 责任 | 有效行 |
| --- | --- | ---: |
| `project_derivation_plan_history.rs` | 计划历史、任务起源及取消 revision | 273 |
| `project_derivation_plan_rewrite.rs` | typed 计划及来源身份重写 | 110 |
| `project_derivation_planning_records.rs` | typed record/head/receipt 转换及未知字段拒绝 | 113 |
| `project_derivation_planning_context.rs` | 冻结上下文完整集合及历史时序校验 | 177 |
| `project_derivation_planning_receipts.rs` | 请求身份、轮次回执与 decision 消费 | 152 |
| `project_derivation_planning_validation.rs` | 状态、head、revision 和 adoption 一致性 | 178 |
| `project_derivation_planning_provenance.rs` | 来源 session/decision 存在性 | 57 |
| `project_derivation_planning_fixture.rs` | 真实业务命令创建历史夹具 | 176 |
| `project_derivation_planning_tests.rs` | 正向派生、重开、历史重放 | 182 |
| `project_derivation_planning_continuation_tests.rs` | 显式回答与 fresh round | 69 |
| `project_derivation_planning_rejection_tests.rs` | schema、身份和来源负控 | 332 |
| `project_derivation_planning_boundary_tests.rs` | 轮次、集合、opaque 扩展与 MAX revision 边界 | 252 |
| `native/desktop/src/migration_derivation_planning_tests.rs` | 正式迁移/规划/草稿消费者集成 | 224 |
| `src/ui/ProjectDerivationPanel.tsx` | 复用正式交互，仅更新支持范围说明 | 158 |

## 实际验证

证据目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-ai-planning-derivation-20261001-224254`。验证脚本为 `verify-planning-derivation.ps1`，结果汇总为 `focused-results.json`，日志统一 UTF-8 无 BOM。使用同一正常 PowerShell/MSVC 环境，未清除 `LIB`。

| 检查 | 结果 | 日志 |
| --- | --- | --- |
| `rtk proxy cargo test --locked -p beaver-core --lib project_derivation` | 70 passed，0 failed | `core-derivation-final.log` |
| `rtk proxy cargo test --locked -p beaver-core --lib object_task_planning` | 14 passed，0 failed | `core-planning-final.log` |
| `rtk proxy cargo test --locked -p beaver-desktop migration_` | 11 passed，0 failed | `desktop-migration-final.log` |
| 相关 14 个 Rust 文件的定向 `rustfmt --check` | 通过 | `rustfmt-check.log` |
| `rtk proxy npx --no-install prettier --check src/ui/ProjectDerivationPanel.tsx` | 通过 | `prettier-ui-check.log` |
| `rtk proxy npm run typecheck` | 通过，此后未修改 TS | exec session `71019` 的完成证据；汇总明确标记为此前完成，不伪造重跑日志 |
| `rtk proxy npm run check:effective-lines` | 1266 sources、17 unchanged legacy files、0 violations | `effective-lines.log`、`effective-code-lines.json` |

这些测试组含各自既有回归，不将测试数量当作开发进度百分比。Desktop 用计数 factory 和受控 preparation failure 验证正式 Service 的调度次数，不启动真实 Codex：旧请求重放/读取/reopen 为 0，新 answer 后为 1，新 start 后为 2；重复命令和再次重开不增加计数。Proposed 显式 adopt 后经正式 dispatcher 编辑、保存、提交草稿，queue/claim 仍为空。

开发批次保留已有 unused mut/dead fields 警告。PowerShell 5 的 `System.Management.Automation.RemoteException` 为 native stderr 包装呈现；上述通过结论根据实际 exit code 和 `test result`，没有隐藏警告。

## 保留的失败与修正

1. Core 身份负控把 `/context/objects/0/id` 改为 `missing`，真实业务先拒绝冻结版本 identity。预期由 `OWNER_MISSING` 修正为 `IMPORT_VERSION_IDENTITY_MISMATCH`；生产校验未放宽。单项重跑 1 passed，日志 `core-identity-negative-retry.log`；首轮失败保留。
2. Desktop 首轮在创建问题夹具时失败：`请通过 beaver_ask_user 提供 recommended、importance 和 reason`。补齐夹具的正式问题协议，未放宽生产要求。单项重跑 1 passed，失败/成功日志分别为 `desktop-planning.log`、`desktop-planning-retry.log`。
3. 首轮 Core 日志被 PS5 `Tee-Object` 写成 UTF-16 BOM；已保留内容并转换为 UTF-8 无 BOM，没有改写中文为问号或 Unicode 转义。

源码摘要、严格 UTF-8/BOM 和 Markdown 链接核对写入同目录 `source-manifest.json` 与 `delivery-integrity.json`。定向测试子进程结束；旧 `release/Beaver-native-0.1.19.43-win32-x64/Beaver.exe` 是既有应用实例，不当作泄漏测试进程终止，无关 Cargo 工作不干预。

## 尚未完成

下一可用切片是规划完成声明的独立派生，需同步 owner、冻结 scope、历史 fingerprint 和不可变回执，并验证正式 API/看板消费者继续使用。入队及执行生命周期、导入/发布记录的转换仍另需闭合。

本批未重跑 UI smoke、前端构建、HTTP/MCP 传输、原生 WebView、真实工具绑定或全量验收；未构建新 EXE、递增版本、提交或推送。版本仍为 `0.1.19.44`，新编译包尚未交付。
