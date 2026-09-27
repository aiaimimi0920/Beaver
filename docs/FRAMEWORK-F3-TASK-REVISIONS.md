# F3.2 未启动任务定义修订实施记录

日期：2026-09-23。对应 [实际框架实现开发计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) 的 F3.2，接续 [人工规划与撤销记录](FRAMEWORK-F3-TASK-PLANNING.md)。本批接通已提交、未启动对象任务的定义编辑、人工复核、修订历史和版本冲突恢复。Codex 追问及拆分提案仍待接入，F3.2 保持未勾选；对象执行继续等待 F4。

## 可观察结果与源码范围

对象任务列表显示任务 revision。`planned` 任务可打开“修订定义”，编辑标题、目标、验收要求和依赖，并填写修订原因。提交前单独展示原定义、新定义、采用者和受影响任务，用户复核后才发送请求。已经撤销的任务保留“修订历史”入口；不可修订任务只读。原来选中的缺失或已撤销依赖继续显示，并允许移除。

界面继续使用已有 Dialog 和样式变量。`ObjectTasksPanel.tsx` 连接项目工作区、列表和独立修订会话；`ObjectTaskList.tsx` 提供入口。`ObjectTaskDefinitionEditor.tsx` 负责定义表单，`ObjectTaskDefinitionComparison.tsx` 负责前后对比，`ObjectTaskRevisionDialog.tsx` 负责复核、冲突和完成状态，`ObjectTaskRevisionHistory.tsx` 展示历史。

`src/ui/object-tasks/object-task-revision.ts` 独立管理修订会话，`object-task-revision-history.ts` 独立管理历史加载和已确认回执。原有 `object-task-workspace.ts` 继续负责规划草稿及同步；修订后的刷新不会丢弃尚未保存的规划内容，会按现有规则暴露计划版本冲突。

## 请求、历史与事务边界

新增原生业务操作 `objectTask.revisePlanned` 和只读 `objectTask.revisions`，通过 `native/desktop/src/data_dispatch_object_tasks.rs` 路由到显式指定、已打开项目的 Runtime Store。严格 schema 和工具属性位于 `object_task_catalog.rs`；关闭、legacy、缺失项目以及跨项目目标不能借用宿主或其他项目的记录。

```text
definition: title, prompt, acceptance, dependsOn
request: projectId, taskId, requestId,
         expectedTaskRevision, expectedPlanRevision,
         definition, reason
receipt/history: projectId, taskId, requestId,
                 previousTaskRevision, taskRevision,
                 previousPlanRevision, planRevision,
                 before, after, reason, adoptedBy, createdAt,
                 affectedTaskIds
history query: projectId, taskId
```

`native/core/src/object_task_definition.rs` 定义可修改字段、请求和不可变历史类型。任务、对象、run、阶段、责任父任务和粒度身份保持固定；未知字段被拒绝，不能借定义修订改变执行身份。标题、目标和原因必须非空，标题、目标、验收要求和原因的 UTF-8 字节上限分别为 300、20,000、10,000 和 2,000；依赖不得重复、自引用、缺失、已撤销或成环。请求不接受 `adoptedBy`，服务端固定记录为 `owner`。

`object_task_revisions.rs` 在一个 Store 事务中检查任务与计划的预期 revision、资格、引用及依赖，更新目标任务定义和计划 revision，最后插入历史回执。两种 revision 各增加 1；无变化提交、版本耗尽及任何写入失败均不会留下部分更新。历史回执同时承担幂等记录：完整请求相同的 `requestId` 返回原回执，即使任务后来已撤销；同 ID 携带不同定义、原因或预期版本会报请求冲突。

`object_task_revision_scope.rs` 校验目标及其责任后代的任务身份、项目归属和 run，要求仍参与执行的责任任务未启动、所属 run 未启动且没有已领取基准。修订前沿责任父子关系和反向依赖传递计算影响范围；计算可以穿过已撤销节点，但回执中的影响列表排除已撤销任务并按 ID 排序。影响列表供复核和追溯使用；本次只改目标定义及计划版本，不改其他任务定义、对象内容、run 或假设历史。

本批还修正了校验顺序：影响范围内存在跨项目依赖后代时，先校验范围归属，再校验提案引用，避免把归属错误误报为 `OBJECT_TASK_DEPENDENCY_NOT_FOUND: dependent-child`。回归 `rejects_project_identity_and_fine_run_mismatches` 同时覆盖项目、任务身份及精修所属 run 不匹配，当前返回相应的归属或身份错误。

## 复核、重试和冲突恢复

共享契约位于 `src/shared/object-task-revisions.ts`，影响范围、修订资格和依赖校验位于 `object-task-revision-plan.ts`。打开会话时复制并校验快照；复核后固定完整请求及请求 ID。重复点击不会并发写入，响应丢失后只能重试原请求，不能在不确定前次结果时改写提交内容。传输层也不能通过修改参数对象改变已冻结的请求。

成功回执必须与完整请求、原定义、前后 revision 及排序后的影响范围精确匹配。错误或畸形回执不会被展示为成功。已确认成功后保留回执；工作区刷新失败可以单独重试刷新，不会再次提交。历史读取独立执行，读取慢或失败不能阻塞已经确认的修订。

`OBJECT_TASK_PLAN_REVISION_CONFLICT` 和 `OBJECT_TASK_REVISION_CONFLICT` 都保留本地定义与原因，并读取最新项目快照。对话框展示原定义、本地修改和最新定义。用户显式采用最新基准后，再次检查固定身份、任务及 run 的修订资格，保留本地输入并要求重新复核；下次提交使用新请求 ID 和新预期 revision。最新依赖已经撤销时，可以采用基准后移除该依赖，再通过复核提交。

目标已撤销或移除、责任身份改变、run 已领取基准时不能采用最新基准继续修订。错误项目、回退或未推进的冲突快照会被拒绝；重新加载前清理先前的最新快照。关闭或切换项目会废弃在途代次，旧提交、快照或历史响应不能更新后续会话。

历史验证包含项目和任务身份、唯一请求 ID、revision 顺序、定义连续性及已知记录不可改写或消失。刚确认的回执会使先前发出的历史读取失效，防止旧读取擦除成功记录。读取失败保留已有历史，清理后重新加载以及 React StrictMode 式清理/重建也有回归覆盖。

## 定向验证

本批通过 100 项不同的定向测试：Core 29 项、Desktop 12 项、前端 59 项。前端包含本批新增的 28 项契约、会话、冲突、历史和静态渲染测试，以及原有直接相关的 31 项测试。全部命令从正常 PowerShell 工具链环境执行；原生命令保留 MSVC `LIB`。

```powershell
rtk proxy cargo test --locked -p beaver-core object_task
rtk proxy cargo test --locked -p beaver-desktop object_task
rtk proxy npx tsx --test tests/object-task*.test.ts
rtk proxy npm run typecheck
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
```

- [Core 日志](../output/f32-task-revisions-continuation/core-object-task.log)：29 项通过，覆盖原持久化/撤销、修订资格、依赖及身份、不可变历史、幂等请求和事务回滚。
- [Desktop 日志](../output/f32-task-revisions-continuation/desktop-object-task.log)：12 项通过，覆盖严格项目路由、未知字段拒绝、历史查询、稳定重试、错误保留及宿主/其他项目隔离。
- [前端日志](../output/f32-task-revisions-continuation/frontend-object-task-1.log)：59 项通过，覆盖复核前禁止提交、冻结重试、并发抑制、回执校验、冲突恢复、历史竞态、关闭隔离及真实工作区草稿保留。
- [前端定向复跑](../output/f32-task-revisions-continuation/frontend-revision-rerun.log)：测试中的数组/正则匹配项收窄后，修改过的冲突与 UI 测试 12 项通过；[类型复查](../output/f32-task-revisions-continuation/typecheck-rerun.log) 退出码 0。
- [Rust 格式复查](../output/f32-task-revisions-continuation/rust-format-check-final.log)：退出码 0。首次检查只发现 `object_task_revision_dispatch_tests.rs` 的格式问题，运行官方 `cargo fmt --all` 后复查通过；格式化后的 [3 项桌面修订测试](../output/f32-task-revisions-continuation/desktop-revision-format-rerun.log) 通过。

新增前端测试位于 `tests/object-task-revision-contract.test.ts`、`object-task-revision.test.ts`、`object-task-revision-conflict.test.ts`、`object-task-revision-history.test.ts` 和 `object-task-revision-ui.test.ts`，共用 `tests/fixtures/object-task-revisions.ts`。静态渲染覆盖编辑/只读入口、定义前后对比、影响范围、显式采用基准、仅刷新恢复、审计元数据及不可用依赖移除控件。

本批定向 Prettier 检查覆盖以下 17 个前端/测试文件和 4 份文档；结果见 [格式检查日志](../output/f32-task-revisions-continuation/prettier-check-final.log)。

```powershell
rtk proxy npx prettier --check `
  src/shared/object-task-revisions.ts `
  src/shared/object-task-revision-plan.ts `
  src/ui/object-tasks/object-task-revision.ts `
  src/ui/object-tasks/object-task-revision-history.ts `
  src/ui/object-tasks/ObjectTaskDefinitionEditor.tsx `
  src/ui/object-tasks/ObjectTaskDefinitionComparison.tsx `
  src/ui/object-tasks/ObjectTaskRevisionHistory.tsx `
  src/ui/object-tasks/ObjectTaskRevisionDialog.tsx `
  src/ui/object-tasks/object-task-revisions.css `
  src/ui/object-tasks/ObjectTasksPanel.tsx `
  src/ui/object-tasks/ObjectTaskList.tsx `
  tests/fixtures/object-task-revisions.ts `
  tests/object-task-revision-contract.test.ts `
  tests/object-task-revision.test.ts `
  tests/object-task-revision-conflict.test.ts `
  tests/object-task-revision-history.test.ts `
  tests/object-task-revision-ui.test.ts `
  docs/FRAMEWORK-F3-TASK-REVISIONS.md `
  docs/FRAMEWORK-F3-TASK-PLANNING.md `
  docs/FRAMEWORK-IMPLEMENTATION-PLAN.md `
  docs/README.md
```

[有效行检查](../output/f32-task-revisions-continuation/effective-lines-final.log) 通过：838 个源文件，17 个未变更历史文件，0 个违规；未更新历史基线。主要有效行数为：核心定义类型 72、影响范围 136、修订事务 154、桌面修订测试 158、共享契约 160、修订对话框 204、修订会话 269。修订会话集中拥有单次操作的复核、提交及恢复状态，历史请求生命周期已独立拆出；本批未扩展原有 477 行的草稿工作区。新增及修改源码和文档保持 UTF-8 无 BOM。

## 交付边界与后续入口

本批完成源码、相关功能测试、类型、格式和结构检查。持久化复用现有项目 Store 的实体表，增加 `object_task_definition_revision` 实体记录，没有执行数据迁移。对象任务不写入旧任务队列，也不唤醒旧调度器。

本批未构建或发布新的 Beaver.exe，未执行浏览器、原生界面或完整游戏创作验收；当前 UI 证据为状态测试与 React 静态渲染。现有内部版本 `0.1.19.36` 保持原交付记录，不能据本批源码结果推断其已包含新增能力。本批尚无新增用户验收反馈。

下一实施入口为 F3.2 的 Codex 需求追问、合理假设和拆分提案，将结果接入同一项目草稿、人工复核和提交协议，再推进 F3.3 的标题/锁定/插入与 F3.4 的泳道、迭代栈和制造导航。F4 的对象队列、领取基准和共享工作区尚未完成，执行继续关闭；运行中取消、成果处置及发布恢复仍由 F4/F7 后续实现。
