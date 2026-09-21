# Beaver 与 Codex 通用框架：本轮完成记录

日期：2026-09-14。

本轮约定范围内的框架代码已完成，相关编译检查和 50 项功能测试通过，另有模拟 Codex 执行器的三种线程协议场景通过。现在可以继续讨论用户新的框架建议。

本次按用户要求只完成通用框架：没有开发具体 NPR 建模小阶段、专用 Skill 或第三方插件调用链，没有进行新的原生交付验收或全流程测试。本轮未重新构建或替换用于交付的 `Beaver.exe`，当前检查结果不代表运行中的应用已更新。内部版本保持 `0.1.19.1`，Rust 包版本保持 `0.1.19`。

## 已完成的代码

Beaver 保存任务、线性阶段、子任务、执行尝试、候选文件和审批状态，Codex 在当前执行身份下理解需求、提出工作、调用工具并提交证据。桌面、HTTP 和业务 MCP 共用 owner 业务入口；受管理 Codex 通过 `beaver_task` 和 `beaver_workflow` 回调。协议升级到版本 5，旧线程会更新工具注册并保留历史身份。

1. **双向接口与权限。** `task.framework` 和 `beaver_workflow` 已接入同一核心实现。模型调用绑定 task / thread / turn，不能修改适配器配置、削弱规则或批准阶段；owner 决定独立保存。等待审批期间允许读取原执行的状态和回执，禁止继续制作。
2. **持久长操作。** 启动、查询、取消和请求去重已落库；每个任务最多一个未结束 operation。文件与进程 IO 在数据库锁外执行，返回后核验身份、状态和版本，过期结果保留为 stale。取消、turn 结束、关闭及重启有明确终态；重启不重放外部副作用，并修复内层 callback 已提交但外层回执未完成的间隙。
3. **通用插件适配。** 支持 Godot、Codex、Blender 的 probe / install / enable / reload 命令配置，固定可执行文件和显式资源哈希，设置超时并读取结构化回执。安装、启用和重载后独立 probe，身份、版本、兼容、启用和可调用状态全部符合才标记 ready。
4. **版本化检查与视觉评价。** 检查冻结候选及上游输入，验证规则身份、版本和导出字节，原子保存报告与 operation 终态。owner 与 model 对冻结 PNG 的评价分别保存；开启视觉门槛时要求当前 owner pass。技术失败不能被人工接受、autoAccept 或普通继续操作绕过。
5. **输入、历史与最终合入。** 每次 attempt 的历史输入可独立导出，同路径的不同版本分别保留。最终合入复核全部批准输出及依赖；后续已批准覆盖允许通过，未审批漂移拒绝。提供已保存 `.blend` 文件的通用外部引用扫描入口。
6. **可观察性与恢复界面。** Codex 实际工具事件写入持久轨迹，开始事件固定归属；缺失事件和中断明确记录。配置、插件状态、操作取消、检查、评价、输入导出、依赖扫描和恢复历史接入资产窗口。修复任务切换、卸载及晚到异步结果覆盖当前状态的竞态。
7. **接入文档。** 运行时指令、业务 API、开发计划和文档索引已更新。Skill 的输入、输出、失败、证据及恢复约定见 [通用框架契约](WORKFLOW-FRAMEWORK.md)，后续无需把制作阶段写死进框架。

## 主要代码位置

| 职责                     | 入口                                                                                                                  |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------- |
| 核心请求、状态和操作     | `native/core/src/framework.rs`、`framework_contract.rs`、`framework_operations.rs`、`framework_jobs.rs`               |
| 命令固定与插件状态       | `native/core/src/framework_command.rs`、`framework_plugins.rs`                                                        |
| 检查、评价和历史输入     | `native/core/src/framework_checks.rs`、`framework_judgments.rs`、`framework_inputs.rs`                                |
| 实际工具轨迹             | `native/core/src/framework_evidence.rs`、`executor.rs`、`call_log.rs`                                                 |
| 任务回调、冻结文件与审批 | `native/core/src/task_callback*.rs`、`asset_delivery*.rs`、`asset_work*.rs`                                           |
| 桌面 / HTTP / MCP 集成   | `native/desktop/src/asset_task_runtime.rs`、`asset_delivery_runtime.rs`、`business_routing.rs`、`business_catalog.rs` |
| 前端契约和资产窗口       | `src/shared/framework.ts`、`src/ui/asset-task/Framework*.tsx`、`use-framework.ts`、`DeliveryPanel.tsx`                |
| 通用依赖扫描             | `resources/workflows/blender_dependencies.py`                                                                         |
| Codex 运行时协议         | `resources/instructions/task-callback.md`                                                                             |

## 本轮定向验证

命令在仓库根目录执行，外部命令通过 `rtk proxy`，Cargo 使用相同的正常 PowerShell / MSVC 环境。下表省略重复的 `rtk proxy` 前缀。日志位于 [output/framework-completion](../output/framework-completion/)。只运行改动涉及的功能组，没有执行 `npm test`、完整原生验收或完整建模流程。

| 命令或检查                                                                                                                                                                | 结果                                                     | 日志                                                        |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ----------------------------------------------------------- |
| `cargo check --locked -p beaver-core`                                                                                                                                     | 通过                                                     | `core-check-2.log`                                          |
| `cargo check --locked -p beaver-desktop`                                                                                                                                  | 通过                                                     | `desktop-check.log`                                         |
| `cargo test --locked -p beaver-core --test framework_operations --test framework_validation`                                                                              | 12 通过；2 个子进程 fixture 入口在常规枚举中 ignored     | `framework-tests-2.log`                                     |
| `cargo test --locked -p beaver-core --test framework_evidence`                                                                                                            | 4 通过；1 个子进程 fixture 入口在常规枚举中 ignored      | `evidence-tests-2.log`                                      |
| `cargo test --locked -p beaver-core --lib executor_tools::tests::`                                                                                                        | 5 通过                                                   | `executor-tools-tests.log`                                  |
| `cargo test --locked -p beaver-core --test task_callback --test asset_delivery --test asset_delivery_integrity --test asset_work_dependencies --test asset_work_recovery` | 22 通过                                                  | `regression-tests.log`                                      |
| `cargo test --locked -p beaver-desktop business_mcp::tests::callback_mcp_stream_http_core_roundtrip`                                                                      | 1 通过                                                   | `desktop-mcp-tests.log`                                     |
| `cargo test --locked -p beaver-desktop business_catalog::tests`                                                                                                           | 4 通过                                                   | `desktop-catalog-tests.log`                                 |
| `npx tsx --test tests/asset-work-panel.test.ts`                                                                                                                           | 2 通过                                                   | `ui-tests.log`                                              |
| `npm run typecheck`                                                                                                                                                       | 通过                                                     | `typecheck-final.log`                                       |
| `cargo run --locked -p beaver-core --example executor_contract -- output/framework-completion/executor-callbacks --callbacks`                                             | 新线程、兼容线程恢复、旧协议线程升级三种场景通过         | `executor-callbacks.log`、`executor-callbacks/proof.json`   |
| `cargo fmt --all -- --check`                                                                                                                                              | 通过                                                     | `rust-format-final.log`                                     |
| Prettier 检查本轮 shared / UI / 组件测试文件                                                                                                                              | 通过                                                     | `ui-format-final.log`                                       |
| Prettier 检查本轮框架文档和协议指令                                                                                                                                       | 通过                                                     | `docs-format-final.log`                                     |
| `npm run check:effective-lines`                                                                                                                                           | 通过；报告 `ok: true`、`violations: []`，未调整 baseline | `effective-lines-final.log`、`../effective-code-lines.json` |
| `git diff --check`、新增/修改文本 UTF-8 无 BOM、相关文档本地链接                                                                                                          | 通过                                                     | 本轮收尾工具结果                                            |

上述功能组共 48 项 Rust 测试与 2 项 TypeScript 测试通过。子进程 fixture 由父测试显式调用，不计入 50 项顶层测试；协议示例单独统计。

测试覆盖了请求重试和执行身份、真实子进程取消、turn 结束及启动恢复、数据库故障时的原子性、过期结果、版本与资源固定、输入篡改、插件就绪判定、owner 图片评价、同路径历史输入、最终依赖覆盖与未批准漂移、工具事件归属和敏感输出摘要。模拟执行器核验两种任务工具的注册、分发和旧线程升级。

MCP 定向测试使用内存流连接和真实 loopback HTTP 调用核心业务，未启动原生桌面窗口。执行器证明明确记录 `realModelTaskVerified: false`。前端结果来自类型检查和组件测试，不能据此宣称新增面板的原生交互已经验收。

## 当前边界与后续衔接

本轮通用代码范围内没有待补实现项。以下事项按当前指令不在本次开发与验证范围内：

- NPR 大阶段下的具体小步骤、角色设计模板、质量规则内容和逐步制作交付。
- 第三方插件的实际选择、下载来源、安装脚本、授权处理及专用 Skill / 工具调用链。框架已提供适配协议，具体能力仍需要配置和实现对应适配器。
- 新原生窗口交互、真实第三方插件和本轮依赖扫描器的真实 Blender 验证，以及最终发布前的完整回归和应用打包。

框架的已知工程边界继续保留：适配器拥有当前用户权限，没有操作系统沙箱；哈希检查存在外部并发写入窗口；取消不能撤销已发生的外部副作用；解释器及传递资源需要显式固定；已保存引用扫描不证明运行时依赖完整；大量历史 operation 的分页和性能优化需要后续实测。详细容量限制和接口约定见 [通用框架契约](WORKFLOW-FRAMEWORK.md#已知边界)。

后续从 [对象创作框架开发设计](OBJECT-CREATION-FRAMEWORK-DESIGN.md) 继续，结合 [原开发计划](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md) 和通用框架契约复用已有能力。新设计的对象模型、统一任务层级、队列及三面板改动尚未实施，不包含在本记录的完成结论中。
