# Beaver 阶段交付与用户审批

日期：2026-09-14。本文保留第二批框架功能及其定向验证的历史记录，包括可选线性阶段、固定候选文件、用户审批、暂停恢复、历史查看和最终合入限制。后续第三批已完成真实 Codex / Blender / 原生宿主最小闭环，包含一次报告目录退回整理后的成功合入，见 [原生验证记录](ASSET-DELIVERY-NATIVE-VALIDATION.md)。最新接口、完成状态和验收边界见 [通用框架契约](WORKFLOW-FRAMEWORK.md)和[开发计划](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md)。

内部版本保持 `0.1.19.1`。第二批未生成发布包；第三批已重新编译 `target/release/Beaver.exe`，包含交付功能及纯资产验证范围修复，未替换用户发布目录中的包。

## 本轮可用流程

1. 受 Beaver 管理的 Codex 在尚无资产阶段时，用 `beaver_task.deliveryPlan` 定义有版本的线性模板。实际动态工具名是 `beaver_task`，操作名放在 `operation` 字段中。
2. Codex 执行当前阶段，保存真实文件，再用 `submitDelivery` 提交。Beaver 捕获文件副本、计算 SHA-256，并将任务停在等待审批状态。
3. 用户在资产任务窗口的“阶段交付”页查看候选文件和历史，批准或填写意见后退回。批准才解锁下一阶段；退回恢复当前阶段制作。
4. 需要修改已批准阶段时，先中止当前执行，选择仍有效的已批准候选，填写意见并点击“从此阶段返工”。受影响的批准失效，历史保留。
5. 所有阶段批准后，Codex 仍需完成资产 round。Beaver 核验最终文件及既有完成条件，再执行正常合入。

这是显式启用的交付模式。历史任务和未配置 `deliveryPlan` 的资产任务保留原流程；第二批没有新增任务创建参数或模板编辑器，也没有写死 NPR 七阶段模板。后续通用框架已增加插件安装、启用、版本和调用的统一适配协议，具体插件适配器另行开发。

## 状态、版本与恢复

模板包含 2-20 个阶段，必须在创建任何阶段之前配置。模板 ID、正整数版本、阶段 ID、名称、顺序及依赖在任务内固定。首个阶段进入 `running`，其余为 `pending`。已有未解决反馈或规划任务不能直接切换为该模式。

`asset.delivery.approved` 保存依次批准的候选 ID；其长度定位当前阶段。`pending` 指向唯一待审批候选，`history` 保留全部提交的候选 ID，最多 200 个，达到上限后拒绝新提交。两个资产阶段入口共用限制：仅当前未提交阶段可更新，模型不能填写 `completed` 来批准阶段。

| 动作           | 阶段和任务变化                                                            | 约束                                                                   |
| -------------- | ------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| 提交候选       | 当前阶段 `checking`；任务 `awaitingInput`，设置 `waitingDelivery`         | 当前执行身份、回调 revision、资产 revision、上游 ID 和实际文件均须有效 |
| 批准 `approve` | 当前阶段 `completed`，下一阶段 `running`；任务 `queued`                   | 必须是当前待审批候选，且固定文件与工作副本通过哈希核验                 |
| 退回 `reject`  | 当前阶段回到 `running`；任务 `queued`                                     | 必须是当前待审批候选，并填写修改原因                                   |
| 返工 `reopen`  | 选中阶段及下游批准失效，选中阶段 `running`，下游 `pending`；任务 `queued` | 无待审批候选，选中候选仍在有效批准列表内，并填写原因                   |
| 最终完成       | 进入既有 round 完成检查和任务合入流程                                     | 所有阶段已批准，最终变更文件与批准版本一致                             |

审批只允许任务处于 `awaitingInput`、`interrupted` 或 `failed`，且没有未回答的澄清问题。正在排队或运行的任务须先正常中止；已完成任务的返工尚不支持。有待审批候选时须先处理该候选，才能返工更早阶段。普通 `task.continue`、调度器和 `autoAccept` 均不能绕过待审批交付。

提交成功后，当前执行身份还可以查询状态、回执以及原样重试已有请求；新的变更被拒绝。执行器收到暂停结果后结束当前 turn，等待标记也会阻止结束路径提前合入。owner 决定先等待旧执行器停止，再核验和提交状态，并由现有调度器恢复执行；接口不递归启动并等待另一轮 Codex。

退回和返工会保存原因、清除受影响阶段的 evidence 与 objects，并把恢复说明追加到任务上下文。候选和决定历史保留。工作副本及 Blender 当前场景不会自动回滚为旧候选；Codex 恢复时必须读取新状态、检查文件和场景，再开始当前阶段工作。交付模式使用候选绑定的 reject / reopen 意见，旧观察反馈提交入口会拒绝新反馈。

有两套独立版本：回调 `expectedRevision` 控制回调日志，`assetRevision` 控制资产状态。owner 的 `assetTask.deliveryDecide.expectedRevision` 是资产 revision。每次都读取实际版本；下文数字只描述一个没有其他变更的全新任务。

回调去重沿用 [任务回调协议](TASK-CALLBACKS.md)：相同任务、执行身份、请求 ID 和 JSON 内容的重试返回原回执，冲突内容或新 turn 复用 ID 会被拒绝。owner 决定按任务和请求 ID 持久去重，完全相同的请求返回原结果，即使任务已推进；修改参数需新请求 ID 和当前资产版本。去重范围限于 Beaver 业务写入，外部 Blender 相对操作仍须核对是否已应用。

## 接口示例

受管理 Codex 首先通过 `beaver_task` 调用 `{ "operation": "state" }`。符合条件时定义最小两阶段模板：

```json
{
  "operation": "deliveryPlan",
  "requestId": "plan",
  "expectedRevision": 0,
  "assetRevision": 0,
  "template": {
    "id": "two-stage",
    "version": 1,
    "stages": [
      { "id": "design", "name": "Design" },
      { "id": "model", "name": "Model" }
    ]
  }
}
```

保存 `design.md` 后提交第一阶段。回调不会替 Codex 创建该文件：

```json
{
  "operation": "submitDelivery",
  "requestId": "design-submit",
  "expectedRevision": 1,
  "assetRevision": 1,
  "stageId": "design",
  "inputCandidates": [],
  "paths": ["design.md"],
  "summary": "Saved design candidate"
}
```

成功返回包含 `candidateId`、`paused: true`、`acceptance: "awaitingOwnerReview"` 和更新后的 revision。后续阶段的 `inputCandidates` 必须与状态中的整个有序 `asset.delivery.approved` 列表完全一致。候选清单保存任务、阶段、模板 ID / 版本、上游候选 ID、文件路径及 SHA-256、摘要、thread / turn、可用的 Blender session ID 和时间。

用户操作复用 [业务 API / MCP](BUSINESS-API.md)。鉴权 `POST /v1/call` 的批准请求如下，任务和候选 ID 替换为查询返回的真实值：

```json
{
  "method": "assetTask.deliveryDecide",
  "input": {
    "id": "actual-task-id",
    "requestId": "unique-owner-request",
    "expectedRevision": 2,
    "candidateId": "actual-candidate-id",
    "decision": "approve",
    "note": ""
  }
}
```

`decision` 只接受 `approve`、`reject`、`reopen`；`note` 字段必须提供，批准时可为空，退回和返工时须为非空白原因。候选摘要和决定原因最多 12000 个 UTF-8 字节，界面意见框最多 4000 个字符。未知字段会被拒绝，不提供 force-approve 开关。

| owner 方法                 | 输入                        | 返回                                                                    |
| -------------------------- | --------------------------- | ----------------------------------------------------------------------- |
| `assetTask.deliveryState`  | `id`                        | 资产 `revision`、`workflow`、从新到旧的 `candidates` 和决定 `decisions` |
| `assetTask.deliveryFile`   | `id`、`candidateId`、`path` | 经核验的固定文件 `{path, sha256, base64}`                               |
| `assetTask.deliveryExport` | `id`、`candidateId`         | 新导出目录 `{path}`                                                     |
| `assetTask.deliveryDecide` | 上述决定字段                | `{candidateId, decision, assetRevision, status: "queued"}`              |

owner 方法通过桌面、HTTP 和业务 MCP 进入相同适配层。现有本机 owner token 具有应用级业务权限；本轮没有新增任务级 HTTP 授权。受管理 `beaver_task` 工具不提供 owner 决定操作，也不注入全局 owner token。不要给需要自我审批隔离的受管理模型额外配置 owner 业务 MCP。

## 文件、观察与核验边界

每个候选提交 1-32 个不同的工作副本相对文件路径，使用正斜杠。文件必须为非空普通文件，每个不超过 256 MiB，合计不超过 512 MiB。路径穿越、链接及内部工作目录文件会被拒绝。`.blend` 所需外部贴图、依赖、导出、预览和报告需显式列入；当前不解析 Blender 依赖。

文件副本复用现有内容寻址 blob 库，候选清单与任务绑定。读取固定文件时再次核验 SHA-256；每次导出创建 native 数据目录 `delivery-exports` 下的独立目录，核验复制后的字节，保留原相对路径。导出目录是用户可打开的副本，后续修改不会改变已保存候选。

界面内读取和下载上限为 16 MiB，更大文件使用“导出全部候选文件”。界面支持常用光栅图片及文本预览，文本最多展示 512 KiB；SVG / HTML 不作为可执行页面打开。候选切换和组件卸载后忽略旧文件请求的返回。“输入与执行身份”显示上游候选及会话标识，历史候选不会获得当前候选的批准按钮。

左侧实时观察仍可能包含未保存修改。“阶段交付”展示提交文件副本，候选中的预览图片是否真实对应 `.blend` 内容尚无语义验证。固定文件和哈希提供字节身份依据，无法直接证明建模质量、拓扑正确性或视觉达标。

提交文件捕获及审批文件核验都在数据库互斥锁外执行；写入前重新检查身份、状态和 revision，候选、回执、等待标记和事件原子提交。批准时核验此前有效批准文件与当前候选的叠加清单，后阶段对同路径的文件可覆盖前阶段版本。

审批核验发生在旧执行器停止之后，但只是该时点的文件检查，数据库互斥锁无法阻止外部 Blender 或其他程序再次写盘。最终合入前会再次核验有效批准文件，并要求所有最终变更的路径与 after 哈希均属于有效批准清单；未提交变更、文件漂移和删除会失败，保留任务工作副本。现有结构检查、round 完成规则及合并冲突保护继续生效。审批不会证明所有外部工具都已停止，也不提供操作系统级文件隔离。

## 代码入口

- [线性阶段约束](../native/core/src/asset_delivery.rs)：模板、当前阶段和输入版本。
- [候选文件](../native/core/src/asset_delivery_files.rs)：捕获、读取、导出和最终哈希门槛。
- [审批状态机](../native/core/src/asset_delivery_review.rs)：决定去重、核验准备、原子批准与返工。
- [回调适配](../native/core/src/task_callback_runtime.rs) 和 [协议](../native/core/src/task_callback_contract.rs)：锁外文件捕获及 protocol 2。
- [桌面业务适配](../native/desktop/src/asset_delivery_runtime.rs)：停止旧执行器、owner 核验及调度唤醒。
- [阶段交付面板](../src/ui/asset-task/DeliveryPanel.tsx) 和 [文件面板](../src/ui/asset-task/DeliveryFiles.tsx)：候选、固定预览与决定操作。
- [Codex 指令](../resources/instructions/task-callback.md)：提交后停止制作、恢复检查和最终交付要求。

## 验证记录

以下均为开发期定向验证，未运行完整 NPR 创作或正式发布流程。日志位于 [output/asset-delivery](../output/asset-delivery/)。同一测试在修复后重复运行只计一次，下面不累加为独立案例数。

| 范围                 | 实际命令                                                                                                             | 最新结果与证据                                                                                                                                                                    |
| -------------------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 线性交付及文件完整性 | `rtk cargo test --locked -p beaver-core --test asset_delivery --test asset_delivery_integrity`                       | 7 项通过，[delivery-final.log](../output/asset-delivery/delivery-final.log)                                                                                                       |
| 回调回归及完整性     | `rtk cargo test --locked -p beaver-core --test asset_delivery_integrity --test task_callback`                        | 10 项通过，含上述 4 项完整性测试，[core-integrity-final.log](../output/asset-delivery/core-integrity-final.log)                                                                   |
| 动态工具 RPC         | `rtk cargo test --locked -p beaver-core executor_tools::tests`                                                       | 4 项通过，含提交后 `AwaitingInput` 分发，[rpc-tests.log](../output/asset-delivery/rpc-tests.log)                                                                                  |
| 文件捕获             | `rtk cargo test --locked -p beaver-core --lib files::tests`                                                          | 4 项通过，[files-final.log](../output/asset-delivery/files-final.log)                                                                                                             |
| 任务结束路径         | `rtk cargo test --locked -p beaver-core --lib task_finish::tests`                                                    | 6 项通过，[finalizer-final.log](../output/asset-delivery/finalizer-final.log)                                                                                                     |
| 桌面目录 schema      | `rtk cargo test --locked -p beaver-desktop business_catalog::tests`                                                  | 4 项通过，含整数资产 CAS 和无强制批准参数，[desktop-catalog.log](../output/asset-delivery/desktop-catalog.log)                                                                    |
| MCP 传输回归         | `rtk cargo test --locked -p beaver-desktop business_mcp::tests`                                                      | 1 项通过，[desktop-mcp.log](../output/asset-delivery/desktop-mcp.log)                                                                                                             |
| 既有资产契约         | `rtk cargo test --locked -p beaver-core --test asset_contract --test asset_delivery --test asset_delivery_integrity` | 该次日志中 20 项 asset_contract 通过；后续完整性路径断言失败已修复并按首行重跑通过，不把旧组合日志称为全部通过，[core-contracts.log](../output/asset-delivery/core-contracts.log) |
| 前端类型             | `rtk npm run typecheck`                                                                                              | 通过，[typecheck-final.log](../output/asset-delivery/typecheck-final.log)                                                                                                         |
| 浏览器组件           | Playwright CLI 挂载实际 DeliveryPanel / DeliveryFiles，模拟 `window.beaver.call`                                     | 历史切换、固定文本、导出、错误展示和相同请求重试通过；[组件证据](../output/playwright/asset-delivery/PROOF.md)                                                                    |

[交付回归](../native/core/tests/asset_delivery.rs) 覆盖线性推进、文本完成拒绝、重复和冲突请求、暂停、普通继续拒绝、上游版本、退回 / 重提 / 返工及候选历史。[完整性回归](../native/core/tests/asset_delivery_integrity.rs) 使用缺失、空、越界、重复、内部、超大文件和损坏 blob，验证失败不消耗请求，跨任务读取被拒绝，数据库触发器故障回滚，并发相同提交只产生一个候选，旧审批 CAS 被拒绝。实际 finalizer 样本检查有效批准、漂移、额外变更和删除，只有批准版本合入项目。

最终检查已完成：

- `rtk cargo check --locked -p beaver-desktop` 通过，覆盖更新后的内嵌回调指令；[编译日志](../output/asset-delivery/desktop-check-final.log)。本项为编译检查，未生成新的发布包。
- `rtk proxy cargo fmt --all -- --check` 返回 0；本轮相关 TS / TSX / CSS、文档和指令的 Prettier 检查通过，见 [格式日志](../output/asset-delivery/format-check-final.log)。
- `rtk npm run check:effective-lines` 通过：475 个源文件、18 个未修改历史超限文件、0 个违规；未更新基线，见 [结构日志](../output/asset-delivery/effective-lines-final.log)。
- `git diff --check` 返回 0。工作区变更及未跟踪文件中，排除 `graphify-out` 和 `output` 后的 61 个源码、文档和配置文件均通过严格 UTF-8 解码且无 BOM；本轮 7 份交付相关文档的本地链接和标题锚点有效。测试日志保留其原始编码。

新增核心模块的有效行数为：阶段约束 134、候选文件 196、审批状态机 173、回调运行适配 93；桌面交付适配为 82。界面按候选审批和文件查看分开，`DeliveryPanel.tsx` 为 202，`DeliveryFiles.tsx` 为 127。各模块职责及入口见上节，未将状态、文件操作和界面集中到一个大文件。

## 剩余工作与下一批交付

第二批核心测试使用真实 SQLite、临时文件和任务 finalizer；RPC / MCP 与 UI 使用测试夹具，浏览器组件导出只验证请求和返回展示。第三批随后通过原生 API 验证真实 Codex 回调和 Blender 制作，查看冻结 PNG、独立解析 GLB，并复核原生导出和项目合入的实际磁盘哈希。待审批宿主重启恢复已验证，未保存 Blender 场景恢复及完整原生窗口联动仍未覆盖。历史测试结果不累加为新的独立案例。

下一批先推进动态资产子任务及 attempt 与工具操作关联，随后逐项补齐源依赖发现、语义及视觉验收、Godot / Codex / Blender 插件生命周期。第三批的资产范围规则仍较保守：普通 JSON 报告放在资产目录会触发代码验证，该轮通过 owner 退回将报告整理到 docs 后完成；没有实现任意产物类型的自动检查器选择。长文件操作目前有大小上限并直接等待，尚无可查询、可取消的 operation 队列。

框架达到 [D1-D8 完成条件](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md#4-实施步骤与完成条件) 后，再逐个迭代 NPR 制作小步骤，结合调研和可靠网络资料选择工具调用链，每步提交可检查的产物并等待用户批准。
