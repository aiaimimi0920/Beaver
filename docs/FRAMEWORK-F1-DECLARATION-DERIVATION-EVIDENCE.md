# F1：规划完成声明的独立派生

日期：2026-10-01（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[AI 规划会话派生](FRAMEWORK-F1-PLANNING-DERIVATION-EVIDENCE.md)；汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。前批结果及支持边界保持历史原样。

## 可用范围与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。新增 `object_task_planning_declaration`（head）和 `object_task_planning_declaration_receipt`（request receipt）转换。副本保留声明的 Owner、原因、创建时间、历史范围及 request 幂等；typed 身份转换后按目标 task ID 排序并重算 scope fingerprint，不复制旧身份的 hash。

副本可经正式任务 API 查看 current/stale/cancelled，重放不可变旧 request、检测 request 冲突，并在独立编辑后显式重新确认。关闭重开和登记重试不会覆写最新 head、接受任务或触发执行；源项目及准备副本不变。声明只表示规划完成确认，不改变 task/run/plan revision、不入队、不派发。

仍不支持已入队、执行准备/尝试、Running AI 规划、导入/发布及其他未知记录；准备前整体拒绝，不静默丢弃。F1.4、F1 及整个 Beaver 开发不因此完成。

## 身份与历史边界

- head key 是 root task ID；receipt key 是 request ID，分别采用 `object_task` 和 `object_task_request` namespace。声明 schema 严格解码，Owner、合法 IDs、非空且有界 reason、RFC3339 时间均核对。
- 历史快照仅重建已经验证过的 never-dispatched 计划：task origin 不晚于声明 revision，下次定义 revision 的 `before` 或当前定义恢复该边界，取消只在其真实 boundary 之后生效。后加入的任务不会进入旧声明范围。
- scope 包含 root 及责任后代，包括 cancelled 后代；依赖不是子任务。hash 是按 task ID 排序的定义 proposal、run ID 与 cancelled 标志的 JSON 数组 SHA-256。转换后复用生产算法，不实现另一套近似 fingerprint。
- 历史 scope、blockers、hash、排序 taskIds 与 receipt 精确核对；head 必须与其 request receipt 完整相等，每 receipt 要有 head，head revision 不低于同 root 的历史 receipt。
- 同 root、同 revision 的多份合法声明和创建时间倒退都允许，不能按 revision/hash 去重或依赖 wall-clock 判先后。旧 request 返回旧 receipt，不覆盖最新 head。
- current 看 scope hash 和 blockers，不要求声明 revision 等于当前全局 plan revision。无关编辑不使其失效，定义改动再恢复可令旧声明重新 current。
- 未放宽旧 migration ownership 或正常业务规则。composition 在同一 source transaction 内只校验 source entities 一次，再构建/核对 identity map，并读取已验证 Plans 供重写。

## 模块责任与有效行数

取自本批 `effective-code-lines.json`，不是物理行估算。没有新增 exception 或更新 baseline。

| 文件                                                                | 责任                                   | 有效行 |
| ------------------------------------------------------------------- | -------------------------------------- | -----: |
| `native/core/src/project_derivation_plan_snapshot.rs`               | 已验证计划在历史 revision 的任务快照   |     42 |
| `native/core/src/project_derivation_declaration_records.rs`         | 严格声明/head/receipt 校验及身份重写   |    127 |
| `native/core/src/object_task_planning_declaration.rs`               | 生产声明及共享 scope/hash 算法         |    195 |
| `native/core/src/project_derivation_declaration_tests.rs`           | 正向状态、历史重放、再声明与再次派生   |    228 |
| `native/core/src/project_derivation_declaration_rejection_tests.rs` | schema、范围、指纹与 head/receipt 负控 |    179 |
| `native/desktop/src/migration_derivation_declaration_tests.rs`      | 正式迁移及任务 API 消费者集成          |    223 |
| `src/ui/ProjectDerivationPanel.tsx`                                 | 复用正式交互，仅更新支持范围说明       |    160 |

## 实际验证

证据目录：`C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-declaration-derivation-20261001-232428`。脚本为 `verify-declaration-derivation.ps1`，实际命令、exit code 和起止 UTC 保存于 `focused-results.json`。沿用同一正常 PowerShell/MSVC 环境，不清除 `LIB`。本次只收取已运行测试的完成结果，不重复运行应用测试。

| 检查                                                                                                     | 结果                                                  | 日志                                               |
| -------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------- |
| `rtk proxy cargo test --locked -p beaver-core --lib project_derivation`                                  | 75 passed，0 failed                                   | `core-derivation.log`                              |
| `rtk proxy cargo test --locked -p beaver-core --lib object_task_planning`                                | 14 passed，0 failed                                   | `core-planning.log`                                |
| `rtk proxy cargo test --locked -p beaver-desktop migration_`                                             | 12 passed，0 failed                                   | `desktop-migration.log`                            |
| `rtk proxy cargo test --locked -p beaver-desktop data_dispatch_object_tasks::planning_declaration_tests` | 3 passed，0 failed；非 0 tests                        | `desktop-declaration.log`                          |
| `rtk proxy npm run typecheck`                                                                            | 通过                                                  | `typecheck.log`                                    |
| 相关 17 个 Rust 文件定向 `rustfmt --check`                                                               | 通过                                                  | `rustfmt-check.log`                                |
| `rtk proxy npx --no-install prettier --check src/ui/ProjectDerivationPanel.tsx`                          | 通过                                                  | `prettier-ui-check.log`                            |
| `rtk proxy npm run check:effective-lines`                                                                | 1271 sources、17 unchanged legacy files、0 violations | `effective-lines.log`、`effective-code-lines.json` |

测试数量包含各组既有回归，不等同进度百分比。Desktop 集成使用正式 migration dispatcher 和 object task dispatcher，覆盖公开 snapshot、历史 replay/conflict、关闭重开、独立编辑/重新确认、登记重试及无接受/派发；Core 同时覆盖新增成员与定义恢复边界。schema、key、owner、time、revision、hash、taskIds、head 与 receipt 的负控保持严格。

已有 unused mut/dead fields 警告保留；PowerShell 5 的 `System.Management.Automation.RemoteException` 是 native stderr 包装呈现，不是测试失败。结论依据真实 exit code 与 `test result`，未修改代码以换取绿色结果。

文档格式、Markdown 本地链接、严格 UTF-8/BOM、SHA-256 source manifest、diff 和进程核对另记于同目录 `source-manifest.json` 与 `delivery-integrity.json`。既有 `release/Beaver-native-0.1.19.43-win32-x64/Beaver.exe` 不当作泄漏测试进程终止。

## 尚未完成

下一可用切片是已入队但从未执行的制造计划独立派生，需保留队列、锁及命令历史，核对正式消费者与显式执行边界；执行准备/attempt、导入/发布和 Running AI 规划另需闭合。

本批未重跑 UI smoke、前端构建、HTTP/MCP 传输、原生 WebView、真实工具绑定或全量验收；未构建新 EXE、递增版本、提交或推送。版本仍为 `0.1.19.44`，新编译包尚未交付。
