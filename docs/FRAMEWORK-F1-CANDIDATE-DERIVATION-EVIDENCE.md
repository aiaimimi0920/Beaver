# F1：已有最终候选审阅的安全派生证据

日期：2026-10-02。接续[待验收安全派生](FRAMEWORK-F1-GATE-DERIVATION-EVIDENCE.md)，本批完成“保留历史审阅 → 安全暂停 → 重新核验和审阅 → 显式返工”的源码工作流，不代表 F1.4 或完整原生验收完成。

## 可用流程与支持边界

正式入口仍是“设置 → 数据迁移 → 派生独立项目副本”。源项目必须离线，目标与准备目录必须为全新位置；源、准备副本和失败目录保留，不覆盖历史。

本批新增支持以下源记录：

- 唯一首个 fine 同时是最终 fine，terminal attempt 为 `AwaitingGate`，已有一条或多条可核对的 `object_candidate_review`。
- 此前可以有同一 fine 的完整 `Failed` / `Interrupted` plain retry 链，仍要求完整输出、冻结内容、工作区、检查及不可变回执。
- 历史审阅的 schema 1 / 2、原始时间、技术检查结果、blockers、文件内容摘要与文案保持。对象目录成员集合必须与当前集合相同；成员的 metadata revision 漂移必须能通过唯一不可变命令回执精确证明。

副本中的审阅可以读取、重放和关闭重开，但只是历史证据，不是新的执行授权。登记、登记重试、历史请求重放、读取、重开、解除安全暂停、重新核验、技术检查和生成审阅均不启动执行。解除暂停后必须重新核验、技术检查并生成新审阅，再从新记录填写反馈并显式确认返工；旧审阅被精确拒绝为 `OBJECT_CANDIDATE_REWORK_SOURCE_MISMATCH`。新返工同请求并发、重放和重开只启动一次。

源中已有跨 fine 推进、返工、处置/导入/发布、非末尾 `AwaitingGate`、Running、仅领取无停止 attempt、并列首 fine、failed / objectHeld 排序历史、Running AI 规划及其他未知记录仍在准备前整体拒绝。副本执行新返工后，不代表该返工历史现在也能再次派生。

## 转换与授权不变量

`object_candidate_derivation::validate` 遍历全部候选实体，包括普通 attempt reader 会跳过的孤儿；校验 key/request/project/check 身份、terminal target、唯一最终 fine、完整历史 Records 与对象目录成员。历史 Records 复用 recovery snapshot 校验，旧 ObjectRecord 必须与不可变回执中的精确快照相符。Records.object / baseline_sources 还必须与同一 Source.objects 快照一致，不能拼接不同时点的 owner 记录。

源 review 引用的技术检查必须通过且匹配 attempt；review 的重新检查可以失败，必须保留当时的 rules 和 issues。规则结构及 typed summary 均严格核对，不通过重新检查当前工作区来改写历史。

转换只处理声明身份：review/check request namespace、target、历史 Records、fine 和对象身份。映射后恢复 canonical 排序，使用原 Source、rules、time 和 schema 重建 source digest、references、owners。文件路径、issue 文案、阶段标题、正文中的源 ID 不作字符串替换。仅接入已存在的离线转换、准备、组装、启用和登记链，没有新开并行持久化协议。

原返工授权比较 `saved.source.records == *records` 和 current Source 精确比较保持不变。派生追加的暂停 revision 使旧审阅不再等于当前 Records；解除暂停也不会还原旧授权。未放宽状态、回执或摘要条件以使测试通过。

## 回归证明及实际消费者

Core 新增五项候选派生测试：

1. schema 1 / 2、直接成功 / retry、Empty / Pinned 基准的组合，包含真实 metadata 更新和 ID 排序变化；检查历史映射、原文保持、源和准备副本 inventory、不自动 claim、旧审阅拒绝以及 fresh review 的唯一返工 lease。
2. 连续两代再派生保留历史审阅和安全暂停，不重复叠加已暂停的 control revision，也不自动领取。
3. 孤儿、错误 key/project/target/check、未知 schema、摘要、规则、fine、对象历史及 control 篡改在 inspect/prepare 阶段拒绝。
4. 缺失、重复、乱序或成员变动的 catalog，以及伪造和不同时点拼接的 owner 快照拒绝。
5. 合法历史 failed recheck 保持失败和原 issue 文案。这是历史记录构造夹具，不是本批实测的 blob 故障注入。

Desktop 新增 `migration_derivation_reviewed_candidate_requires_fresh_review_and_consumes_feedback_once`，使用正式 migration dispatcher 和活动 Scheduler，完成 inspect → prepare → assemble → activate → register。源已有真实 API 生成的审阅，副本读取、旧请求重放和重开均零启动；解除暂停、fresh verify 后旧审阅仍拒绝。fresh check/review 后相同显式 rework 请求并发只产生一个新 attempt，输入等于 terminal output，不复用旧 thread/turn。

Scheduler factory 使用当前 Rust 测试 EXE 的 RPC 子测试，走真实子进程协议与产物捕获，不调用外部 AI。反馈 `Reduce movement speed` 实际进入新产物：`result.txt` 精确为 `feedback received: Reduce movement speed`。随后关闭重开、重复请求、再次读取历史均不重复执行；源、preparation、宿主库和其他项目隔离有断言。

生产 UI 的派生入口说明与候选审阅面板同步说明新范围和 fresh review 流程。新增两项 schema 1 / 2 UI 回归使用真实 execution/check/candidate/recovery session 与面板 SSR，证明 refresh 只查询、旧审阅错误不保留 retry 授权、新旧记录共存、从 fresh review 准备确认不 dispatch、显式 submit 使用新 review ID 和反馈且不重复提交。UI API 为 fixture，唯一实际启动及反馈产物由 Desktop 回归证明；本批未运行浏览器或 WebView。

## 模块责任与结构

本批改动 19 个 Rust 文件、5 个 TypeScript/TSX 文件，均不超过 250 有效行；最大为 247 行。重点模块如下：

| 文件                                                              | 责任                                   | 有效行 |
| ----------------------------------------------------------------- | -------------------------------------- | -----: |
| `native/core/src/object_candidate_derivation.rs`                  | 冻结审阅校验及 typed 身份转换          |    139 |
| `native/core/src/object_recovery_derivation_records.rs`           | 复用历史 Records 转换，不改变快照语义  |     56 |
| `native/core/src/object_recovery_derivation_snapshot.rs`          | 不可变历史对象及执行快照锚定           |    162 |
| `native/core/src/object_attempt_checks.rs`                        | 原技术检查规则结构条件共用             |    175 |
| `native/core/src/project_derivation_candidate_fixture.rs`         | 真实源审阅和 fresh verify 构造         |     61 |
| `native/core/src/project_derivation_candidate_tests.rs`           | 映射、再派生及重新授权正向回归         |    247 |
| `native/core/src/project_derivation_candidate_rejection_tests.rs` | 孤儿、历史、catalog 和 rules 负控      |    223 |
| `native/desktop/src/migration_derivation_candidate_fixture.rs`    | 正式 API 的源审阅、重放及旧授权拒绝    |    120 |
| `native/desktop/src/migration_derivation_gate_tests.rs`           | 派生到唯一推进/返工与产物隔离          |    222 |
| `src/ui/ProjectDerivationPanel.tsx`                               | 正式派生入口和支持范围                 |    168 |
| `src/ui/object-tasks/ObjectCandidateReviewPanel.tsx`              | 历史审阅展示与新授权操作指引           |    185 |
| `tests/fixtures/object-candidate-review.ts`                       | 共用 schema 1 / 2 报告夹具             |     51 |
| `tests/object-candidate-derivation.test.ts`                       | 历史读取到 fresh review 显式确认消费者 |    159 |

最终 `npm run check:effective-lines` 为 **1311 个源码、17 个未变动历史文件、0 个违规**；没有更新 baseline 或新增 exception。

## 实际验证与失败记录

证据根目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-reviewed-candidate-derivation-20261002`。`run-focused.ps1` 为各 job 保存独立日志和 `.result.json`，禁止覆盖已有日志。Cargo 全程使用普通 PowerShell 工具链，保留原 LIB，TEMP/TMP 指向本批 `temp`；`environment.json` 保存前置环境。

| 检查                                                                                      | 实际结果                                              | 日志                                               |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------- |
| `rtk proxy cargo test --locked -p beaver-core project_derivation`                         | 107 passed / 0 failed                                 | `core-derivation.log`                              |
| `rtk proxy cargo test --locked -p beaver-core object_recovery`                            | 20 passed / 0 failed                                  | `core-recovery.log`                                |
| `rtk proxy cargo test --locked -p beaver-core candidate_`                                 | 22 个单元测试与 2 个集成测试通过                      | `core-candidate.log`                               |
| `rtk proxy cargo test --locked -p beaver-core object_attempt_check`                       | 3 passed / 0 failed，非零测试                         | `core-checks.log`                                  |
| `rtk proxy cargo test --locked -p beaver-desktop migration_derivation`                    | 11 passed / 0 failed                                  | `desktop-derivation.log`                           |
| `rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime::candidate_tests` | 1 passed / 0 failed                                   | `desktop-candidate.log`                            |
| `rtk proxy npx tsx --test`：派生审阅、候选审阅、返工、技术检查消费者                      | 15 passed / 0 failed                                  | `ui-consumers.log`                                 |
| 修改类型断言后，仅复测 `tests/object-candidate-derivation.test.ts`                        | 2 passed / 0 failed                                   | `ui-derivation-retest.log`                         |
| `rtk npm run typecheck`                                                                   | 首轮 TS2532；修正后 exit 0                            | `typecheck.log`、`typecheck-retest.log`            |
| 19 个 Rust 文件定向 `rustfmt --check --edition 2021 --config skip_children=true`          | exit 0                                                | `rustfmt-check.log`、`rustfmt-check.result.json`   |
| 5 个 TS/TSX 文件 `prettier --check`                                                       | exit 0                                                | `prettier-check.log`                               |
| `rtk npm run check:effective-lines`                                                       | 1311 sources、17 unchanged legacy files、0 violations | `effective-lines.log`、`effective-code-lines.json` |

首轮类型检查在新增 UI 测试访问 `reports[1].request` 时报 `TS2532: Object is possibly 'undefined'.`。修复为局部绑定后 `assert.ok(fresh)`，显式证明第二条记录存在，未使用非空断言绕过。仅重新格式化该文件、复测其 2 项测试和类型检查；未重复未受影响的 UI 测试。原失败日志保留。既有 unused/dead-code 警告未改动。

测试数量包含既有回归，且 candidate 过滤器与派生回归有交集，不合计为独立覆盖数或项目完成百分比。本批 107 项是当前源码的全组派生结果，不回写前批“101 passed / 1 failed，修正后仅单项复测”的历史证据。

`before-manifest.json` 保存 1855 个改前文件的 SHA-256；交付时另存 `source-manifest.json` 和 `delivery-integrity.json`，核对本批文件、UTF-8 无 BOM、本地文档链接、结构报告、非本批内容保持和测试子进程退出。本批没有启动 Godot/Blender，没有按名称关闭用户应用。

## 尚未交付的范围

本切片完成上述有限源状态的源码工作流；完整对象历史派生、F1.4、F1 及整体计划继续保持未完成。后续需要处理已有返工/跨 fine/处置/发布等各自的真实历史协议，不能直接扩展白名单。

未执行真实 HTTP/MCP transport、WebView、视觉、外部 AI/Godot/Blender 或完整 native acceptance；未构建新 EXE、递增版本、提交或推送。源码版本保持 `0.1.19.44`，未替换用户正在使用的 Beaver。
