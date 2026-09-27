# F9.3：旧数据迁移激活与重开验证

日期：2026-09-26。状态：本次 Core 集成验证完成；F9.3 和 F9 总体仍未完成。

## 结果与范围

本次补齐旧数据备份、恢复、项目分区、实际激活及关闭重开的集成回归。完成条件是：原宿主和项目路径不可用时，目标副本仍能完成工具绑定并打开项目；审批、取消、历史输入和验证证据保持可查，运行中和排队任务转为中断状态，源数据及迁移归档不变。

新增回归直接调用 Desktop 所使用的 `project_migration_activation::activate_copy`，不手动删除 `.beaver-migration-pending` 或写入 `ACTIVATION.json`。第一次为 Codex 指定错误类型的可执行文件，激活按真实版本校验失败，目标仍处于待激活状态，项目路由拒绝打开。修正配置后重试成功，真实激活入口写入回执和工具设置，再解除待激活限制。

成功激活后连续创建两组全新的 `Store` 和 `ProjectStorageRouter`，每轮关闭所有项目并释放数据库和锁。两次查询所得历史及恢复状态完全一致，覆盖以下边界：

- 两个项目分别使用各自的项目数据库；已转换的任务、资产状态和验证记录从宿主分区移除，不串入另一个项目。
- 用户接受记录通过生产 `task_actions::accept` 生成；审批来源、事件和未知旧字段保持不变，工作区路径按项目存储规则转换。
- 同一子任务的两次已完成尝试使用相同文件路径、不同冻结正文；当前工作区已再次修改，重开仍能读取各自历史 blob，并通过输入校验。
- 已取消子任务及取消原因保留；未完成尝试和子任务转为 `interrupted`，保存结束时间和恢复要求。运行中及排队的任务同样转为 `interrupted`，第二次打开不再次改变恢复状态。
- 验证记录及其文件内容、摘要保留在目标项目的证据目录。
- 应用目录外的任务明确列在未转换清单中，保留原事件及 `FILE_OUTSIDE_APPLICATION` 原因；不能排入执行或直接继续。
- 测试将原宿主和两个原项目目录改名，确保原路径不可用；激活失败后和最终重开后均比对迁移归档清单，最终还比对原宿主数据清单，结果一致。

工具探测使用测试期间由 `rustc` 编译的独立原生夹具，分别复制为 Codex、Godot、Blender 和 Node 的可执行文件，仅接受 `--version`，记录实际调用并返回各自版本。工具绑定执行真实子进程和版本校验，但未使用本机安装的这些工具，也未调用模型或引擎。此记录只证明 Core 迁移入口及项目路由集成，不覆盖 Tauri UI、真实安装工具兼容性或原生应用全流程验收。

## 源码范围

- [迁移激活测试入口](../native/core/src/project_migration_activation_tests.rs)：接入新的重开测试模块，116 个有效行。
- [迁移激活与重开回归](../native/core/src/project_migration_reopen_tests.rs)：失败重试、实际激活、隔离路由、两次重开和源数据比对，244 个有效行。
- [旧历史夹具](../native/core/src/project_migration_reopen_history.rs)：通过现有审批、交付、子任务动作和冻结输入模块生成小型历史样本，165 个有效行。
- [原生版本探测夹具](../native/core/tests/fixtures/migration_tool_probe.rs)：受限的版本探测进程，14 个有效行。

本次只新增集成回归及测试夹具，没有修改生产迁移规则、恢复校验或版本元数据。

## 实际检查与证据

所有 Cargo 检查在同一正常 PowerShell/MSVC 环境执行并保留 `LIB`。证据目录为 [output/f9-migration-activation-20260926-192812](../output/f9-migration-activation-20260926-192812)。

1. `rtk proxy cargo test --locked --manifest-path native/core/Cargo.toml --lib project_migration`：首次执行 29 项，既有 28 项通过，新增用例在夹具准备阶段失败。原因是重开子任务后缺少生产校验要求的 `recoveryNote`。为夹具补充实际检查说明，保留生产校验；见 [首次日志](../output/f9-migration-activation-20260926-192812/core-migration-tests-r1.log)。
2. `rtk proxy cargo test --locked --manifest-path native/core/Cargo.toml --lib actual_activation_retries_tools_and_reopens_preserved_history_without_source_paths`：修正后新增用例 1 项通过，581 项被过滤，执行耗时 2.32 秒，编译及测试退出码 0。既有 28 项未受夹具修正影响，沿用其已通过证据；见 [重开回归日志](../output/f9-migration-activation-20260926-192812/core-reopen-tests-r2.log)。编译仍报告 `object_metadata_tests.rs` 的 3 项既有 `unused_mut` 警告。
3. 四个变更 Rust 文件执行 `rtk proxy rustfmt --edition 2021 --config skip_children=true`，随后同范围 `--check` 通过；见 [Rust 格式日志](../output/f9-migration-activation-20260926-192812/source-format.log)。
4. `rtk proxy npm run check:effective-lines`：1182 个源文件，17 个未改动历史文件，0 项违规；未修改结构 baseline。见 [结构日志](../output/f9-migration-activation-20260926-192812/effective-lines.log) 和 [本次结构报告](../output/f9-migration-activation-20260926-192812/effective-code-lines.json)。
5. 本文及实施计划的 Prettier 检查、`git diff --check`、六个变更文件的严格 UTF-8、无 BOM、行尾空白及文档链接检查通过；见 [交付检查日志](../output/f9-migration-activation-20260926-192812/delivery-checks.log)。

最初一次日志捕获因 PowerShell 将 Cargo 的 stderr 视为终止错误而中止，留下空的 `core-migration-tests.log`；该文件不作为测试证据。随后捕获保留完整 stderr 并使用实际退出码判断结果。

## 剩余工作

本次关闭 F9.3 的实际激活及重开证据缺口，未完成该条目要求的已替代重复写入口清理，也未穷举其他历史版本的语义转换。后续先核对仍可调用的旧写入口、当前消费者及保留理由，再确定移除范围；不能据本次测试直接删除旧兼容路径。

F8.5 的旧拓扑重新定位、F9.1/F9.2 的其他生产路径核对、F9.4 的其他传输与模型工具边界、F9.5 的整体完成记录仍按[实施计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)保留。此次未构建或发布新的原生 EXE，未执行正式验收，未提交或推送代码；全部框架完成仍需逐项满足计划条件。
