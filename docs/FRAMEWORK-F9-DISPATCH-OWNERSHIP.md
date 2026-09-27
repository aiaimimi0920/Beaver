# F9.3：项目创建与回调查询入口清理

日期：2026-09-26。状态：本次入口清理及定向验证完成；F9.3 和 F9 总体仍未完成。

## 结果与范围

完成条件是：移除已经被接管的项目创建分支和重复回调查询分支，保留项目本地存储、旧任务查询和公开参数校验行为。项目创建继续由工作流入口执行，回调状态统一经过项目路由读取；本次不改变业务协议、迁移格式或历史记录。

`business_routing_dispatch` 已在通用数据分发之前将 `project.create` 交给 `workflow_runtime::call`。该入口调用 Core 创建项目，再打开项目本地 runtime，并处理可选 NPR 安装。本次删除 `data_dispatch` 中不可达的旧创建分支，以及只属于该旧分支的调度唤醒配置；工作流入口自己的变更通知保留。

`task.callbackState` 原先同时出现在业务路由、项目数据路由及最终宿主分支。本次保留已有的项目数据路由，删除另外两个重复分支，并把公开参数校验移入唯一的 `task_callback_state` helper，确保先校验再解析存储。缺失参数、非对象参数、错误字段类型和未知字段仍返回原公开入口的错误文本；查询不唤醒调度器、不发出变更通知。

新增回归证明：项目本地回调 revision 和指定请求回执可以读取；即使宿主存在同名任务，关闭项目 runtime 后仍拒绝回退；尚未迁移、没有项目本地数据库的旧项目仍可读取宿主中的历史回调回执。已有的宿主影子隔离、任务创建、项目状态和路由回归同时通过。

保留 `project.import`，它仍由数据分发器实际处理。保留旧任务的宿主回退和回调写操作，它们仍有当前消费者；本次没有将这些路径判定为失效实现。迁移激活与两次重开的证据沿用[前一批记录](FRAMEWORK-F9-MIGRATION-ACTIVATION.md)，未重复运行该批检查。

## 源码范围

- [业务入口](../native/desktop/src/business_routing_dispatch.rs)：删除独立回调查询分支，保留项目创建的工作流路由，366 个有效行。
- [数据分发](../native/desktop/src/data_dispatch.rs)：保留项目回调查询，删除最终匹配中的重复创建及宿主回调查询，357 个有效行。
- [查询实现](../native/desktop/src/data_dispatch_tasks.rs)：合并公开参数校验与存储选择，385 个有效行。
- [调用副作用](../native/desktop/src/business_effects.rs)：删除旧创建分支的调度配置，100 个有效行。
- [测试入口](../native/desktop/src/data_dispatch_tests.rs)：接入回调回归模块，466 个有效行。
- [回调回归](../native/desktop/src/data_dispatch_callback_tests.rs)：参数错误、项目回执、关闭后拒绝回退和旧任务兼容，85 个有效行。

## 实际检查与证据

Cargo 检查均在正常 PowerShell/MSVC 环境执行，保留 `LIB`。证据目录为 [output/f9-dispatch-ownership-20260926-195111](../output/f9-dispatch-ownership-20260926-195111)。五个既有源文件的修改前副本保存在 [before](../output/f9-dispatch-ownership-20260926-195111/before)，本次差异按这些副本核对，避免把既有工作区改动计入本批。

1. 修改前执行 `rtk proxy cargo test --locked --manifest-path native/desktop/Cargo.toml task_callback_state`：1 项通过，158 项过滤，编译及测试退出码 0；见 [基线日志](../output/f9-dispatch-ownership-20260926-195111/callback-before.log)。
2. 修改后执行 `rtk proxy cargo test --locked --manifest-path native/desktop/Cargo.toml -- data_dispatch::tests business_routing::tests workflow_runtime::tests business_effects::tests backend::tests`：57 项通过，104 项过滤，编译及测试退出码 0，测试耗时 1.18 秒。其中数据分发 33 项、业务路由 20 项、项目调用日志 2 项、工作流 runtime 1 项、副作用 1 项；见 [Desktop 日志](../output/f9-dispatch-ownership-20260926-195111/desktop-focused.log)。
3. 执行 `rtk proxy cargo test --locked --manifest-path native/core/Cargo.toml --lib projects::tests::create_embedded_templates_preserves_bytes_and_initial_plan`：1 项通过，581 项过滤，退出码 0，测试耗时 0.11 秒。覆盖 blank/nightbar 模板字节、初始规划、宿主登记及项目本地记录，并保留无效路径和模板的拒绝行为；见 [Core 日志](../output/f9-dispatch-ownership-20260926-195111/core-project-create.log)。
4. 六个变更 Rust 文件执行 `rtk proxy rustfmt --edition 2021 --config skip_children=true`，随后同范围 `--check` 通过。
5. `rtk proxy npm run check:effective-lines`：1183 个源文件、17 个未改动历史文件、0 项违规，未修改 baseline；见 [结构日志](../output/f9-dispatch-ownership-20260926-195111/effective-lines.log)及[结构报告](../output/f9-dispatch-ownership-20260926-195111/effective-code-lines.json)。
6. 本文和实施计划的 Prettier、局部链接、八个变更文件的严格 UTF-8/无 BOM/行尾空白以及 `git diff --check` 通过；见 [交付检查](../output/f9-dispatch-ownership-20260926-195111/delivery-checks.log)。

编译仍报告既有警告：Core `validation/service.rs` 的未使用字段、`object_metadata_tests.rs` 的三项 `unused_mut`，以及 Desktop `game_runtime.rs` 的 `unused_mut`。本次未修改这些文件。

## 证据边界与后续入口

本批证明修改后的 Rust 编译、项目创建 Core 契约和 Desktop 路由相关行为。项目创建的公开工作流分流与 NPR 调用链经过源码核对；本批没有通过 Tauri UI 启动完整创建或安装流程，也没有构建原生开发版 EXE、调用模型或引擎、进行全流程验收。

F9.3 已补齐一组明确失效分支的清理，其他写入口仍需按当前消费者继续审计，其他历史版本的语义转换也未穷举。F9.1/F9.2 的其他生产路径、F9.4 的剩余传输边界、F9.5 的原生交付及完成记录，以及 F8.5 的旧拓扑重新定位仍以[实施计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)为准；本次不勾选 F9.3 或 F9，不变更版本、不提交或推送代码。
