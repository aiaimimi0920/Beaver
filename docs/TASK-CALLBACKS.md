# Beaver 任务回调：首批框架交付

日期：2026-09-14。本文保留首批统一回调的历史实现与测试记录。当前协议已升级为版本 5，最新接口与边界见 [通用框架契约](WORKFLOW-FRAMEWORK.md)。历史批次的线性阶段、固定候选文件和用户批准推进见 [阶段交付说明](ASSET-DELIVERIES.md)；资产子任务与执行尝试见 [执行记录说明](ASSET-WORK-ATTEMPTS.md)；源文件快照、依赖核验与恢复见 [输入清单说明](ASSET-WORK-INPUTS.md)。

## 调用链和职责

```text
Beaver 调度 -> Execution -> Codex thread / turn
                              |
                    动态工具 beaver_task
                              |
                              v
桌面业务调用 ----------> task_callback 业务实现 -> 同一 SQLite
HTTP /v1/call ---------->        ^
业务 MCP -> HTTP ---------------|
```

受管理 Codex 使用动态工具 `beaver_task`，由宿主绑定 task ID，并核验服务器提供的 thread / turn 与当前运行任务一致。模型参数不接受另一个 task ID。原有 `beaver_ask_user`、规划任务的 `beaver_submit_plan` 和资产任务工具继续承担各自职责。

外部 owner 通过既有鉴权 API 或业务 MCP 使用 `task.callback` 和 `task.callbackState`，两者进入同一核心业务实现。owner 仍有本机应用级权限；`task.callback` 要求明确提供当前执行身份。本批没有新增任务级 HTTP 凭据。受管理 Codex 不需要 owner token，也不注入这份全局凭据。

新会话注册回调工具；已记录当前 `callbackToolVersion` 的会话继续恢复。旧任务缺少对应版本时建立新会话，保留 `priorCallbackThreadId`、工作副本及已有结果；旧资产工具迁移保留原有 `priorAssetThreadId`。新会话应先读取状态并核对现场。

## 七个回调操作

| 操作             | 输入                                                                                               | 作用                                                                              |
| ---------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `state`          | `operation`                                                                                        | 查询任务、既有资产阶段、回调 revision 和最近 20 条回执摘要。摘要最多 200 个字符。 |
| `receipt`        | `operation`、`requestId`                                                                           | 查询一条持久回执，包括原始请求与响应；不存在时返回 `receipt: null`。              |
| `report`         | `requestId`、`expectedRevision`、`report`                                                          | 保存进度、检查点声明、结果或阻塞报告；输入和输出须为对象。                        |
| `stages`         | `requestId`、`expectedRevision`、`assetRevision`、完整 `stages`                                    | 调用现有资产阶段业务规则，更新同一份资产状态。                                    |
| `deliveryPlan`   | `requestId`、`expectedRevision`、`assetRevision`、`template`                                       | 创建有版本的线性阶段模板。                                                        |
| `work`           | `requestId`、`expectedRevision`、`assetRevision`、`stageId`、`change`                              | 登记、修订、取消同工作副本的串行子任务，开始或结束执行尝试，以及 reopen 后重做。  |
| `submitDelivery` | `requestId`、`expectedRevision`、`assetRevision`、`stageId`、`inputCandidates`、`paths`、`summary` | 固定候选文件并暂停等待 owner 审批。                                               |

版本 2 还提供 `deliveryPlan` 和 `submitDelivery`：前者在尚无阶段时配置有版本的线性模板；后者核验并保存当前阶段的文件候选，暂停执行等待 owner 决定。完整参数和示例见 [阶段交付说明](ASSET-DELIVERIES.md)。

版本 3 增加 `work`，支持 `create`、`revise`、`cancel`、`begin`、`finish`。宿主分配 subtask / attempt ID，绑定执行身份、检查点及上游候选。提交交付前，当前阶段已登记的子任务必须完成或取消；成功尝试绑定到固定候选。完整请求与恢复约定见 [执行记录说明](ASSET-WORK-ATTEMPTS.md)。

版本 4 要求 `begin` 显式提供 `inputFiles`，`[]` 表示声明无文件输入，历史未采集状态另行保留。Beaver 捕获源文件和依赖的字节与 SHA-256；`source` 允许后续修改，`dependency` 在成功结束、阶段提交和候选审批时复核当前文件。`work.reopen` 可以重开当前阶段已完成的子任务，并失效该步骤及同阶段后续未取消步骤。参数、限制和依赖变更恢复见 [输入清单说明](ASSET-WORK-INPUTS.md)。

所有请求都需要 `operation`。请求上限为 64 KiB，产物使用文件引用，避免内联大文件。字段使用 camelCase；未知请求字段会被拒绝。具体 schema 由 `task_callback_contract.rs` 生成，并随业务能力目录及动态工具发布。

`report.kind` 可取 `progress`、`checkpoint`、`result`、`blocked`；必须提供 `summary`、`inputs` 和 `outputs`。可选 `tools` 用于报告 Skill、工具或插件及其观察到的版本。针对资产阶段报告时，`stageId` 与 `assetRevision` 必须成对提供，且与当前阶段及版本匹配。

## 使用和恢复

受管理 Codex 首先调用 `beaver_task`：

```json
{ "operation": "state" }
```

读取实际 revision 后报告一个步骤。以下文件名仅为协议示例，回调不会创建这些文件：

```json
{
  "operation": "report",
  "requestId": "design-draft-1",
  "expectedRevision": 0,
  "report": {
    "kind": "result",
    "summary": "设计草稿已写入工作副本，等待检查",
    "inputs": { "request": "用户确认的角色方向" },
    "outputs": { "design": "docs/character-design.md" },
    "tools": []
  }
}
```

成功结果包含 `requestId`、新 `revision`、`recorded: true`、当前 `assetRevision`（无资产任务时为 null）和 `acceptance: "notEvaluated"`。回调 revision 与资产 revision 分开维护；报告增加回调 revision，阶段修改同时经过原资产 revision 校验。

若响应丢失，先查询：

```json
{ "operation": "receipt", "requestId": "design-draft-1" }
```

同一任务、同一活跃 thread / turn、相同请求 ID 和相同 JSON 内容的重试返回原响应，不再次增加 revision 或事件。JSON 对象键顺序不影响相等性，省略字段与显式默认值仍可能不同。同一 ID 的不同内容或新 turn 重用会报冲突。旧 turn 在中断、恢复或切换后不能继续读写；新的活跃 turn 可查旧回执，并用新请求 ID 继续工作。

提交候选会将当前执行停在 `awaitingInput`。该执行身份仍可查询 state、receipt 或原样重试已保存请求，新变更会被拒绝；后续 owner 决定清除旧 turn，恢复时重新读取状态。

回执、回调 revision、阶段变更及事件在同一数据库事务提交；交付提交还原子保存候选清单和等待标记。事件只有查询指针和摘要；完整请求与响应保存在既有 `entities` 中，不依赖有界事件历史。默认状态只返回最近 20 条摘要，较早记录可凭请求 ID 读取。

owner 在任务中断后仍可查询。按 [业务 API](BUSINESS-API.md) 使用鉴权 `POST /v1/call`：

```json
{
  "method": "task.callbackState",
  "input": { "id": "实际任务 ID", "requestId": "design-draft-1" }
}
```

owner 提交时使用 `method: "task.callback"`，`input` 包含真实的 `id`、`threadId`、`turnId` 和上述 `request` 对象。MCP 的 `tools/call` 使用相同业务方法名和参数。只有仍在运行的当前执行身份可提交新变更；因交付暂停的原执行身份仍可原样重试其已保存请求。

## 交付语义和观察范围

回调记录来源明确标为 `codexReported`。普通 report 保存模型报告的输入、输出、工具信息和阶段进度；插件确实运行或视觉达标仍需独立证据。`result` 不会批准阶段或结束任务；`checkpoint` 报告不替代真实保存；`blocked` 报告不会自动中断执行。`submitDelivery` 的文件哈希由 Beaver 计算，审批单独记录为 owner 决定。

未启用交付计划时，`stages` 沿用既有 `asset_stages` 规则。启用后两个资产工具共用当前阶段限制，模型不能通过文本完成阶段，只有 owner 批准候选后解锁后续阶段。程序没有写死七阶段 NPR 模板。回执去重只保护 Beaver 数据库写入；断线后的 Blender 相对修改仍须核对场景，不能直接重放。

现有任务会话的“执行记录”显示 `taskCallback` 的请求 ID、操作和摘要，完整输入输出由状态及回执接口提供。资产窗口“阶段交付”页提供候选历史、文件预览与批准操作，并显示只读子任务和 attempt 历史、执行身份、恢复说明及 Codex 报告的输入输出和工具。宿主输入清单单独显示路径、角色和 SHA-256，区分历史未采集与显式空清单。真实工具调用的自动拦截和完整轨迹仍未实现；新增面板已有组件验证，尚无本批原生窗口联动证据。

## 验证记录

以下为首批回调交付当时的记录，保留日志于 `output/task-callback/`。版本 2 的新增和回归验证见 [本轮记录](ASSET-DELIVERIES.md#验证记录)；开发期不运行全流程 NPR 建模或正式发布验收。

| 范围         | 命令 / 证据                                                                                                                             | 结果与覆盖                                                                                                   |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 回调业务     | `rtk cargo test --locked -p beaver-core --test task_callback`；`core-tests.log`                                                         | 6 项通过：持久去重、跨入口重试、重启回执、旧身份拒绝、新 turn 继续、版本冲突、事务回滚、输入限制和摘要查询。 |
| 动态工具分发 | `rtk cargo test --locked -p beaver-core executor_tools::tests`；`rpc-tests.log`                                                         | 3 项通过：子进程 RPC 响应、模型可读的业务错误和失败后继续查询。                                              |
| 资产阶段规则 | `rtk cargo test --locked -p beaver-core --test asset_contract stages::`；`asset-stage-tests.log`                                        | 3 项通过：验证回调直接复用的阶段更新规则。                                                                   |
| 业务能力目录 | `rtk cargo test --locked -p beaver-desktop business_catalog::tests`；`catalog-tests.log`                                                | 3 项通过：包含回调入口的工具目录及 schema。                                                                  |
| 执行器握手   | `rtk cargo run --locked -p beaver-core --example executor_contract -- output/task-callback/executor --callbacks`；`executor/proof.json` | 3 个场景通过：新会话注册、兼容恢复、旧会话升级及执行器到回调落库往返；使用模拟 Codex 进程。                  |
| MCP 传输     | `rtk cargo test --locked -p beaver-desktop business_mcp::tests`；`mcp-tests.log`                                                        | 1 项通过：MCP 流解析、初始化、目录、HTTP 往返和共享业务回调；HTTP 服务使用测试夹具。                         |

共 16 项定向测试及 3 个执行器握手场景通过。上述 Cargo 命令包含相关 Rust 代码的编译验证。`rtk proxy cargo fmt --all -- --check` 通过；`rtk npm run check:effective-lines` 扫描 463 个源文件，0 项违规，18 个未改动的历史文件继续沿用原有基线，本批未修改基线。日志分别为 `rust-format.log` 和 `effective-lines.log`。

4 份本批文档经 Prettier 格式化及检查通过；`git diff --check` 通过。检查的 18 个本批源文件及文档全部为 UTF-8 无 BOM。格式及差异检查日志为 `docs-format-check.log` 和 `diff-check.log`。

以上夹具没有证明真实 Codex、实际桌面宿主 API 中间件和观察 UI 的完整联动，也没有生成或检查真实 Blender 产物。本批未构建或替换供用户运行的 Beaver 发布包，版本号保持原状。

## 后续框架批次

最小两阶段交付已完成 [真实 Codex / 原生宿主联动验证](ASSET-DELIVERY-NATIVE-VALIDATION.md)。当前已增加同任务串行资产子任务、attempt 关联和恢复规则，并完成显式源文件与依赖输入捕获，见 [第五批记录](ASSET-WORK-INPUTS.md)。后续补充依赖发现、语义检查、真实工具追踪及 Godot / Codex / Blender 插件准备与调用记录；新增 attempt 和输入面板仍需原生联动验证。

框架完成后再逐个迭代 NPR 小步骤：资料核验、工具及插件选型、调用链试作、提交产物、用户查看和批准。每批保留可交接记录；总体进度见 [开发计划](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md)。
