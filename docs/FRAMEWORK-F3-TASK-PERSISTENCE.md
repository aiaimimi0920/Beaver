# F3.1 对象任务持久化实施记录

日期：2026-09-22。对应 [实际框架实现开发计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) 的 F3.1。本批完成对象任务层级、项目本地草稿、事务提交、查询 API 和原生任务视图的最小真实数据接入。F3 继续进行，F3.2-F3.4 及 F4 执行队列仍待实施。本批未构建或发布新的 Beaver.exe，未执行完整原生或游戏创作验收。

## 模块与存储边界

核心入口为 `native/core/src/object_tasks.rs`，职责分布如下：

| 模块                                                                         | 职责                                                                                |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `object_task_types.rs`、`object_task_validation.rs`                          | 提案、草稿、任务、run、回执和快照契约；层级、身份、依赖及项目归属校验               |
| `object_task_drafts.rs`                                                      | 项目草稿保存、读取，以及草稿和计划 revision 校验                                    |
| `object_task_commit.rs`、`object_task_commit_records.rs`                     | 提交前记录准备、已存在记录复用，以及对象、任务、run、计划 revision 和回执的事务提交 |
| `object_task_storage.rs`、`object_tasks.rs`                                  | 存储种类、项目范围的任务/run 查询和一致性快照                                       |
| `native/desktop/src/object_task_catalog.rs`、`data_dispatch_object_tasks.rs` | 六个原生业务 API 的输入约束与分发                                                   |
| `native/desktop/src/business_effects.rs`                                     | 区分界面通知和调度器唤醒，保持对象任务的执行隔离                                    |
| `src/shared/object-tasks.ts`、`object-task-snapshot.ts`                      | 共享类型、响应校验和任务展示数据                                                    |
| `src/ui/object-tasks/`、`src/ui/TasksView.tsx`                               | 真实快照查询、过期响应隔离、任务层级和查询状态展示                                  |

记录写入已经打开的目标项目 Runtime Store，使用独立的 `object_task`、`object_run`、`object_task_draft`、`object_task_plan_state` 和 `object_task_commit_receipt` 存储种类。所需新对象复用 `object_catalog` 的登记约束。

对象任务不写入旧 `kind: "task"` 记录，不创建旧任务工作区，不进入旧调度器。普通任务继续使用现有协议。所有新增任务和 run 保持 `planned`，run 的 `baselineVersionId` 为 `null`；工作基准和执行权留给 F4 领取时确定。

## 层级、草稿与原子提交

- 粗修没有对象、run 或责任父任务，可通过子任务组织跨对象目标。
- 中修关联一个对象并拥有自己的 run；责任父任务可为粗修，也允许直接创建没有粗修父任务的中修。
- 精修必须归属中修，继承其对象和 run，并指定 `stageId`。向已保存中修追加精修时，提交回执包含复用的 run。
- 责任父任务与 `dependsOn` 分开保存。校验拒绝循环依赖、悬空引用、非法层级、跨项目关联和跨对象精修。

`saveDraft` 持久化提案。新草稿的 `expectedRevision` 为 0，保存成功后草稿 revision 递增；再次保存须携带当前 revision。同时校验 `expectedPlanRevision`，将保存时的计划 revision 固定在草稿中。草稿可以从同一项目重新读取，尚未提供编辑草稿的完整 UI。

首次提交在 Store 的 SQLite Immediate 事务中校验请求、计划 revision、草稿 revision 和草稿所依据的计划 revision，再登记所需对象、写入任务和 run、更新计划状态，最后保存请求回执。任一步骤失败都会回滚这些写入。回执写入故障注入测试覆盖对象、run、任务、计划 revision 和回执全部回滚，草稿仍可读取。

提交首先检查 `requestId` 的已保存回执；请求的项目、草稿及预期 revision 必须与回执一致，匹配时直接返回回执，不匹配则返回请求冲突。没有回执时才读取当前草稿。成功提交后即使草稿已被修改，原请求仍可重放原回执。使用新请求重复提交内容相同、身份相同的提案时复用记录，不重复创建对象、任务或 run，也不推进计划 revision。复用同一身份但改变内容会被拒绝。

`snapshot` 在同一个读取事务中获得计划 revision、任务和 run，避免把不同时刻的记录拼为快照。

## 原生 API

六个 API 均要求显式 `projectId`，且目标项目 Runtime 已经打开。读取也遵循该约束；关闭项目不会隐式打开，调用不会回退到宿主 Store。输入使用 camelCase，拒绝未知嵌套字段和执行字段。

| 方法                   | 输入字段                                                                             | 返回值                                                    |
| ---------------------- | ------------------------------------------------------------------------------------ | --------------------------------------------------------- |
| `objectTask.saveDraft` | `projectId`、`draftId`、`expectedRevision`、`expectedPlanRevision`、`plan`           | 保存后的草稿，包含 `revision` 和 `planRevision`           |
| `objectTask.getDraft`  | `projectId`、`draftId`                                                               | 草稿或 `null`                                             |
| `objectTask.commit`    | `projectId`、`requestId`、`draftId`、`expectedDraftRevision`、`expectedPlanRevision` | 提交回执，包含前后计划 revision、对象/任务 ID 和 run 记录 |
| `objectTask.snapshot`  | `projectId`                                                                          | `{ planRevision, tasks, runs }`                           |
| `objectTask.get`       | `projectId`、`taskId`                                                                | 任务记录或 `null`                                         |
| `objectTask.getRun`    | `projectId`、`runId`                                                                 | run 记录或 `null`                                         |

`plan.objects` 的记录使用 `id`、`name` 和可选 `category`。`plan.tasks` 的记录使用 `id`、`granularity`、`title`、`prompt`、`acceptance`，以及按层级适用的 `objectId`、`parentTaskId`、`dependsOn`、`stageId`；`granularity` 为 `coarse`、`medium` 或 `fine`。调用方不能自行写入任务执行状态或 run 执行信息。

通常先读取快照的 `planRevision`，再保存带预期草稿 revision 的提案，最后使用返回的草稿 revision 提交。响应保留内层冲突和校验错误，调用方可区分失败原因。调用日志按显式项目路由；对象任务 ID 即使与宿主中的旧任务 ID 相同，也不会改变日志归属。

`saveDraft` 和 `commit` 只通知界面，不唤醒调度器。四个读取方法既不通知界面，也不唤醒调度器。该边界由原生分发和副作用测试共同覆盖。

## 最小真实任务视图

原生 `TasksView` 增加“普通任务 / 对象任务”按钮组，默认显示普通任务。对象任务面板调用 `objectTask.snapshot`，分别显示加载、空数据、错误和手动刷新状态。`App.tsx` 的任务及创作路由共用该视图，现有入口可达。

对象任务列表按粗、中、精层级展示真实任务 ID、对象 ID、责任父任务、run、阶段和依赖。面板明确展示已规划状态、未确定基准和执行尚未开放；不提供执行按钮，也不生成精确进度百分比。

共享响应校验覆盖项目归属、重复身份、父子层级、对象/run/identity 一致性及依赖引用。查询使用 generation 防护，较旧请求的成功或失败、切换项目后的迟到响应以及取消后的响应均不会覆盖当前结果。

这里完成任务层级的最小只读展示。完整泳道、迭代栈、制造导航、计划编辑和标题交互继续属于 F3.2-F3.4。

## 定向验证

日志位于 [`output/f31-continuation/`](../output/f31-continuation/)。本批通过核心 14 项、桌面 63 项、前端 15 项，共 92 项定向测试，以及 TypeScript、Rust 格式、本阶段前端格式和有效行数检查。

核心验证命令：

```powershell
rtk proxy cargo test --locked -p beaver-core object_task --lib
rtk proxy cargo test --locked -p beaver-core --test object_framework_boundary --test object_framework_executor
```

分别通过 8 项和 5 + 1 项，见 [核心持久化日志](../output/f31-continuation/core-tests.log) 与 [核心执行边界日志](../output/f31-continuation/core-execution-boundary-tests.log)。覆盖草稿恢复、revision 冲突、请求重放、重复提案复用、层级和依赖拒绝、向现有中修追加精修、事务回滚及旧执行路径隔离。

桌面验证命令：

```powershell
rtk proxy cargo test --locked -p beaver-desktop object_task --bin Beaver
rtk proxy cargo test --locked -p beaver-desktop --bin Beaver -- data_dispatch::tests business_routing::tests business_routing::object_import_tests business_routing::registration_tests
```

分别通过 7 项和 56 项，见 [对象任务 API 日志](../output/f31-continuation/desktop-object-task-tests.log) 与 [路由回归日志](../output/f31-continuation/desktop-routing-regression-tests.log)。覆盖严格项目路由、关闭项目拒绝、嵌套输入拒绝、错误保留、调用日志归属和不唤醒调度器。

路由回归中修正了 `data_dispatch_tests.rs` 的两个旧夹具：`task_actions_keep_legacy_host_tasks_available` 和 `task_settings_keep_legacy_host_tasks_available`。它们现通过既有辅助函数先创建并登记项目，再插入宿主旧任务，满足已有生产路由的项目登记约束；生产路由行为未因此改变。

前端验证命令：

```powershell
rtk proxy npx tsx --test tests/object-tasks.test.ts tests/object-task-query.test.ts tests/object-framework.test.ts tests/task-board.test.ts tests/task-plan.test.ts
rtk proxy npm run typecheck
```

15 项测试和类型检查通过，见 [前端测试日志](../output/f31-continuation/ui-tests.log) 与 [类型检查日志](../output/f31-continuation/typecheck.log)。覆盖真实记录解析、层级及身份一致性、加载/错误/空结果、刷新重试、旧请求成功或失败隔离，以及取消和项目切换。

格式和结构检查命令：

```powershell
rtk proxy cargo fmt --all -- --check
rtk proxy npx prettier --check src/shared/object-tasks.ts src/shared/object-task-snapshot.ts src/ui/object-tasks src/ui/TasksView.tsx tests/fixtures/object-tasks.ts tests/object-tasks.test.ts tests/object-task-query.test.ts
rtk proxy npx prettier --check docs/FRAMEWORK-F3-TASK-PERSISTENCE.md
rtk proxy npm run check:effective-lines
```

以上检查通过，见 [Rust 格式日志](../output/f31-continuation/rustfmt-check.log)、[F3.1 前端格式日志](../output/f31-continuation/f31-format-check.log)、[本记录格式日志](../output/f31-continuation/docs-format-check.log) 和 [有效行检查日志](../output/f31-continuation/effective-lines.log)。结构检查扫描 802 个源文件，保留 17 个未变更历史文件，0 个违规；未更新历史基线。机器报告为 [`output/effective-code-lines.json`](../output/effective-code-lines.json)。

本批较大的变更文件及有效行数如下：

| 文件                                              | 有效行数 |
| ------------------------------------------------- | -------: |
| `native/desktop/src/data_dispatch_tests.rs`       |      464 |
| `native/desktop/src/data_dispatch.rs`             |      370 |
| `native/desktop/src/business_catalog.rs`          |      360 |
| `native/desktop/src/business_routing.rs`          |      348 |
| `native/desktop/src/business_routing_dispatch.rs` |      308 |
| `src/ui/TasksView.tsx`                            |      306 |
| `native/core/src/object_task_commit_records.rs`   |      237 |
| `native/core/src/object_task_types.rs`            |      215 |

36 个选定的 F3.1 源码、测试和集成文件已核验为 UTF-8 无 BOM。上述 251-500 行文件保持各自的分发、路由、目录、视图或测试职责；新增持久化职责已拆入独立模块。

全仓 `rtk proxy npm run format:check` 仍未通过，见 [全仓格式日志](../output/f31-continuation/format-check.log)。它报告 16 个本批未修改、相对 HEAD 保持干净的文件存在格式问题：

- `src/ui/asset-task/` 下的 `AnnotationOverlay.tsx`、`asset-preview.css`、`asset-task-window.css`、`AssetPreview.tsx`、`AssetProgress.tsx`、`AssetTitlebar.tsx`、`feedback-draft.ts`、`FeedbackComposer.tsx`、`FeedbackHistory.tsx`、`preview-controls.ts`、`ReferenceEditor.tsx`、`use-asset-state.ts`、`use-observer.ts`。
- `src/ui/ProjectRegistrationDialog.tsx`。
- `tests/asset-feedback-draft.test.ts`、`tests/asset-preview.test.ts`。

F3.1 文件的定向格式检查已通过；全仓格式门槛仍有上述历史问题。

## 后续范围

F3.2 继续接入 Codex 追问、假设和提案、用户编辑计划、未启动项增撤及 revision 冲突交互。F3.3 继续接入标题手填/AI 建议、提交锁定、解锁草稿和插入位置；项目本地草稿 API 已提供基础。F3.4 继续接入全层级泳道、对象迭代栈、制造导航和基于事实的进度。

F4 继续实现对象串行队列、事务写入权、代次、领取时基准绑定、共享工作区及暂停/取消/失败恢复。完成这些执行约束之前，对象任务保持不可执行。F2.4 仍保留未勾选状态，F7 继续负责正式复制、身份改写、目标对象提交、发布和恢复；本批对象计划持久化不改变该导入边界。
