# F1：已有跨 fine 阶段历史的安全派生证据

日期：2026-10-02。接续[已有最终候选审阅派生](FRAMEWORK-F1-CANDIDATE-DERIVATION-EVIDENCE.md)，本批完成“复制有序阶段历史 → 保留已接受阶段并安全暂停 → 重新核验 → 显式恢复、推进或最终候选返工”的源码工作流。F1.4、F1 和整体开发仍未完成。

## 可用流程与支持边界

正式入口仍为“设置 → 数据迁移 → 派生独立项目副本”。源项目须离线，准备和组装目标须为全新目录；沿用 inspect、prepare、assemble、activate、register 与重开流程，不创建另一套迁移协议。

新增支持有完整回执的有序跨 fine 推进历史：

- 同 fine 的 Failed / Interrupted plain retry 与 AwaitingGate 后推进至不同 fine 可以组成完整执行链。末尾可以是 Failed、Interrupted 或 AwaitingGate；全部 attempt 的冻结输入、输出、准备、检查和中断证据必须完整。
- 保留已 accepted 的前置阶段、原 revision、推进 request / verification / result、head / successor、历史技术检查和冻结产物；typed 身份转换不替换标题、note、反馈或 prompt 中的原文。
- 最终 fine 的 schema 1 / 2 历史候选审阅可以带有 accepted 前置阶段。审阅的 fine 列表必须精确匹配 canonical 历史，末项必须为其冻结的当前 fine，前项全部 accepted；既有对象目录成员及不可变 metadata 历史校验保持。

副本默认安全暂停。复制、启用、登记、重开、历史 request 重放、解除暂停和重新核验不启动执行，不复制旧 lease。停止链必须 fresh verify 后显式 resume；待验收阶段必须 fresh verify/check 后显式 advance；最终候选必须 fresh verify/check/review 后显式 rework。已有历史审阅不能作为当前返工授权。同请求并发、重放及重开只产生一次新执行。

已有 Gate → 同 fine 返工链、处置/导入/发布、Running、仅领取无停止 attempt、pending 或孤立 journal、缺失冻结内容、歧义 fine position、failed / objectHeld 排序历史、Running AI 规划及其他未支持记录仍拒绝。中间 AwaitingGate 仅在完整回执证明其推进到另一 fine 时允许，不是任意放开历史状态。副本本轮显式返工成功后，其返工历史仍不能再次派生。

## 历史转换与执行授权

`object_recovery_derivation_stages` 从每个 distinct fine 的首次 planned 快照重建初始计划，再按当前回执在执行链中的位置投影此前 accepted 阶段。历史校验复用生产 `object_stage_advance::select_from_tasks`，检查相邻后继、依赖、技术检查、身份及 saved.next 精确快照，不拿当前已接受列表反推旧请求资格。同 medium 非 cancelled fine 的 position 不得并列，避免身份映射改变 ID 排序后的执行顺序。

`object_recovery_derivation_receipts` 继续验证完整请求/结果、generations、heads、edges 和 closed-world 实体集合；允许真实 advance，仍拒绝 rework。typed rewrite 映射 advance 的 attempt/check/next fine 身份和 saved.next，保留原 note、revisions 和 fresh_checks，不重新运行检查来改写历史结论。

`project_derivation_execution_inventory` 改为从真实 `resume::chain(...).last()` 取 run 终点。不同 fine 的 revision 不是可比较的时间线：首阶段 retry 次数较多时，其 revision 可以高于后续阶段。每个历史 blob 仍要完整匹配，当前 workspace 必须对应真实链尾，不能选中最大 fine.revision 的旧产物。

候选历史验证新增 `DERIVATION_CANDIDATE_FINAL_FINE_HISTORY_MISMATCH`，拒绝 accepted 阶段标题、revision、缺失或顺序伪造。原返工授权 `saved.source.records == *records` 和 current Source 精确比较未削弱；副本安全暂停使旧授权过期，fresh review 才能进入新的显式确认。

## 回归及实际消费者

Core 阶段夹具先形成 Interrupted → Failed → AwaitingGate 的同 fine retry 链，再推进至第二或第三个 fine；包含取消/Blocked 的历史推进及重新核验后的 Started 回执。各阶段输出不同，首阶段 revision 故意高于后续阶段。末尾 Failed / Interrupted / AwaitingGate 矩阵验证复制、组装、启用、两次重开、所有旧请求重放、accepted 历史、冻结产物、源/准备副本 inventory 和唯一新 lease。

负控覆盖篡改 check、note、next、dependency、fresh rules、accepted 历史、并列 position、pending 回执、缺失 edge 和旧 blob。候选矩阵覆盖 schema 1 / 2、多阶段历史、旧授权拒绝、fresh review、唯一返工、再次派生不叠加暂停，以及返工成功后再次派生仍被 `DERIVATION_EXECUTION_FIRST_STOPPED_ATTEMPT_REQUIRED` 拒绝。

Desktop 新增以下两个流程，使用正式 migration dispatcher、ProjectStorageRouter 和活动 Scheduler：

- `migration_derivation_stage_history_advances_next_fine_only_once`：派生已有跨阶段历史后，登记、重开、旧请求重放均零启动；fresh 显式推进的并发请求只启动一次，前置 accepted fine 不变。
- `migration_derivation_stage_history_requires_fresh_review_and_consumes_feedback_once`：保留历史审阅但拒绝旧授权，fresh 审阅后显式返工只启动一次；反馈实际进入捕获产物，`result.txt` 精确为 `feedback received: Reduce movement speed`。

这两项沿用真实子进程 RPC 与产物捕获，但模型由当前 Rust 测试 EXE 的 RPC 子测试模拟，不是外部 AI。输入等于链尾输出、不复用旧 thread/turn、源/准备副本/宿主/其他项目隔离均有断言。

UI 使用生产派生面板、候选 session 与 SSR：schema 1 / 2 × 单阶段/跨阶段矩阵证明历史阶段可展示、旧授权拒绝、fresh 记录用于显式唯一提交；派生面板 render 不执行 perform。API 响应使用 fixture，未运行浏览器或 native WebView。

## 模块责任与结构

本批涉及 23 个 Rust 文件、2 个 TypeScript/TSX 文件，新增 5 个源码文件；修改/新增源码的最大有效行数为 285，生产模块均不超过 250 行。重点边界如下：

| 文件                                                          | 责任                                    | 有效行 |
| ------------------------------------------------------------- | --------------------------------------- | -----: |
| `native/core/src/object_recovery_derivation_stages.rs`        | 回执时点的阶段计划和后继重建            |     79 |
| `native/core/src/object_stage_advance.rs`                     | 生产与历史共用的阶段选择                |    180 |
| `native/core/src/object_recovery_derivation_receipts.rs`      | 恢复/推进回执完整性                     |    183 |
| `native/core/src/project_derivation_execution_inventory.rs`   | 链尾工作区与全部冻结内容清单            |     77 |
| `native/core/src/object_candidate_derivation.rs`              | 最终候选历史及身份转换                  |    144 |
| `native/core/src/project_derivation_stage_history_tests.rs`   | 跨阶段派生到显式恢复的正向矩阵          |    145 |
| `native/core/src/project_derivation_stage_rejection_tests.rs` | 跨阶段历史损坏负控                      |    147 |
| `native/core/src/project_derivation_retry_rejection_tests.rs` | retry 历史快照、journal 和内容拒绝矩阵  |    285 |
| `native/core/src/project_derivation_candidate_tests.rs`       | schema/阶段组合的历史保留与重新授权矩阵 |    274 |
| `native/desktop/src/migration_derivation_gate_tests.rs`       | 共用 gate_flow 的派生后唯一执行工作流   |    278 |
| `src/ui/ProjectDerivationPanel.tsx`                           | 正式派生入口的支持范围和操作说明        |    168 |
| `tests/object-candidate-derivation.test.ts`                   | 候选历史到 fresh 显式返工的 UI 消费者   |    196 |

三个 251–500 行测试文件保持各自单一回归责任，结构扫描状态为 `cohesion`，不是违规或例外；保护命令分别为下表的 Core 派生与 Desktop 派生组。夹具和负控已按责任分开，没有为凑行数拆碎同一端到端断言。最终结构检查为 **1316 个源码、17 个未变动历史文件、0 个违规**，未更新 baseline 或新增 exception。交付审计精确核对这三个文件的状态、行数及全部 owned source 的扫描 hash。

## 实际检查与失败记录

证据根目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-stage-history-derivation-20261002`。各 job 的 `.log` 和 `.result.json` 由 `run-focused.ps1` 保存，既有日志禁止覆盖。Cargo 保持同一普通 PowerShell 工具链及原 LIB；TEMP/TMP 位于本批 `temp`。

| 检查                                                                                      | 实际结果                                                     | 日志                                               |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------ | -------------------------------------------------- |
| `rtk proxy cargo test --locked -p beaver-core project_derivation`                         | 首轮 111 passed / 1 failed；修正后整组 112 passed / 0 failed | `core-derivation.log`、`core-derivation-r2.log`    |
| `rtk proxy cargo test --locked -p beaver-core object_recovery`                            | 20 passed / 0 failed                                         | `core-recovery.log`                                |
| `rtk proxy cargo test --locked -p beaver-core stage_advance`                              | 4 passed / 0 failed                                          | `core-advance.log`                                 |
| `rtk proxy cargo test --locked -p beaver-core candidate_`                                 | 23 个单元及 2 个集成测试通过                                 | `core-candidate.log`                               |
| `rtk proxy cargo test --locked -p beaver-desktop migration_derivation`                    | 13 passed / 0 failed                                         | `desktop-derivation.log`                           |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::advance_tests`   | 2 passed / 0 failed                                          | `desktop-advance.log`                              |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::candidate_tests` | 1 passed / 0 failed                                          | `desktop-candidate.log`                            |
| `rtk proxy npx tsx --test`：候选派生、审阅、返工和阶段推进消费者                          | 17 passed / 0 failed                                         | `ui-consumers.log`                                 |
| `rtk npm run typecheck`                                                                   | exit 0                                                       | `typecheck.log`                                    |
| 23 个 Rust 文件定向 `rustfmt --check --edition 2021 --config skip_children=true`          | exit 0                                                       | `rustfmt-check.log`                                |
| 2 个 TS/TSX 文件 `prettier --check`                                                       | exit 0                                                       | `prettier-check.log`                               |
| `rtk npm run check:effective-lines`                                                       | 1316 sources、17 unchanged legacy files、0 violations        | `effective-lines.log`、`effective-code-lines.json` |
| `rtk proxy npx tsx --test tests/product-version.test.ts tests/native-release.test.ts`     | 6 passed / 0 failed                                          | `version-metadata.log`                             |

测试数包含既有回归，各 Core 过滤器有交集，不合计为独立总数或完成百分比。

首轮派生失败为 `project_derivation_retry_rejects_non_retry_purposes_and_missing_old_content` 的旧预期：伪造 advance 被真实 read-back 合同拒绝为 `OBJECT_STAGE_RECEIPT_MISMATCH`，而测试仍期待一律 `DERIVATION_RECOVERY_PLAIN_RETRY_REQUIRED`。修正为逐项精确预期并改名为 `project_derivation_retry_rejects_forged_purposes_and_missing_old_content`：伪造 advance、freshChecks、next 仍被拒绝；rework 仍要求 plain retry；缺失历史内容、源 inventory 不变和目标不创建断言保留。未为得到绿色结果放宽生产校验；整组复测 112/0，原失败日志保留。

Desktop 夹具检查中修复了按 fineTaskId 选当前候选会命中同 fine 早期 Failed attempt 的问题。现在从最后 resume.result.attemptId 精确定位映射后的链尾，再断言 fine 身份与 awaitingGate；修复后正式派生组 13 项通过。

本轮交接中曾误占已有 `project_derivation_stage_tests.rs`；已从改前备份逐字节恢复，将新增测试移入独立的 `project_derivation_stage_history_tests.rs` 并更正模块挂接。原 identity-stage 四项回归保留并在 112 项派生组通过。交付审计额外断言旧文件与改前 SHA-256 完全相同：`BE71106EF71DBE15948FACC1592A7611C283629BFF1D147B843EC818546F13FA`。

`before-manifest.json` 记录 1863 个改前 tracked / untracked 非忽略文件；`verify-delivery.ps1` 输出 `source-manifest.json` 和 `delivery-integrity.json`，核对无非本批改动/删除、UTF-8 无 BOM、文档链接、diff、扫描 hash、测试进程退出和原用户 Beaver 保留。既有 unused/dead-code 警告未改动。本批未运行 Godot/Blender，也未按名称关闭用户应用。只读审查代理因 HTTP 503 未返回结论，本批不宣称独立代理审查通过。

## 交付与未完成项

本批内部源码交付版本为 `0.1.19.45`，仅将 `beaverBuild` 从 44 改为 45，npm/Cargo/Tauri 的公开 SemVer 基础仍为 `0.1.19`。版本/包元数据相关 6 项测试通过，不等于已生成或验收新安装包。

已有返工、处置/导入/发布及其他未支持历史仍需按真实协议逐项接通，完整对象历史派生、F1.4、F1 和整体计划保持未完成。未执行真实 HTTP/MCP transport、WebView、视觉、外部 AI/Godot/Blender 或完整 native acceptance；未构建新 EXE、替换运行包、提交或推送。
