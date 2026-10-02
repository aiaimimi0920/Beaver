# F1：独立项目派生 API/UI 实施与验证

日期：2026-10-01（America/Los_Angeles）。对应 [实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)；继承的准备、组装、启用和登记边界见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。

后续范围更新：对象目录、固定版本与接受历史的派生已在下一切片接通，见[对象派生证据](FRAMEWORK-F1-OBJECT-DERIVATION-EVIDENCE.md)。下文保留本阶段的支持范围、测试计数和交付事实；不表示下一切片重跑过 UI 交互或构建。

## 本次可用范围

正式入口：**设置 → 数据迁移 → 派生独立项目副本**。

对于既有派生白名单支持的旧项目，正式 UI 已接通以下流程：

1. 选择离线源项目，读取其身份、可转换实体和调用记录数量，并提出新的项目 ID 与请求 ID；检查不写源数据库、不登记或打开源 Runtime。
2. 填写尚不存在的准备目录。准备重新验证源项目及其离线状态，并把该次请求和新身份写入磁盘回执。
3. 填写尚不存在的组装目录，生成独立身份副本。界面显示最终项目目录、准备摘要、历史任务中断数和会话路径转换数；最终项目位于组装目录的 `project` 子目录。
4. 核验后，先点击启用，再二次确认。启用不隐式登记，未启用的副本不能登记。
5. 单独点击“登记并恢复运行时”，分别展示登记是否提交、Runtime 是否恢复及调度通知错误。Runtime 就绪后可以“打开派生项目”：刷新宿主状态，选择返回的新项目 ID，进入项目概览并关闭弹窗。

当前仍只支持可转换的旧记录。包含 `object_*` 等新对象工作流数据的项目在源检查阶段明确拒绝，准备入口也会再次拒绝；不会静默丢弃未知记录或放宽身份/引用校验。源数据库离线写入排斥沿用当前 Windows 实现，不据此宣称其他平台可用。

## 目录和恢复协议

源目录、准备目录与组装目录不能互相嵌套，也不能与当前宿主数据目录重叠。新目录必须不存在、父目录必须存在；已绑定的组装绝对路径不得擅自移动。

| 所处状态                                    | 用户操作及保证                                                                                                                                         |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 源检查完成                                  | 提出的身份尚未被保留；准备时重新检查源项目，不能把检查结果当作持续持锁或源快照。                                                                       |
| 准备失败或响应丢失                          | 保留准备目录，先“核验已有准备副本”；完整回执恢复原请求与新身份。缺失或损坏时保留现场，换全新目录，不覆盖重试。                                         |
| 准备完成、原源项目不可用                    | 可以从“继续已有准备副本”恢复，不再依赖原源位置；`DERIVATION-COPY.json` 及内容仍须核验通过。                                                            |
| 组装失败或响应丢失                          | 保留组装目录，先核验组装回执；不完整时换全新组装目录。                                                                                                 |
| 启用响应丢失、尚未普通使用                  | 可以核验已启用副本，或沿既有幂等启用协议恢复；启用回执先落盘，再移除 pending 标记。                                                                    |
| 登记响应丢失，或登记提交后 Runtime 恢复失败 | 使用完全相同的准备/组装路径重试 `migration.registerAssembly`。不要重新组装、重新启用或比较已使用项目与旧 inventory；已提交的登记和后续项目修改均保留。 |
| 关闭后重新打开弹窗                          | 重新输入保留的目录。已尝试登记的副本选择“重试已启用副本登记”，直接恢复登记；本次未添加 UI 持久草稿。                                                   |

修改路径会清除对应结果及待确认状态。处理中阻止重复提交、关闭按钮和 Escape 关闭；关闭空闲弹窗不会清理副本。转换后的历史 `running`/`queued` 任务变为 `interrupted`，`waitingChildren` 保持原状态并设置 `planPaused: true`，不自动恢复历史执行。

## 实现归属

| 位置                                                                 | 责任                                                                                                                     |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `native/core/src/project_derivation_copy.rs`                         | 新增 `inspect_source`：项目锁、原始 DB writer exclusion、临时 SQLite 快照、身份和引用白名单检查；复用原有准备机制。      |
| `native/desktop/src/migration_derivation.rs`                         | 提供五个新公共方法，返回精简请求、计数和摘要，不把大型 inventory 传给 UI。                                               |
| `native/desktop/src/business_catalog.rs`、`migration_runtime.rs`     | 注册 schema、只读提示、支持范围，复用迁移操作锁和宿主路径隔离。测试间同步锁仅在 `cfg(test)` 下存在，不改变生产并发门禁。 |
| `src/ui/project-derivation-session.ts`                               | 新建、已有准备、登记恢复三种模式；请求状态、路径失效和响应丢失处理。                                                     |
| `src/ui/ProjectDerivationPanel.tsx`、`ProjectDerivationAssembly.tsx` | 源检查、目录说明、回执信息、显式启用与登记、打开项目按钮。                                                               |
| `src/ui/MigrationDialog.tsx`、`App.tsx`                              | 替换旧的仅启用/登记区域；保留归档迁移流程和全局 busy 保护，接入实际项目选择。                                            |

新增方法为 `migration.inspectDerivationSource`、`migration.prepareDerivation`、`migration.inspectDerivation`、`migration.assembleDerivation`、`migration.inspectAssembly`；启用和登记继续使用既有 `migration.activateAssembly` 与 `migration.registerAssembly`。

## 实际运行的定向验证

运行产物和日志目录：

```text
C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-derivation-ui-20261001-191526
```

工作目录为 Beaver 根目录。外部命令通过 `C:\Users\vmjcv\.local\bin\rtk.exe proxy` 执行；测试时 `TEMP` / `TMP` 指向上述目录，`BEAVER_UI_SMOKE_OUTPUT` 指向其 `ui` 子目录。Native 检查在同一 PowerShell 工具链环境下执行，保留现有 `LIB`，没有切换或清空 MSVC 环境。

| 命令或检查                                                                                 | 结果                                                                                                 | 日志                                                           |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `cargo test --locked -p beaver-core project_derivation`                                    | 47 passed，0 failed，568 filtered out；后续集成目标无匹配测试，不重复计数。                          | `core-project-derivation.log`                                  |
| `cargo test --locked -p beaver-desktop migration_`                                         | 8 passed，0 failed，158 filtered out。                                                               | `desktop-migration.log`                                        |
| `npm run typecheck`                                                                        | 通过。                                                                                               | `typecheck.log`                                                |
| `node node_modules/tsx/dist/cli.mjs scripts/migration-ui-smoke.ts project-derivation-flow` | 通过；Electron DOM 挂载正式 `MigrationDialog` 与生产 CSS，API 使用夹具。                             | `ui-derivation.log`                                            |
| `node node_modules/tsx/dist/cli.mjs scripts/migration-ui-smoke.ts migration-flow`          | 既有归档迁移交互回归通过。                                                                           | `ui-migration.log`                                             |
| `npm run build:native:ui`                                                                  | 通过；`Native frontend built without Electron or production source maps`。                           | `native-ui-build.log`                                          |
| `cargo fmt --all -- --check`                                                               | 命令 exit 0，无输出；日志辅助函数对空输出写文件失败，因此没有独立格式日志。此错误不是 rustfmt 失败。 | 本轮命令返回结果                                               |
| `node node_modules/prettier/bin/prettier.cjs --check`，参数限定本批 8 个 TS/TSX 文件       | 通过。                                                                                               | `prettier-source-check.log`                                    |
| `npm run check:effective-lines`                                                            | 1239 sources，17 unchanged legacy files，0 violations；未更新 baseline。                             | `effective-lines.log`、仓库 `output/effective-code-lines.json` |
| 本批 15 个源码/测试/脚本的严格 UTF-8 与 BOM 检查，以及结构报告 canonical SHA-256 核对      | 全部通过；规范化 LF 后的 SHA-256 与通过检查时一致，收尾没有再改源码。                                | `source-encoding-and-evidence-check.log`                       |

Core 新增回归证明源检查无写入、活动 ProjectStore 和原始数据库 writer 被拒绝，以及 `object_task/new-task: UNKNOWN_ENTITY_KIND` 拒绝且源内容不变。其余 45 个派生回归复用既有记录、历史、文件、会话、组装与启用测试，不算成本批新增用例。

Desktop 新增两个测试：一个通过 `migration_runtime::call` 串联完整流程并验证迁移并发门禁、源离线后恢复、未启用不能登记、启用可重试、2 个任务中断、等待子任务暂停、登记后修改项目再重试不覆盖；另一个验证公共 schema、只读提示、路径边界、损坏回执与禁止覆盖。其余迁移测试复用既有启用/登记恢复用例，包括登记已提交而 Runtime 恢复失败后的原路径重试。

UI 夹具覆盖目录选择取消、源/路径变化失效、重复提交和关闭保护、确认/取消启用、失败现场保留、准备/组装/启用/登记响应丢失、重开弹窗及打开准确的新项目 ID。这是正式组件的交互检查，不是原生 WebView、HTTP `/v1/call` 或 MCP 传输端到端验收。

有效行数：新 Desktop 适配 75、测试 204；新 UI 状态 hook 154、源面板 155、组装面板 124；新交互夹具 125/164。改动的 `App.tsx` 为 483、业务目录 398、`MigrationDialog.tsx` 为 270，其余本批文件均低于 251；没有修改超限 baseline 来接受改动。

编译保留已有无关警告：`native/core/src/validation/service.rs` 的 `store` / `files` 未读取，以及 `native/desktop/src/game_runtime.rs` 的 unused `mut`。本次没有顺便修改这些模块，也没有改动或撤销原有 asset-task 工作区内容。

## 交付边界与剩余项

- 本批完成受支持旧项目的源码 API/UI 流程及定向证明，未完成 `object_*` 全量数据派生；F1.4、F1 和 Beaver 总开发任务保持未完成。
- 没有构建新的 `Beaver.exe`，没有执行 `start-fresh-native-test.ps1`、真实工具绑定或正式原生/发布验收；旧 `0.1.19.43` 手测包不包含本批修改。
- `package.json` 仍为 `version: 0.1.19`、`beaverBuild: 44`，本批未递增版本；没有提交、推送或替换既有发布包。
- 文档收尾只检查格式、链接、编码和差异，不重新运行已经通过且源码未变的功能测试或构建。
