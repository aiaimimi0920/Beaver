# F1：对象目录与版本历史的独立派生

日期：2026-10-01（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[独立派生 API/UI 切片](FRAMEWORK-F1-DERIVATION-UI-EVIDENCE.md)；进度汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。

## 可用范围与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。本批完成条件是：含父子对象、固定版本引用、冻结文件和接受历史的受支持项目，可以完成离线检查、准备、组装、启用、显式登记/恢复，打开后继续更新、捕获和接受版本，关闭重开后状态仍在，源项目不变。

在既有旧记录白名单上，仅新增以下三种实体：

- `object`：当前对象目录、元数据、组件、文件归属、父子关系、固定版本引用。
- `object_version`：真实 `(String, ObjectVersion)` owner/version 结构及冻结 manifest。
- `object_command_receipt`：注册、更新、捕获、接受命令的输入摘要及历史结果。

对象制造任务、运行尝试、导入/发布操作记录等未支持实体仍在源检查和准备阶段明确拒绝，不会复制后丢弃数据。此切片不等于全量现代对象数据转换，也不代表 F1.4、F1 或 Beaver 开发全部完成。

## 身份、历史与文件边界

对象、版本、组件和命令请求获得独立身份；当前记录、冻结 manifest、父子关系、固定版本引用及回执结果使用一致映射。回执 key 按目标 project/request 的真实摘要算法计算，先用真实 `RegisterRequest`、`UpdateRequest`、`CaptureRequest`、`AcceptanceRequest` 重构并验证原 `inputDigest`，再转换身份和重算摘要。不能用任意 JSON 对象替代 typed serialization，因为字段顺序影响摘要。

文件路径、blob hash 和不透明内容保持原样。当前工作文件可以不同于旧版本快照，尚未创建的已登记文件也不因此被误认成冻结内容损坏；真正被版本引用的 `.beaver/content/blobs/<sha256>` 必须存在，且长度与内容摘要相符。源数据库只在隔离快照中打开；检查冻结 blob 时仍持有源排他边界，prepare 在创建目标目录之前验证，准备副本核验和组装也复核对应 inventory。

历史读取不再错误要求冻结版本的分类、标签或组件等于当前对象。父对象在被子对象固定引用后更新分类/标签，派生仍保留当前新元数据和冻结旧元数据；最新接受版本由回执 revision 决定，覆盖“先接受第二版，再接受第一版”。接受事件当时的回执仍按当时对象与版本验证；没有放宽全局 catalog 新写入规则或共享旧迁移 ownership。

派生专属校验覆盖版本 owner 与当前版本投影一致、历史回执 revision/版本一致、同 revision 不冲突、固定引用目标存在且已接受，以及当前组件 ID 唯一、文件路径忽略大小写后唯一、父子链存在且无环。篡改、未知类型或损坏副本被拒绝，不覆盖准备目录，不提前启用。

## 实现归属与有效行数

以下计数来自本批通过的 `effective-code-lines.json`，不是物理行数。

| 模块                                                           | 责任                                 | 有效行 |
| -------------------------------------------------------------- | ------------------------------------ | -----: |
| `native/core/src/project_derivation_object_records.rs`         | Typed 对象、版本及引用身份转换       |    106 |
| `native/core/src/project_derivation_object_receipts.rs`        | 真实命令输入重构、摘要验证与回执 key |    124 |
| `native/core/src/project_derivation_object_validation.rs`      | 当前归属、冻结历史与 blob 校验       |    205 |
| `native/core/src/project_derivation_object_fixture.rs`         | 经真实对象 API 创建父子和版本夹具    |    165 |
| `native/core/src/project_derivation_object_tests.rs`           | 完整生命周期与固定历史回归           |    238 |
| `native/core/src/project_derivation_object_rejection_tests.rs` | 损坏、未知数据与拒绝前无写入回归     |    139 |
| `native/desktop/src/migration_derivation_object_tests.rs`      | 公共迁移 API 的对象目录闭环          |     84 |

集成沿用 `project_derivation_validation`、`project_derivation_identity_keys`、`project_derivation_database`、`project_storage_database`、`project_derivation_copy` 与既有模块注册。`object_command_receipt` 仅将真实回执 schema 和摘要函数开放为 `pub(crate)` 供转换复用；`object_acceptance_history` 从冻结版本判断已接受状态（94 有效行）。`ProjectDerivationPanel.tsx` 仅更新支持范围文案（154 有效行），不改变上批交互流程。

## 实际验证

运行产物与日志：

```text
C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-object-derivation-20261001-202142
```

命令从 Beaver 根目录执行，外部命令通过 `C:\Users\vmjcv\.local\bin\rtk.exe proxy`。Native 测试及结构检查使用同一 PowerShell 工具链，保留现有 `LIB`，`TEMP` / `TMP` 指向证据目录的 `temp` 子目录；没有切换或清空 MSVC 环境。

| 命令或检查                                                            | 结果                                                  | 日志或证据                                                                        |
| --------------------------------------------------------------------- | ----------------------------------------------------- | --------------------------------------------------------------------------------- |
| `cargo test --locked -p beaver-core --lib project_derivation`         | 52 passed，0 failed                                   | `core-derivation.log`                                                             |
| `cargo test --locked -p beaver-core --lib object_version_acceptance`  | 4 passed，0 failed                                    | `core-acceptance.log`                                                             |
| `cargo test --locked -p beaver-core --lib object_run_baseline`        | 5 passed，0 failed                                    | `core-baseline.log`                                                               |
| `cargo test --locked -p beaver-core --lib object_recovery`            | 20 passed，0 failed                                   | `core-recovery.log`                                                               |
| `cargo test --locked -p beaver-desktop migration_`                    | 9 passed，0 failed                                    | `desktop-migration.log`                                                           |
| `npm run typecheck`                                                   | 通过                                                  | `typecheck.log`                                                                   |
| `npx --no-install prettier --check src/ui/ProjectDerivationPanel.tsx` | 通过                                                  | `prettier-ui.log`                                                                 |
| 定向 `rustfmt --check --edition 2021 --config skip_children=true`     | exit 0，无输出                                        | 终端返回结果；日志辅助错误见下文                                                  |
| `npm run check:effective-lines`                                       | 1246 sources，17 unchanged legacy files，0 violations | `effective-lines.log`、`effective-lines-result.json`、`effective-code-lines.json` |

五组 Native 定向测试合计 **90 项**，包含已有回归，不是 90 个新测试；不把早期首轮验证重复计入。机器可读退出码见 `focused-results.json`。收尾只核对保存日志、源码摘要和文档，不重新跑功能测试或构建。

关键证明：

- Core 完整流程验证独立对象/版本/组件身份、父子与固定引用、冻结文件读取、与冻结内容不同的工作文件、真实接受顺序、历史命令回放、新命令幂等、登记 replay、关闭重开以及再次派生。源项目和准备副本 inventory 保持不变。
- `project_derivation_keeps_pinned_history_after_referenced_parent_metadata_changes` 使用正常 API 修改被引用父对象的分类/标签，证明离线检查至启用成功、新旧元数据分开保存、固定旧版本仍可读取、最新接受顺序不变，父对象可继续捕获和接受。
- 拒绝回归覆盖 owner/version tuple 错配、摘要篡改、悬空/跨项目引用、未知 `object_task`、历史 parent 丢失、缺失/损坏 blob、覆盖准备目录和损坏副本组装。重复组件、大小写重复文件及父子环分别断言 `DUPLICATE_OBJECT_COMPONENT`、`DUPLICATE_OBJECT_FILE`、`OBJECT_PARENT_CYCLE`，不是任意报错即通过。
- Desktop 用例通过公共 `migration.*` 业务入口串联检查、准备、组装、启用、登记，随后读取新对象目录、继续捕获、重试登记和关闭重开；这不是经过 HTTP/MCP 传输或原生 WebView 的端到端验收。

### 日志和工具边界

保存的 Native 日志已核对各组 `test result` 与退出码，没有 Cargo error。按约定尝试 context-mode 读取日志索引，但服务配置的项目根为 `C:\Users\Public\nas_home\Crow`，返回 `File access blocked`；未更改其权限或绕过边界，改用正常 PowerShell 精确分析已有日志，结果为 `saved-log-analysis.json`。

保留已有无关警告：`object_metadata_tests.rs` 的三个 unused `mut`、`validation/service.rs` 的 `store` / `files` 未读取、`game_runtime.rs` 的 unused `mut`。日志中的 `System.Management.Automation.RemoteException` 对应 Windows PowerShell 对 native stderr 空行的呈现；以实际退出码和 `test result` 为准，五组测试与结构检查均 exit 0。

定向 rustfmt 实际 exit 0 且无输出，随后日志辅助函数对 `$null` 调用 `WriteAllLines`，产生 `Value cannot be null. Parameter name: contents`，因此没有成功写出 `rustfmt-check.log`。这是证据写入缺陷，不是 formatter 失败；没有为补空日志重跑功能测试。收尾检查记录见 `source-encoding-and-evidence-check.log` 和 `documentation-check.log`。

## 剩余项与交付边界

- F1.4、F1 和整体开发保持未完成；制造任务、运行尝试、导入/发布操作记录的转换仍需后续完整切片。
- 正式 UI 交互逻辑未变，本批复用上批的两套组件 UI smoke 证据；没有重跑 UI smoke、前端构建或宣称原生交互已验收。
- 没有构建新的 `Beaver.exe`，没有运行 `start-fresh-native-test.ps1`、真实工具绑定、HTTP/MCP 传输端到端或正式发布验收。用户目标中的新编译包尚未交付。
- `package.json` 保持 `version: 0.1.19`、`beaverBuild: 44`；未递增版本、提交、推送或替换已有发布包。
- 未修改结构 baseline，保留原有 asset-task、相关测试和缓存等无关 dirty 内容；收尾没有再改源码。
