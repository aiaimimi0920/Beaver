# 资产子任务与执行尝试：第四批框架交付

日期：2026-09-14。已实现同任务串行资产子任务、执行尝试、阶段候选关联、失败恢复和只读观察面板。Beaver 管理持久状态与执行门禁，Codex 通过协议版本 3 的 `beaver_task` 登记和执行工作。本批相关功能验证通过；整个工作流框架仍未完成，后续缺口见本文末尾和 [开发计划](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md)。

本文保留第四批记录和当时的协议示例。当前调用约定已升级为版本 4，`begin` 必须提供 `inputFiles`，新增依赖核验及 `work.reopen`；当前行为和剩余工作见 [第五批交付](ASSET-WORK-INPUTS.md)。

## 本批行为与边界

资产步骤存放在主任务的 `asset.work` 中，沿用主任务的工作副本、资产能力、反馈和自动强度。一个阶段可包含多个子任务，按登记顺序执行。Beaver 只允许首个未批准阶段修改工作，同一资产工作副本最多存在一个 running attempt。此处不创建额外的调度任务、任务卡片或工作副本；原 `task_plan` 会展开独立代码任务，需要独立工作副本，不适合当前共用 Blender 场景的步骤。

Codex 可以创建、修订和取消当前阶段的子任务，开始一次执行，最后报告完成或失败。宿主分配子任务和 attempt ID，保存定义快照、thread / turn、可用的 Blender session、起止检查点引用、开始时的资产 revision、已批准上游候选、时间和恢复说明。子任务修订的原始请求保留在回执中，每次执行保留当时的定义。

提交阶段候选时，所有已登记且未取消的当前阶段子任务都必须已报告完成，其最近成功 attempt 的上游候选必须仍有效。Beaver 将这些 attempt ID 绑定到冻结的候选文件。用户批准后才能执行下一阶段；子任务完成报告不产生阶段批准。

旧资产状态缺少 `work` 时读取为空；旧候选缺少 `attemptIds` 时读取为空。未登记子任务的历史交付仍可按原规则进行，不伪造执行历史。本批未提供子任务删除、重排、并行写场景或独立子任务审批；阶段 owner 审批继续负责确认交付。

## 协议与最小调用顺序

首先调用 `state`，读取回调 `revision`、`asset.revision`、当前阶段、已有 work 和回执。`deliveryPlan` 必须先存在。下列 JSON 中的 revision 和 ID 仅为示例，实际调用必须取当前状态或上次成功响应。

```json
{
  "operation": "work",
  "requestId": "brief-create-1",
  "expectedRevision": 1,
  "assetRevision": 1,
  "stageId": "design",
  "change": {
    "action": "create",
    "definition": {
      "title": "Character brief",
      "goal": "Record the confirmed character design",
      "acceptance": "Owner can inspect saved design and references"
    }
  }
}
```

响应包含宿主分配的 `subtaskId`、新回调 `revision`、新 `assetRevision`、`recorded: true` 和 `acceptance: "notEvaluated"`。后续 work 请求沿用相同公共字段，并替换 `change`：

| action   | 必需内容                                                | 行为                                                                             |
| -------- | ------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `create` | `definition: {title, goal, acceptance}`                 | 在当前阶段追加子任务，返回 `subtaskId`。                                         |
| `revise` | `subtaskId`、完整 `definition`                          | 修改尚未执行或需重试的定义，保留回执和旧 attempt 快照。                          |
| `cancel` | `subtaskId`、`reason`                                   | 显式取消尚未执行或需重试的步骤，保留记录。                                       |
| `begin`  | `subtaskId`、对象形式的 `inputs`                        | 为首个未完成且未取消的子任务创建新 attempt，返回 `attemptId`。                   |
| `finish` | `attemptId`、`outcome`、`summary`、对象形式的 `outputs` | 结束当前执行；`outcome` 只接受 `completed` / `failed`，可另传 `tools` 对象数组。 |

`running`、`completed`、`cancelled` 子任务不可修订或取消。`begin` 重试 `failed`、`interrupted`、`stale` 子任务时，必须提供非空 `recoveryNote`。一个 task 最多 100 个子任务和 200 个 attempts；单请求仍受 64 KiB 限制。标题最多 300 UTF-8 字节，其余受限说明字段最多 12000 字节；文本须非空，工具数组最多 100 个对象。细碎操作应记录到当前 attempt，无需每个点击都建立子任务。

开始与结束的 `change` 示例：

```json
{
  "action": "begin",
  "subtaskId": "host-assigned-subtask-id",
  "inputs": { "request": "confirmed design", "references": ["refs/front.png"] },
  "recoveryNote": "Inspected saved files and the scene before retry"
}
```

```json
{
  "action": "finish",
  "attemptId": "host-assigned-attempt-id",
  "outcome": "completed",
  "summary": "Saved design for owner review",
  "outputs": { "brief": "docs/design.md" },
  "tools": [{ "name": "selected-skill", "version": "observed-version" }]
}
```

公共请求中的回调 revision 和资产 revision 都要匹配。宿主同时校验 task 的当前 thread / turn；finish 还必须属于 begin 时的执行身份和子任务的最近 attempt。创建、状态变更、双 revision、回执及事件在同一 SQLite 事务提交。失败事务不消耗 request ID；同一身份、相同 request ID 和内容重试返回原响应，包含原来分配的 ID。重试旧回执前先查询，跨 turn 的新工作使用新 request ID。

真实文件保存后再调用 `submitDelivery`。该操作按原规则计算哈希、冻结文件并暂停等待用户；新 work 变更在待审批期间被拒绝。完整交付参数见 [阶段交付说明](ASSET-DELIVERIES.md)，注入 Codex 的操作指令见 [task-callback.md](../resources/instructions/task-callback.md)。

## 中断、重试和返工

开始新 turn 前以及 turn 结束后，执行器都会关闭残留的 running attempt，记录为 interrupted，并保存结束时间和原因。宿主中断 / 重启恢复也执行此处理，不要求存在 Blender session。持久化失败会使执行失败，不能继续假装状态已记录。

恢复后先查看状态、旧回执、工作文件和实际场景，再用 `begin` 建立新的 attempt。旧尝试保留；新尝试记录恢复说明并绑定新的执行身份。即使 Codex 正常结束 turn，只要没有 finish，未完成 attempt 也会标记 interrupted。跨 turn 不能补交旧 attempt 的完成报告。

reject / reopen 会把当前及受影响下游的非取消子任务标记为 stale，失效相应阶段批准，并保留候选与执行历史。重新执行必须绑定更新后的上游候选；上一次成功报告不能继续作为有效输入使用。取消的步骤保持取消，必要的新工作重新登记。

数据库去重只保护 Beaver 状态变更。恢复说明是 Codex 报告的检查结果，Beaver 尚不能据此证明外部 Blender 相对操作已经执行或未执行；本批不自动重放这些操作，也不提供外部操作恰好执行一次的保证。

## 可观察的证据

宿主绑定执行身份、状态转换、检查点引用及上游候选；候选文件字节由现有捕获与哈希核验路径保存。attempt 的 `inputs`、`outputs`、`tools` 由 Codex 报告，尚未自动截获真实插件或工具调用。工具名和版本已登记，不代表插件实际运行；检查点 ID 已关联，也不代表未保存场景可以完整恢复。

“阶段交付”页增加只读子任务列表，可展开每次 attempt，查看定义、验收要求、身份、时间、检查点、恢复说明和报告的输入输出 / 工具；候选详情显示关联的 attempt ID。面板明确说明模型报告仍需要候选文件核验和用户批准。旧状态没有 work 时显示“尚未登记子任务。”，保留阶段交付视图。

本批浏览器组件使用真实 `DeliveryPanel` 和 `WorkPanel`、模拟读取 API，验证历史展开、旧 / 新身份、输入输出、工具版本、恢复说明、下游 stale 状态、候选绑定以及脚本文字转义。它未启动原生宿主，也没有验证这批改动在 Beaver.exe 中的完整联动。

## 文件职责

- `native/core/src/asset_work_contract.rs`：work 请求动作、定义与 schema。
- `native/core/src/asset_work.rs`：持久状态、当前阶段和提交门禁、失效 / 中断处理。
- `native/core/src/asset_work_actions.rs`：创建、修订、取消、串行开始和同身份结束。
- `task_callback.rs`、`asset_delivery*.rs`：既有回调事务、候选绑定、交付及返工集成。
- `asset_task.rs`、`asset_tool.rs`、`executor.rs`：状态投影、重启与 turn 生命周期接入。
- `src/shared/asset-work.ts`、`src/ui/asset-task/WorkPanel.tsx`：共享类型及只读观察；由既有 DeliveryPanel 展示。
- `native/core/tests/asset_work*.rs`、`native/core/tests/support/asset_work.rs`：业务和恢复回归。
- `native/core/examples/support/executor_callbacks.rs`：通过公共回调建立未完成记录，验证执行器生命周期，不公开内部实现模块。

## 本批验证记录

全部命令在项目根目录执行。Cargo 使用正常 PowerShell / MSVC 环境，保留 `LIB`；日志保存后读取。PowerShell 日志中的 `NativeCommandError` 包装可能来自 Cargo 的正常 stderr，判定依据是命令退出码、测试汇总和 proof。

```powershell
rtk proxy cargo test --locked -p beaver-core --test asset_work --test asset_work_recovery --test task_callback --test asset_delivery --test asset_delivery_integrity
rtk proxy cargo run --locked -p beaver-core --example executor_contract -- output/asset-work-validation/executor-final --callbacks
rtk proxy npm run typecheck
```

- 核心测试 21 项通过：asset_work 4、asset_work_recovery 4、task_callback 6、asset_delivery 3、asset_delivery_integrity 4。覆盖顺序和当前阶段限制、并发尝试拒绝、成功候选绑定、修改历史、跨身份拒绝、幂等 ID、事务回滚、无 Blender 会话恢复、上游返工失效和历史字段读取。
- 执行器 3 个模拟 Codex 场景通过：新线程、兼容恢复、旧协议升级。每个场景验证新 turn 前关闭旧 attempt、动态回调创建恢复 attempt、正常 turn 结束后关闭未 finish 的尝试；不生成模型文件。
- TypeScript 类型检查通过。
- 浏览器组件 2 个状态通过：展开的执行历史和无 work 的旧状态；脚本文字没有成为 DOM script 节点，控制台 0 errors / 0 warnings，截图已查看。本批不重复候选审批按钮的既有组件测试。

证据目录：

- [核心测试日志](../output/asset-work-validation/rust-tests.log)、[类型检查日志](../output/asset-work-validation/typecheck.log)。
- [执行器 proof](../output/asset-work-validation/executor-final/proof.json)、[执行器日志](../output/asset-work-validation/executor-final.log)。
- [展开历史截图](../output/playwright/asset-work/history.png)、[旧状态截图](../output/playwright/asset-work/legacy.png)、[组件核验记录](../output/playwright/asset-work/proof.json)。
- 可重用组件夹具：`output/playwright/asset-work/fixture.tsx` 与 `serve.ts`。本轮创建的 browser session 和服务已关闭。

最终检查通过：10 个本批前端及文档文件的 Prettier 检查、`cargo fmt --all -- --check` 和 `git diff --check` 均通过。`npm run check:effective-lines` 扫描 484 个源文件，18 个未改动历史超限文件，0 个违规；未调整基线。新增工作流模块分别为 64、142、199 有效行，观察面板为 104 行，执行器为 464 行，执行器夹具为 471 行。30 个本批源文件、文档及组件夹具文件均为有效 UTF-8 且无 BOM；4 份更新文档的本地链接存在性检查通过。汇总见 [最终检查记录](../output/asset-work-validation/final-checks.json)，原始格式和差异检查日志保存在同目录。

## 交接与下一批

本批没有重编译或替换 Beaver.exe，没有替换发布包，内部版本保持 `0.1.19.1`。运行中的旧可执行文件不会自动获得这些源码改动。[前批原生验证](ASSET-DELIVERY-NATIVE-VALIDATION.md) 保留为历史证据，不作为新增 attempt 面板已经完成原生验证的依据。未运行完整游戏创作、完整 NPR 流程或正式发布验收。

剩余框架工作包括：源文件和外部依赖清单及版本校验；实际工具调用与 attempt 的自动关联；Godot / Codex / Blender 插件的发现、准备、启用、重载和运行结果；技术 / 视觉检查的可版本化规则和回执；耗时操作查询与取消；未保存场景恢复及新增面板的相关原生联动。

下一批先定义并接入一次执行的源文件与依赖清单，复用现有文件快照 / 哈希能力，明确缺失、漂移和上游变化时的处理。选取一个小范围输入变更样本做相关功能验证，再推进插件生命周期。NPR 各小步骤的工具链试作仍在框架条件满足后进行，每批提交可查看的产物和验证证据。
