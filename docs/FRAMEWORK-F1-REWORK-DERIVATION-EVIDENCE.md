# F1：已有纯文本返工历史的安全派生证据

日期：2026-10-02。接续[跨 fine 阶段历史派生](FRAMEWORK-F1-STAGE-DERIVATION-EVIDENCE.md)，本批完成“复制已有一次或多次纯文本返工的停止项目 → 保留反馈和完整历史 → 安全暂停、登记与重开 → 重新核验后显式恢复或返工”的源码工作流。派生副本可再次派生；F1.4、F1 和整体开发仍未完成。

## 可用流程与支持边界

正式入口仍为“设置 → 数据迁移 → 派生独立项目副本”。源项目须离线，准备和组装须使用全新目录；复用 inspect、prepare、assemble、activate、register 和重开流程，不增加另一套迁移协议。

- 支持最终 fine 的一次或多次纯文本候选返工，允许此前有完整同 fine retry 或有序跨 fine 推进。中间 AwaitingGate 必须有真实完整回执证明其返工后继；末尾可以为 Failed、Interrupted 或 AwaitingGate。
- 保留 accepted 前置阶段、用户反馈原文、全部 attempt 的冻结输入输出、历史检查、schema 1 / 2 候选审阅、Blocked / Started 回执以及 head / successor。任务、请求和 attempt 等 typed 身份同步转换，不把旧审阅替换成当前状态。
- 副本默认安全暂停。派生、登记、重开、历史重放、解除暂停、重新核验和生成审阅本身都不执行。Failed / Interrupted 需 fresh verify 后显式 resume；最终 AwaitingGate 需 fresh verify/check/review 后显式 rework，同请求并发和重放只启动一次。
- 历史 verification 和 review 不能充当新授权。新执行不复用旧 thread/turn，输入等于链尾输出；源项目、准备副本、宿主和其他项目保持隔离。

含 image、previewFrame 或 relocation 的返工仍以 `DERIVATION_REWORK_TEXT_ONLY` 拒绝。Running、仅领取无停止 attempt、pending / 孤立或缺失回执、缺失冻结内容、歧义 fine position、failed / objectHeld 排序历史、Running AI 规划、处置/导入/发布及其他未支持记录仍在准备前拒绝，不丢弃未知数据。

## 反馈与冻结历史转换

生产 `object_candidate_rework::text_suffix` 统一生成以下尾缀；派生只转换其系统生成的 review / attempt 身份，反馈原文不做全局替换：

```text
\n\nOwner rework feedback (candidate review {}, attempt {}):\n{}
```

`object_recovery_derivation_rework::Prompts` 仅在完整 closed-world 源验证通过后读取真实已完成回执，按 runId / recoveryGeneration 而非 request ID 排序。每项精确证明 `next.prompt == previous.prompt + text_suffix(approval)`，再用已映射的完整 prefix 和 typed approval 重建下一条 prompt。映射以源 task ID 和完整源 prompt 为键，覆盖连续返工与再次派生；歧义映射或超过生产 20 KB 限制在发布目标前拒绝。

全部 live / frozen task copies 共用同一映射，并在 task ID 转换前执行：当前计划、执行记录、recovery verification、saved.next、嵌套历史以及候选 Source。用户反馈中的旧身份、中文、换行或仿造尾缀均逐字保留，不搜索替换用户文本。

候选历史校验不再要求其 attempt 恰好是链尾，而是以真实完整链验证当时 Records 前缀。最终 fine 的历史 revision / prompt 必须由回执证明；canonical 前置 fine 必须 accepted，再精确比较 frozen fines。对象目录成员、不可变 metadata 依据、技术检查、rules、time、schema 及重建报告的原校验保留。生产 `saved.source.records == *records` 与 current Source 精确比较不变，不让历史审阅获得当前执行权。

## 回归与实际消费者

Core 夹具使用真实 Core 操作生成 1 / 2 次返工，request ID 故意先 `z-rework-*` 后 `a-rework-*`，避免误用字典序；反馈含旧身份、伪生成尾缀和“用户原文，不改写”。矩阵覆盖单 fine / 两个 accepted 前置 fine、schema 1 / 2、末尾 Failed / Interrupted / AwaitingGate，并连续派生两次。每次均重放历史回执及审阅，独立重建预期 prompt，核对冻结内容、状态、时间和规则、解除暂停零执行、fresh 显式操作唯一 lease，以及源/准备 inventory 不变。

旧 verification 必须被 `OBJECT_RECOVERY_RESUME_REVERIFY_REQUIRED` 拒绝；Gate 的旧 review 仍不可授权，fresh review 后才允许返工。负控覆盖 pending、伪造 prompt / feedback / review / attempt / Started / 嵌套 history、缺少回执或 successor、伪造历史审阅及三种非纯文本附件；inspect 与 prepare 均拒绝，目标不创建，源不变。

Desktop 新增两个完整消费者，使用正式 migration dispatcher、ProjectStorageRouter 与活动 Scheduler：

- `migration_derivation_rework_history_reopens_and_only_fresh_feedback_launches_once`：派生带有两次既有返工历史的项目，登记、重开、解除暂停和旧请求重放均不启动；fresh 核验/审阅后同请求并发只执行一次。
- `migration_derivation_staged_rework_history_preserves_accepted_prefix_and_launches_once`：相同流程带 accepted 前置阶段，保留其身份转换后的冻结历史及接受状态。

两项均在多个时点重放全部历史候选，验证关闭重开后的幂等与隔离。执行器通过真实测试 EXE 子进程 RPC 捕获产物，`result.txt` 精确为 `feedback received: Reduce movement speed`；这是 Scheduler / 执行器集成证据，不是外部 AI 验收。

正式派生面板同步支持/拒绝说明。UI 的生产候选 session 加 API fixture / SSR 回归覆盖 schema 1 / 2 × 单阶段/多阶段，同一 attempt 的多个历史报告与 fresh 报告同时展示；重开仅查询，历史 ID 不用于新授权，fresh explicit rework 唯一提交。不混入其他 attempt 报告，也未放宽 production strict target 检查。本批未运行真实 WebView 或浏览器。

## 模块责任与结构

本批涉及 21 个 Rust 文件和 2 个 TS/TSX 文件，新增 5 个源码文件。关键边界如下：

| 文件                                                           | 责任                                   | 有效行 |
| -------------------------------------------------------------- | -------------------------------------- | -----: |
| `native/core/src/object_candidate_rework.rs`                   | 生产返工合同和共用生成尾缀             |    188 |
| `native/core/src/object_recovery_derivation_rework.rs`         | 纯文本审批与完整 prompt 的回执依据转换 |     88 |
| `native/core/src/object_candidate_derivation.rs`               | 历史审阅验证与 frozen Source 身份转换  |    152 |
| `native/core/src/project_derivation_rework_fixture.rs`         | 真实 Core 单/多阶段连续返工夹具        |    117 |
| `native/core/src/project_derivation_rework_tests.rs`           | 再次派生、历史保留与重新授权矩阵       |    263 |
| `native/core/src/project_derivation_rework_rejection_tests.rs` | 不完整/伪造回执与非纯文本历史负控      |    134 |
| `native/desktop/src/migration_derivation_rework_fixture.rs`    | 正式 API/Scheduler 生成连续返工源      |    107 |
| `native/desktop/src/migration_derivation_gate_tests.rs`        | 共用 gate_flow 的安全派生到唯一执行    |    304 |
| `src/ui/ProjectDerivationPanel.tsx`                            | 生产入口的能力说明与操作约束           |    168 |
| `tests/object-candidate-derivation.test.ts`                    | 多个历史报告与 fresh 唯一提交消费者    |    210 |

四个凝聚测试文件为 263 / 284 / 285 / 304 有效行，分别负责本轮返工正向矩阵、候选派生矩阵、retry 负控和 Desktop gate workflow；其结构状态为 `cohesion`，由下列 Core / Desktop 派生组保护。生产模块均不超过 250 有效行。结构检查为 **1321 个源码、17 个未变动历史文件、0 个违规**；未更新 baseline 或新增 exception。

## 实际检查与失败记录

证据根目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-rework-history-derivation-20261002`。`run-focused.ps1` 为每个 job 保存 `.log` 和 `.result.json`，不覆盖旧日志；Cargo 保持普通 PowerShell 的同一 LIB 环境，TEMP/TMP 位于本批 `temp`。

| 检查                                                                                      | 实际结果                                              | 日志                                               |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------- |
| `rtk proxy cargo test --locked -p beaver-core project_derivation`                         | 117 passed / 0 failed                                 | `core-derivation.log`                              |
| `rtk proxy cargo test --locked -p beaver-core object_recovery`                            | 20 passed / 0 failed                                  | `core-recovery.log`                                |
| `rtk proxy cargo test --locked -p beaver-core stage_advance`                              | 4 passed / 0 failed                                   | `core-advance.log`                                 |
| `rtk proxy cargo test --locked -p beaver-core candidate_`                                 | 23 个单元及 2 个集成测试通过                          | `core-candidate.log`                               |
| `rtk proxy cargo test --locked -p beaver-desktop migration_derivation`                    | 15 passed / 0 failed                                  | `desktop-derivation.log`                           |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::advance_tests`   | 2 passed / 0 failed                                   | `desktop-advance.log`                              |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::candidate_tests` | 1 passed / 0 failed                                   | `desktop-candidate.log`                            |
| `rtk proxy npx tsx --test`：候选派生、审阅、返工和阶段推进消费者                          | 17 passed / 0 failed                                  | `ui-consumers.log`                                 |
| `rtk npm run typecheck`                                                                   | exit 0                                                | `typecheck.log`                                    |
| 21 个 Rust 文件定向 `rustfmt --check --edition 2021 --config skip_children=true`          | exit 0                                                | `rustfmt-check.log`                                |
| 2 个 TS/TSX 文件 `prettier --check`                                                       | exit 0                                                | `prettier-check.log`                               |
| `rtk npm run check:effective-lines`                                                       | 1321 sources、17 unchanged legacy files、0 violations | `effective-lines.log`、`effective-code-lines.json` |
| `rtk proxy npx tsx --test tests/product-version.test.ts tests/native-release.test.ts`     | 6 passed / 0 failed                                   | `version-metadata.log`                             |

测试数包含既有回归，各 Core 过滤器有交集，不合计为独立总数或完成百分比。集中检查通过后仅运行本次文档和版本元数据检查，不重复功能组或构建 EXE。

早期 `core-rework.log` 保存测试编译错误 `E0609`：`object_run_recovery::Target` 没有 `attempt_id`。修复后 `core-rework-r2.log` 为 3 passed / 2 failed，均为负控的精确错误预期与实际拒绝层不符：缺回执为 `OBJECT_RECOVERY_HISTORY_MISSING`，缺 successor 为 `OBJECT_RECOVERY_HISTORY_MISMATCH`；候选 source.records.history 篡改由 `OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH` 提前拒绝，source.fines 篡改由 `OBJECT_CANDIDATE_REPORT_MISMATCH` 拒绝。按真实合同调整精确预期，旧 forged rework 负控同样保留精确来源拒绝；未放宽生产校验。最终 117/0 派生整组覆盖修复和新增的旧 verification 拒绝断言，失败日志保留。

`before-manifest.json` 记录 1869 个改前 tracked / untracked 非忽略文件；`verify-delivery.ps1` 对比本批允许清单并输出 `source-manifest.json`、`delivery-integrity.json`，核对非本批文件与删除、UTF-8 无 BOM、文档链接、diff、结构报告 hash、测试子进程与原用户 Beaver。既有 `project_derivation_stage_tests.rs` 保留改前 SHA-256：`BE71106EF71DBE15948FACC1592A7611C283629BFF1D147B843EC818546F13FA`。未修改既有 unused/dead-code 警告。只读代理未返回有效结论，不宣称独立代理审查通过。

## 交付与未完成项

本批内部源码版本为 `0.1.19.46`，仅将 `beaverBuild` 从 45 改为 46，npm/Cargo/Tauri 的公开 SemVer 基础仍为 `0.1.19`。版本元数据 6 项通过不等于新安装包已构建或验收。

本段取代旧批次“一律拒绝已有返工历史”的当前范围，不改写旧证据。含图片、预览帧或区域重定位的返工，以及处置/导入/发布等完整对象历史仍需接通。F1.4、F1 和整体计划保持未完成。未执行真实 HTTP/MCP transport、WebView、视觉、外部 AI/Godot/Blender 或完整 native acceptance；未构建新 EXE、替换运行包、提交或推送。
