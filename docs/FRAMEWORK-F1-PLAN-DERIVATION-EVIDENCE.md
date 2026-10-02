# F1：未入队手工制造计划的独立派生

日期：2026-10-01（America/Los_Angeles）。对应[实施计划 F1.4](FRAMEWORK-IMPLEMENTATION-PLAN.md)，接续[对象目录与历史派生](FRAMEWORK-F1-OBJECT-DERIVATION-EVIDENCE.md)；进度汇总见 [F1 实施记录](FRAMEWORK-F1-IMPLEMENTATION.md)。前两批证据保持原样，不用本批范围或计数改写历史。

## 可用范围与完成条件

正式入口仍为 **设置 → 数据迁移 → 派生独立项目副本**。本批闭合以下工作流：离线项目中的未入队手工计划经过检查、准备、组装、显式启用和登记后，保留任务层级、依赖、固定版本引用、草稿、修订及执行前取消历史；正式界面使用的草稿可以继续读取、保存、提交和解锁，计划可继续修订、取消，关闭重开后状态仍在。副本不会自动启动任务，源项目和准备副本保持不变。

在既有白名单上新增八类实体：`object_task`、`object_run`、`object_task_draft`、`object_task_plan_state`、`object_task_commit_receipt`、`object_task_cancel_receipt`、`object_task_definition_revision`、`object_task_draft_unlock_receipt`。其中 `object_run` 只允许尚未执行的计划记录或执行前取消记录，不代表支持运行尝试。

已入队、执行准备、运行尝试、AI 规划会话、规划完成声明、导入/发布操作和其他未知记录仍在创建准备副本前整体拒绝，不静默省略、不覆盖已有目录。F1.4、F1 和 Beaver 整体开发不因此标记完成。

## 身份与历史边界

- 对象、任务、run、命令 request 使用一致的新身份映射；草稿 proposal 中尚未实体化的对象和任务也采用相同 namespace，后续提交不会恢复成旧身份。
- 项目内的草稿 ID、stage ID 和 assumption ID 保留语义键。特别保留正式 UI 固定读取的 `object-task-plan`，避免副本中的草稿存在但界面无法找到。提示文字、路径、时间和其他不透明内容不做字符串替换。
- 转换复用真实 typed schema，检查实体 key、project、task/run identity、引用归属、依赖环、当前状态与修订来源。未知字段或不受支持的记录明确失败，防止反序列化后丢字段。
- 草稿与回执是历史，不当作对当前定义重新执行的新写入。固定版本历史通过冻结版本读取；被引用父对象的当前分类变化不应使旧计划无法派生，也没有放宽正常 pinned 编辑规则或共享迁移 ownership 校验。
- 计划 revision 必须有连续的已知命令来源，允许真实 no-op commit 不增加 revision；task revision 由定义修订和有效取消历史解释。支持级联取消、子任务先取消后父任务取消，以及旧提交回执中的 planned run 对应当前 cancelled run。
- 派生后的历史 commit、revise、cancel 和 unlock 可以幂等重放；过期草稿仍按正常业务规则拒绝提交。使用后的副本可以再次派生，不能靠删掉历史才能继续工作。

## 实现归属与有效行数

以下计数来自本批通过的 `effective-code-lines.json`，不是物理行数。

| 模块                                                         | 责任                                 | 有效行 |
| ------------------------------------------------------------ | ------------------------------------ | -----: |
| `native/core/src/project_derivation_plan_rewrite.rs`         | 计划、草稿、任务、run 的一致身份映射 |     92 |
| `native/core/src/project_derivation_plan_records.rs`         | 八类 typed 记录及回执转换            |     98 |
| `native/core/src/project_derivation_plan_validation.rs`      | 当前归属、identity 与未执行状态校验  |    215 |
| `native/core/src/project_derivation_plan_proposals.rs`       | proposal 引用、冻结版本与无环检查    |    123 |
| `native/core/src/project_derivation_plan_history.rs`         | 草稿、回执及 revision 历史一致性     |    266 |
| `native/core/src/project_derivation_plan_fixture.rs`         | 通过真实业务命令创建计划历史         |    121 |
| `native/core/src/project_derivation_plan_tests.rs`           | 派生后继续工作、幂等、重开和再次派生 |    220 |
| `native/core/src/project_derivation_plan_rejection_tests.rs` | 损坏、不支持数据及拒绝前无写入       |    301 |
| `native/desktop/src/migration_derivation_plan_tests.rs`      | 迁移 API 与正式 task dispatcher 串联 |    134 |

集成沿用 `project_derivation_identity_keys`、`project_derivation_identity`、`project_derivation_database`、`project_derivation_validation` 和模块注册。`object_task_draft_unlock` 只把已有 Receipt 结构及字段开放为 `pub(crate)` 供转换复用。`data_dispatch.rs` 新增 `#[cfg(test)]` 薄转发供集成测试调用正式 object task dispatcher，不扩大生产模块可见性，不复制或伪造业务逻辑。

`ProjectDerivationPanel.tsx` 本批只有支持范围说明和格式变化（155 有效行），沿用此前的正式组件交互流程。旧“未知 object_task”拒绝回归改为仍不支持的 `object_attempt`，并明确断言 `UNKNOWN_ENTITY_KIND`；不是删除未知数据保护。

## 实际验证

本批证据目录：

```text
C:\Users\Public\nas_home\AI\GameEditor\linshi\beaver-plan-derivation-20261001-210538
```

命令从 Beaver 根目录执行，外部命令通过 `C:\Users\vmjcv\.local\bin\rtk.exe proxy`。Native 测试和结构检查使用同一 PowerShell 工具链，保留现有 `LIB`，`TEMP` / `TMP` 指向上述证据目录；没有清空或切换 MSVC 环境。

| 命令或检查                                                            | 结果                                                  | 日志或证据                                                                       |
| --------------------------------------------------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------------------- |
| `cargo test --locked -p beaver-core --lib project_derivation`         | 57 passed，0 failed                                   | `core-derivation-final.log`                                                      |
| `cargo test --locked -p beaver-desktop migration_`                    | 10 passed，0 failed                                   | `desktop-migration-final.log`                                                    |
| `npm run typecheck`                                                   | 通过                                                  | `typecheck.log`                                                                  |
| `npx --no-install prettier --check src/ui/ProjectDerivationPanel.tsx` | 通过                                                  | `prettier-ui-check.log`                                                          |
| 定向 `rustfmt --check --edition 2021 --config skip_children=true`     | 通过                                                  | `rustfmt-check.log`、`rustfmt-regression-check.log`、`rustfmt-desktop-check.log` |
| `npm run check:effective-lines`                                       | 1255 sources，17 unchanged legacy files，0 violations | `effective-lines.log`、`effective-code-lines.json`                               |

两组 Native 测试合计 **67 项**，包含已有回归，不是 67 个新测试；新增计划用例为 Core 5 项、Desktop 1 项。首轮失败和修复后的重跑不重复计数，上批 90 项也不计入本轮。最终退出码、测试行和结构计数见 `focused-results.json`，20 个本批源码的 SHA-256、严格 UTF-8 无 BOM 和有效行数见 `verified-sources-final.json`。

关键证明：

- Core 经真实对象与任务 API 创建包含父子任务、依赖、assumptions、固定版本、历史草稿、修订、解锁、级联取消和当前 UI 草稿的源项目；派生后验证身份一致、内容保留、队列为空、无法领取旧计划，继续编辑后关闭重开，再次派生仍成功。
- Core 拒绝用例分别检查 inspect 和 prepare 的错误码，验证源 inventory 不变且目标目录未创建，覆盖身份损坏、缺失或篡改历史、悬空引用、依赖环、未知字段、未知实体以及真实 `enqueue` 后拒绝。
- Desktop 用例通过公共 `migration.*` 分发入口完成离线检查至登记，再走生产 `objectTask.*` dispatcher 的 catalog 参数验证、项目 Runtime 获取和业务调用；继续保存、提交、修订、解锁、取消并关闭重开。队列与 claim 均证明没有自动派发，源和准备副本 inventory 保持不变。

### 失败修复与工具边界

保留首轮失败日志：`core-plan-first.log`、`core-derivation.log`、`desktop-migration.log` 和 `prettier-ui.log`。Core 首轮暴露了断言错误码不匹配及旧测试仍把新支持的 `object_task` 当作未知实体；按实际校验顺序和新白名单修正，最终 57 项通过。Desktop 首轮为 `error[E0433]`：测试引用了 crate 根下不存在的私有模块；改用上述 test-only 转发后，10 项全部通过。没有用放宽生产写入校验使测试通过。

按约定尝试 context-mode 读取保存日志，但服务根配置为其插件目录，返回 `File access blocked`；未改变权限或绕过边界，改用 PowerShell 聚合既有日志。只读子代理均因模型渠道 503 不可用，本批没有有效的独立代理审查结论。FastCtx 连接为 `Transport closed`，精确读取使用 PowerShell 替代。

保留已有无关警告：`object_metadata_tests.rs` 的 unused `mut`、`validation/service.rs` 的未读取字段、`game_runtime.rs` 的 unused `mut`。`System.Management.Automation.RemoteException` 是 PowerShell 包装 native stderr 的呈现；最终结果按实际 exit code 和 `test result` 判断，两组测试、类型、格式及结构检查均通过。

## 剩余项与交付边界

- 执行/入队记录、运行尝试、AI 规划和声明、导入/发布操作的转换及原生整体验收仍未完成；F1.4、F1 保持未勾选。
- 正式 UI 只更新支持范围文案，复用此前组件交互证据；本批未重跑 UI smoke、前端构建、HTTP/MCP 传输或原生 WebView 验收。
- 没有构建或启动新 `Beaver.exe`，未运行 `start-fresh-native-test.ps1` 或真实工具绑定，不能把测试二进制当作新手测包。用户目标中的新编译包尚未交付。
- 版本保持 `version: 0.1.19`、`beaverBuild: 44`。未递增版本、提交、推送、替换发布包或更新结构 baseline，保留原有 asset-task、相关测试和缓存等无关修改。
- 功能检查通过后不再扩展测试。收尾仅更新文档、检查文档链接及编码、复核源码摘要与 `git diff --check`。
