# F4 对象队列、工作区准备、精修执行与控制记录

日期：2026-09-24，后续增量更新至 2026-09-25。记录 F4 队列生命周期、准备、执行及恢复控制。总体状态及下一入口见 [框架实施计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)。F4.2 已完成定向开发验证，F4.1、F4.3、F4.4 继续进行；下文保留各历史切片交付时的边界。

## 显式重试执行增量（2026-09-25）

失败或中断的 fine 可以在最新核验和用户确认后重试。新尝试沿用前次输出检查点，保留原执行记录；请求和创建回执持久化，响应丢失只能按原请求重试，重开项目不会自动执行。生产恢复面板提示模型执行费用，并区分创建回执与当前执行进度。Core、Scheduler、Desktop 路由和 UI 已完成定向验证，详见 [显式重试交付证据](FRAMEWORK-F4-RESUME-EVIDENCE.md)。受管 Godot/Blender 会话恢复和 F5 门槛推进不在本切片内，F4.4 保持未勾选。

## 实现范围

入队只接受身份完整且一致的 medium 任务及其 run，粗修和精修不能独立取得对象写入权。同一任务重复入队保持幂等；队列持久顺序、完整 queued 集合重排和只读查询继续沿用现有协议。

| 阶段                 | Queue 状态           | Medium / run 状态    | 持有对象 |
| -------------------- | -------------------- | -------------------- | -------- |
| 已入队，尚未领取     | `queued`             | `planned`            | 否       |
| 已领取，等待实际执行 | `claimed`            | `queued`             | 是       |
| fine 正在执行        | `running`            | `running`            | 是       |
| 成功收尾，等待验收   | `awaitingAcceptance` | `awaitingAcceptance` | 是       |
| 失败，等待恢复       | `failed`             | `failed`             | 是       |
| metadata 取消完成    | `cancelled`          | `cancelled`          | 否       |

领取原子更新 queue、medium 和 run，递增相应 revision 及 claim generation。成功或失败收尾均保留对象所有权，重开项目后仍阻止同对象后继领取。回调必须匹配 owner、claim token 和 generation；身份撕裂、重复或迟到 finish 无法改写记录。

同对象的第一个 queued 项先保留队首位置，再检查依赖。受阻队首不会被该对象的后继越过，其他对象仍可领取。依赖必须属于同一项目、记录身份匹配且状态为 `accepted`；当前 finish 只进入待验收，接受和发布链路留待后续实现。

`objectTask.cancelClaim` 校验持有者身份后，在同一事务内关闭责任任务、所属 run 及 queue，并推进计划 revision。`cancelPlanned` 复用同一责任范围实现；依赖关系不扩大取消范围，同对象独立后继保留。活动 run 下仍为 planned 的 fine 可单独撤销。修订溢出、状态冲突或中途写入失败会回滚全部 metadata 变更。

上述队列生命周期切片的取消只闭合数据库 metadata，已开始的 fine 会阻止该路径。该切片交付时尚未接入 Scheduler。后续执行增量已接通首个 fine 与 Windows 进程树收尾，证据见下文；文件取消处置、受管引擎资源释放及恢复仍待实现，metadata 取消不能证明这些操作已完成。

## Desktop 与前端接入

Desktop 的 `objectTask.*` 沿用 `ProjectStorageRouter` 的项目 runtime，拒绝 legacy、关闭或缺失的项目，并验证跨项目隔离。对象任务和队列不写入 host 或 legacy task 表。公开工具 proposal schema 补齐已有的 `position` 字段，范围为 `0..=1_000_000_000`，并增加非法位置分发回归。

共享任务和 run schema 接受 `planned`、`queued`、`running`、`awaitingAcceptance`、`failed`、`cancelled`。快照要求 medium 与其 run 状态一致，fine 保留自己的状态。状态文案统一为“已规划”“已领取（待执行）”“执行中”“待验收”“失败待恢复”“已撤销”；预览适配器区分运行、待验收和失败，百分比进度继续固定为 0。

任务列表及定义修订冲突视图使用真实状态。活动责任范围保留修订历史读取，禁止重新修订；领取竞态按既有冲突协议保留本地输入。整组撤销会检查完整责任任务及所属 medium runs，遇到活动项进入 `blocked`，不展示不完整范围或写入按钮，`submit()` 也不调用 API。planned fine 单独撤销仍可使用。

## 源码职责与结构

以下为队列生命周期切片当时的有效行数；未提高历史 baseline。新增 fixture 和拆分后的测试模块分别承接生命周期及完整性回归，避免继续扩张原测试文件。后续准备增量的结构数据单独列于下文。

| 文件                                                                                             | 职责                                  | 有效行数 |
| ------------------------------------------------------------------------------------------------ | ------------------------------------- | -------- |
| [object_task_queue.rs](../native/core/src/object_task_queue.rs)                                  | 队列读写、重排、队首选择及公共入口    | 322      |
| [object_task_queue_claim.rs](../native/core/src/object_task_queue_claim.rs)                      | Medium/run 校验、领取、收尾和取消事务 | 192      |
| [object_task_cancel.rs](../native/core/src/object_task_cancel.rs)                                | 规划撤销和共享责任范围取消            | 202      |
| [object_task_queue_tests.rs](../native/core/src/object_task_queue_tests.rs)                      | 队列基础行为回归                      | 213      |
| [object_task_queue_lifecycle_tests.rs](../native/core/src/object_task_queue_lifecycle_tests.rs)  | 队首、重开、所有权及责任范围回归      | 208      |
| [object_task_queue_integrity_tests.rs](../native/core/src/object_task_queue_integrity_tests.rs)  | 身份、代次、溢出及事务回滚回归        | 233      |
| [object_task_queue_dispatch_tests.rs](../native/desktop/src/object_task_queue_dispatch_tests.rs) | 项目 API 队列生命周期和隔离回归       | 240      |
| [object_task_dispatch_tests.rs](../native/desktop/src/object_task_dispatch_tests.rs)             | 现有对象任务 API 与非法位置回归       | 277      |
| [object-task-cancellation.ts](../src/ui/object-tasks/object-task-cancellation.ts)                | 撤销范围检查、阻塞和提交会话          | 159      |
| [ObjectTaskCancellationDialog.tsx](../src/ui/object-tasks/ObjectTaskCancellationDialog.tsx)      | 撤销确认与阻塞界面                    | 112      |
| [ObjectTaskList.tsx](../src/ui/object-tasks/ObjectTaskList.tsx)                                  | 真实状态、修订及撤销入口              | 167      |

共享类型、快照校验、状态文案和预览映射分别位于 `src/shared/object-tasks.ts`、`src/shared/object-task-snapshot.ts`、`src/ui/object-tasks/object-task-status.ts` 及 `src/ui/object-preview/object-task-preview-adapter.ts`。队列切片的 UI 回归集中在 [object-task-queue-ui.test.ts](../tests/object-task-queue-ui.test.ts)，当时为 8 项；执行增量扩展为 9 项，并覆盖 running 状态。

## 定向验证

Rust 命令均在相同 PowerShell/MSVC 环境运行，保留 `LIB`。以下是队列生命周期切片的历史结果；Desktop 的 13 项已经包含 queue dispatch 的 4 项，不重复计数。

| 验证范围                     | 结果                                       | 日志                                                             |
| ---------------------------- | ------------------------------------------ | ---------------------------------------------------------------- |
| Core queue                   | 18 passed，0 failed                        | [Core queue](../output/f4-queue-core-20260924.log)               |
| Core cancellation            | 10 passed，0 failed                        | [Core cancellation](../output/f4-queue-cancel-core-20260924.log) |
| Desktop object-task dispatch | 13 passed，0 failed                        | [Desktop dispatch](../output/f4-queue-desktop-20260924.log)      |
| 前端及直接依赖               | 70 passed，0 failed，0 skipped（历史记录） | 原日志被后续定向运行覆盖，见下述证据说明                         |

证据说明：执行增量误复用了 `f4-queue-ui-20260924.log`，覆盖了历史 70 项日志。该次 9 项队列 UI 结果已改存 [执行增量 UI](../output/f4-attempt-queue-ui-20260924.log)，不作为历史 70 项结果的证明。本轮前端及直接依赖的独立验证日志列于执行增量部分。

实际命令：

```powershell
rtk proxy cargo test --locked -p beaver-core object_task_queue --lib
rtk proxy cargo test --locked -p beaver-core object_tasks::tests::cancel --lib
rtk proxy cargo test --locked -p beaver-desktop data_dispatch::data_dispatch_object_tasks
rtk proxy npx tsx --test tests/object-task-queue-ui.test.ts tests/object-tasks.test.ts tests/object-task-query.test.ts tests/object-task-preview-adapter.test.ts tests/object-task-cancellation.test.ts tests/object-task-cancellation-ui.test.ts tests/object-task-revision.test.ts tests/object-task-revision-ui.test.ts tests/object-task-revision-conflict.test.ts tests/object-task-revision-contract.test.ts tests/object-task-revision-history.test.ts tests/object-task-rebase.test.ts tests/object-task-workspace.test.ts
rtk proxy npm run typecheck
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
rtk proxy npx prettier --check src/shared/object-tasks.ts src/shared/object-task-snapshot.ts src/ui/object-tasks/object-task-status.ts src/ui/object-tasks/ObjectTaskList.tsx src/ui/object-tasks/ObjectTaskRevisionDialog.tsx src/ui/object-preview/object-task-preview-adapter.ts src/ui/object-tasks/object-task-cancellation.ts src/ui/object-tasks/ObjectTaskCancellationDialog.tsx tests/object-task-cancellation-ui.test.ts tests/object-task-queue-ui.test.ts docs/FRAMEWORK-IMPLEMENTATION-PLAN.md docs/README.md docs/FRAMEWORK-F4-QUEUE-LIFECYCLE.md
rtk proxy git diff --check
```

类型检查、Rust 格式检查和有效行检查均退出 0。结构结果为 **865 sources，17 unchanged legacy files，0 violations**。日志分别为 [typecheck](../output/f4-queue-typecheck-20260924.log)、[Rust 格式](../output/f4-queue-rustfmt-final-20260924.log)、[有效行检查](../output/f4-queue-effective-lines-20260924.log)，详细数据见 [effective-code-lines.json](../output/effective-code-lines.json)。

Rust 首次格式检查只发现新增 Desktop queue 测试的格式问题，定向 rustfmt 后复查通过。编译日志仍有既有 `validation/service.rs` 未读字段及测试 `unused_mut` 警告；本批未扩大清理范围。

修正前的失败证据保留在 [Core 生命周期日志](../output/f4-queue-lifecycle-before-20260924.log) 和 [前端日志](../output/f4-queue-ui-before-20260924.log)。前端失败包括状态 parser 拒绝、列表标签错误、待验收映射为排队中及修订 latest 为空。`f4-queue-desktop-before-20260924.log` 中的 4 项 queue dispatch 已通过；它记录 schema 补齐前的行为，不作为 position 保存失败证据。

定向 Prettier 和 `git diff --check` 均退出 0，日志见 [Prettier](../output/f4-queue-prettier-20260924.log) 和 [Git 差异检查](../output/f4-queue-diff-check-20260924.log)。Git 只报告现有换行策略的 LF/CRLF 提示。本批 22 个源文件及 3 个文档均为 UTF-8 无 BOM，无行尾空白；22 个源文件的 SHA-256 与结构检查报告一致，新记录的相对链接及两个文档入口均已核对。

## 基准与工作区准备增量

Core 新增 `object_run_preparation::claim_next/get/prepare`。领取在一个短事务内取得队列写入权、解析并绑定基准、保存准备日志；文件恢复在数据库锁外执行，随后用另一个短事务复核 owner、claim token、generation 和完整 medium/run 记录。日志状态为 `Pending`、`Preparing`、`Ready`、`Failed`。执行凭据 `PreparationClaim` 的字段私有，且不能 Clone 或 Deserialize；读取持久日志不会重新取得执行能力。

`latestAccepted` 按接受回执的对象 revision 判断最新版本，接受顺序可以不同于捕获顺序。多个 accepted 版本缺少可证明的接受顺序时拒绝领取基准；回执 key、项目/对象身份、命令摘要及 manifest 必须一致，独立 `object_version` 实体也必须与 manifest 相符。没有 accepted 版本时，`latestAccepted` 明确失败，首次从空对象制作需要显式选择 `empty`。

`pinned` 固定同对象的指定 accepted 版本；`empty` 固定空内容。三种策略均独立记录领取时的接受头和对象 revision，并冻结所选版本的引用闭包及内容摘要，供后续发布比较。领取后新接受的引用版本不会改变已冻结的准备输入。

工作区恢复先验证内容 blob，再原子创建新目录；复制后再次检查大小和 SHA-256。已有目录拒绝覆盖。基准解析失败或文件准备失败均保留对象写入权；文件准备完成后若记录或领取凭据已变化，提交被拒绝，`Preparing` 日志和目录保留。四种状态均通过真实关闭/重开项目验证，不会自动重放准备或放行同对象后继，独立对象仍可领取。

任何已有准备日志的 run 均拒绝直接 metadata finish/cancel，返回 `OBJECT_RUN_DISPOSITION_REQUIRED`。这为后续文件、进程和资源处置保留明确边界；当前尚未实现处置和恢复命令。

medium 草稿可选择最新已接受版本、指定历史版本或空基准，沿用现有 dirty/save/revision 协议。旧提案省略 `baseline` 时仍默认 `latestAccepted`，序列化保留字段省略；保存、提交、重放及已提交定义修订保留策略。共享 TypeScript schema、Desktop proposal schema 和 Core 验证均接通，coarse/fine 不可携带基准，pinned 必须指向同对象 accepted 版本。切换到非 medium 会清除基准，Codex 规划指令提示新对象显式使用 `empty`。

## 准备增量的结构与验证

本次结构检查为 **879 sources，17 unchanged legacy files，0 violations**，未修改历史 baseline。草稿编辑器的选择逻辑提取到 `object-task-draft-choices.ts`，基准控件独立到 `ObjectTaskBaselineEditor.tsx`；编辑器降至 463 个有效行，因此删除 `.beaver-code-structure.json` 中原有的 503 行例外。

| 文件                                                                                 | 职责                               | 有效行数 |
| ------------------------------------------------------------------------------------ | ---------------------------------- | -------- |
| [object_acceptance_history.rs](../native/core/src/object_acceptance_history.rs)      | 接受回执与版本顺序校验             | 93       |
| [object_run_baseline.rs](../native/core/src/object_run_baseline.rs)                  | 领取时基准、接受头和引用闭包冻结   | 115      |
| [object_run_preparation.rs](../native/core/src/object_run_preparation.rs)            | 准备状态、执行凭据与公共入口       | 115      |
| [object_run_prepare_store.rs](../native/core/src/object_run_prepare_store.rs)        | 短事务、领取身份复核及失败持权     | 175      |
| [object_run_workspace.rs](../native/core/src/object_run_workspace.rs)                | 内容验证、新目录和恢复             | 44       |
| [object_run_integrity_tests.rs](../native/core/src/object_run_integrity_tests.rs)    | 回执顺序、身份与版本实体损坏       | 107      |
| [object_run_workspace_tests.rs](../native/core/src/object_run_workspace_tests.rs)    | 冻结引用、内容损坏和已有目录保护   | 110      |
| [object_run_recovery_tests.rs](../native/core/src/object_run_recovery_tests.rs)      | 四种状态重开、锁外竞态及处置边界   | 179      |
| [object_task_baseline_tests.rs](../native/core/src/object_task_baseline_tests.rs)    | 草稿、提交、重放及定义修订         | 132      |
| [object_task_dispatch_tests.rs](../native/desktop/src/object_task_dispatch_tests.rs) | Desktop 保存、提交、重放及输入拒绝 | 330      |
| [ObjectTaskBaselineEditor.tsx](../src/ui/object-tasks/ObjectTaskBaselineEditor.tsx)  | medium 基准选择控件                | 53       |
| [ObjectTaskDraftEditor.tsx](../src/ui/object-tasks/ObjectTaskDraftEditor.tsx)        | 草稿字段与 revision 编辑流         | 463      |
| [object-task-draft-choices.ts](../src/ui/object-tasks/object-task-draft-choices.ts)  | 草稿父任务和对象选项               | 59       |
| [object-task-baseline.test.ts](../tests/object-task-baseline.test.ts)                | schema、保存恢复及控件渲染         | 134      |

| 验证范围                     | 结果                           | 日志                                                              |
| ---------------------------- | ------------------------------ | ----------------------------------------------------------------- |
| Core `object_task`           | 79 passed，0 failed            | [Core 对象任务](../output/f4-preparation-core-20260924.log)       |
| Core `object_run`            | 15 passed，0 failed            | [Core 准备边界](../output/f4-preparation-boundaries-20260924.log) |
| Desktop object-task dispatch | 15 passed，0 failed            | [Desktop 分发](../output/f4-preparation-desktop-20260924.log)     |
| 前端及直接依赖               | 42 passed，0 failed，0 skipped | [前端测试](../output/f4-preparation-frontend-20260924.log)        |

两个 Core 过滤范围有交集，不相加为独立测试总数。新增 Core 边界用例覆盖回执和 blob 损坏、已有目录保护、冻结引用不漂移、重启保持所有权、锁外 I/O，以及 medium/run revision 或 claim generation 变化后拒绝提交。Desktop 新增的两项验证显式基准保存/提交/重放和非法输入不写草稿；前端新增三项覆盖策略 schema、dirty/save/revision 和 medium 控件渲染。

实际命令：

```powershell
rtk proxy cargo test --locked -p beaver-core object_run --lib
rtk proxy cargo test --locked -p beaver-core object_task --lib
rtk proxy cargo test --locked -p beaver-desktop data_dispatch::data_dispatch_object_tasks
rtk proxy npx tsx --test tests/object-task-baseline.test.ts tests/object-tasks.test.ts tests/object-task-workspace.test.ts tests/object-task-rebase.test.ts tests/object-task-cancellation-ui.test.ts tests/object-task-queue-ui.test.ts tests/object-task-preview-adapter.test.ts tests/object-task-query.test.ts tests/object-task-revision-contract.test.ts
rtk proxy npm run typecheck
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
rtk proxy npx prettier --check .beaver-code-structure.json src/shared/object-tasks.ts src/ui/object-tasks/ObjectTaskBaselineEditor.tsx src/ui/object-tasks/ObjectTaskDraftEditor.tsx src/ui/object-tasks/object-task-draft-choices.ts tests/object-task-baseline.test.ts docs/FRAMEWORK-IMPLEMENTATION-PLAN.md docs/FRAMEWORK-F4-QUEUE-LIFECYCLE.md
rtk proxy git diff --check
```

类型、Rust 格式及有效行检查均退出 0，日志见 [typecheck](../output/f4-preparation-typecheck-20260924.log)、[Rust 格式](../output/f4-preparation-rustfmt-final-20260924.log) 和 [有效行检查](../output/f4-preparation-effective-lines-final-20260924.log)。首次结构检查因已缩小文件的旧例外失效而退出 1，删除该例外后通过，原始日志保留在 [首次结构检查](../output/f4-preparation-effective-lines-20260924.log)。编译仍报告已有的未读字段及 `unused_mut` 警告。

定向 [Prettier](../output/f4-preparation-prettier-20260924.log) 和 [Git 差异检查](../output/f4-preparation-diff-check-20260924.log) 均退出 0。27 个相关源文件、2 个文档和结构例外配置均为 UTF-8 无 BOM，无行尾空白；27 个源文件的 SHA-256 与当前结构报告一致，文档相对链接均存在，见 [源码与文档核验](../output/f4-preparation-source-proof-20260924.log)。历史 baseline 与 Git 中的原记录一致。

## 首个 fine 执行增量

Scheduler 现在通过 `TaskRuntime::from_project` 和项目登记 gate 取得一次性的 `PreparationClaim`，对象执行与 legacy 工作共享全局槽位。重复枚举同一项目不会重复计算活动对象或重复领取；批内后续领取失败时，先前已取得的凭据仍交给 Scheduler 持有和收尾。对象任务使用独立 worker，不进入 legacy `Execution`、`task_finish` 或合并流程。

共享工作区准备完成后，事务选择本 medium 下按 position/ID 排序的首个未撤销 fine，并校验其依赖。fine 定义、输入快照及完整 project/object/medium/run/fine/attempt 身份随尝试冻结；queue、medium、run 和该 fine 原子进入 running。只有本次内存凭据能启动执行，读取持久化记录不会重新获得启动能力。

每个 attempt 使用独立 `CODEX_HOME`，工作目录固定到 run 的共享工作区。工厂读取宿主 `ExecutionSettings`，清除继承的 Codex/OpenAI/Beaver 配置环境，只经环境传入 provider key。配置禁用宿主技能发现、MCP、网页搜索和多代理，使用 `workspace-write` 和 approval never；已有 attempt HOME/session 拒绝覆盖。启动隔离测试验证原项目与模拟个人配置、会话保持原文。

RPC 绑定 thread/turn，错误或过期身份不能收尾，steer 返回 `OBJECT_ATTEMPT_DEFINITION_FROZEN`。Windows 以挂起方式创建 app-server，加入启用 KILL_ON_JOB_CLOSE 的 Job 后恢复运行；关闭时终止本次 Job 并确认活动进程数为 0，再等待根进程退出。根进程退出通过 Child 句柄检测，避免 writer 继承 stdout 时误等到超时。无法证明进程树已停止时不捕获检查点，保持 Running 和对象写入权，并停止 Scheduler。

确认进程树停止后，在锁外捕获输出，并在最终事务内复核身份与取消请求。成功 turn 只把 attempt 置为 `AwaitingGate`，queue/medium/run/fine 对外显示 `awaitingAcceptance`；失败或中断保存部分输出，attempt 分别为 `Failed` / `Interrupted`，其任务和队列均为 `failed`。三者都保留 owner/token/generation 与共享工作区；等待期间释放执行槽位供独立对象使用，同对象后继 medium 和后继 fine 不启动。冻结完成后的 interrupt 不改写已保存结果。

Desktop 已连接对象工厂及 `Scheduler::start_with_objects`；只有 `objectTask.enqueue` 唤醒 Scheduler，对象只读查询继续无副作用。前端 schema、列表和预览显示“执行中”，运行中的修订保持只读、整组撤销受阻，同轮尚未启动的 planned fine 仍可单独撤销。真实关闭/重开 ProjectStore 后，新 Scheduler 不重放 Ready 或 Running 记录，两种工厂均不被调用。

## 执行增量的结构与验证

结构检查为 **893 sources，17 unchanged legacy files，0 violations**。以下为本次报告中的有效行数，历史 baseline 未提高；Scheduler 编排、共享槽位和 RPC 生命周期分别保持单一职责。

| 文件                                                                                      | 职责                                  | 有效行数 |
| ----------------------------------------------------------------------------------------- | ------------------------------------- | -------- |
| [object_attempt.rs](../native/core/src/object_attempt.rs)                                 | 类型、不可重放凭据和检查点入口        | 149      |
| [object_attempt_store.rs](../native/core/src/object_attempt_store.rs)                     | fine 选择、完整身份复核和状态事务     | 169      |
| [object_attempt_launch.rs](../native/core/src/object_attempt_launch.rs)                   | 隔离 HOME、配置和进程启动参数         | 96       |
| [object_attempt_rpc.rs](../native/core/src/object_attempt_rpc.rs)                         | thread/turn 绑定、协议和冻结输入      | 111      |
| [object_attempt_worker.rs](../native/core/src/object_attempt_worker.rs)                   | 单次执行、中断、关树与捕获顺序        | 132      |
| [rpc_process_tree.rs](../native/core/src/rpc_process_tree.rs)                             | Windows Job 进程树所有权和停止证明    | 154      |
| [rpc.rs](../native/core/src/rpc.rs)                                                       | RPC 管道、根进程等待和关闭            | 326      |
| [scheduler_work.rs](../native/core/src/scheduler_work.rs)                                 | typed 工作领取、失败保权和分发        | 144      |
| [scheduler.rs](../native/core/src/scheduler.rs)                                           | 活动执行、控制和收尾编排              | 320      |
| [scheduler_runtime_ops.rs](../native/core/src/scheduler_runtime_ops.rs)                   | Runtime 去重、槽位及领取核验          | 415      |
| [task_runtime.rs](../native/desktop/src/task_runtime.rs)                                  | Desktop 工厂和 Scheduler 生命周期接线 | 195      |
| [object_attempt_claim_tests.rs](../native/core/src/object_attempt_claim_tests.rs)         | 批内失败、Runtime 去重和槽位回归      | 81       |
| [object_attempt_scheduler_tests.rs](../native/core/src/object_attempt_scheduler_tests.rs) | Scheduler 并行、中断和重启不重放      | 195      |
| [object_attempt_launch_tests.rs](../native/core/src/object_attempt_launch_tests.rs)       | 启动隔离和已有配置保护                | 145      |
| [object-task-queue-ui.test.ts](../tests/object-task-queue-ui.test.ts)                     | 真实状态、修订及撤销界面边界          | 339      |

| 验证范围                              | 结果                           | 日志                                                                    |
| ------------------------------------- | ------------------------------ | ----------------------------------------------------------------------- |
| Core `object_attempt`                 | 18 passed，0 failed            | [执行集成](../output/f4-attempt-integration-20260924.log)               |
| Core `object_tasks`，含准备及 attempt | 76 passed，0 failed            | [对象任务直接依赖](../output/f4-attempt-core-dependencies-20260924.log) |
| Core `scheduler`                      | 22 passed，0 failed            | [Scheduler](../output/f4-attempt-scheduler-20260924.log)                |
| Desktop object-task dispatch          | 15 passed，0 failed            | [Desktop 分发](../output/f4-attempt-desktop-dispatch-20260924.log)      |
| Desktop business effects              | 1 passed，0 failed             | [enqueue 唤醒](../output/f4-attempt-desktop-effects-20260924.log)       |
| Desktop project lifecycle             | 4 passed，0 failed             | [项目生命周期](../output/f4-attempt-desktop-lifecycle-20260924.log)     |
| 前端及直接依赖，含队列 UI             | 43 passed，0 failed，0 skipped | [前端测试](../output/f4-attempt-ui-dependencies-20260924.log)           |

Core 三组过滤有交集，不相加为独立总数。18 项 attempt 测试含 2 个没有环境变量时直接返回的子进程测试入口，实际行为用例为 16 项；这些入口由行为测试再次启动为真实 app-server/writer 进程。覆盖完成后关树、失败、超时、交互请求、根进程退出、启动失败、过期回调、取消采样、并行上限、等待释放槽位、重复 Runtime 和重启不重放。前端 43 项包含单独运行通过的 9 项队列 UI 测试。

实际命令：

```powershell
rtk proxy cargo test --locked -p beaver-core object_attempt --lib
rtk proxy cargo test --locked -p beaver-core object_tasks --lib
rtk proxy cargo test --locked -p beaver-core scheduler --lib
rtk proxy cargo test --locked -p beaver-desktop data_dispatch::data_dispatch_object_tasks
rtk proxy cargo test --locked -p beaver-desktop business_effects
rtk proxy cargo test --locked -p beaver-desktop project_runtime_lifecycle
rtk proxy npx tsx --test tests/object-tasks.test.ts tests/object-task-baseline.test.ts tests/object-task-cancellation.test.ts tests/object-task-cancellation-ui.test.ts tests/object-task-preview-adapter.test.ts tests/object-task-queue-ui.test.ts tests/object-task-revision-ui.test.ts tests/object-task-workspace.test.ts
rtk proxy npm run typecheck
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
rtk proxy npx prettier --check src/shared/object-tasks.ts src/shared/object-task-snapshot.ts src/ui/object-tasks/object-task-status.ts src/ui/object-tasks/ObjectTaskList.tsx src/ui/object-tasks/ObjectTaskRevisionDialog.tsx src/ui/object-preview/object-task-preview-adapter.ts tests/object-task-queue-ui.test.ts docs/FRAMEWORK-IMPLEMENTATION-PLAN.md docs/FRAMEWORK-F4-QUEUE-LIFECYCLE.md
rtk proxy git diff --check
```

类型、Rust 格式和有效行检查均退出 0，见 [typecheck](../output/f4-attempt-typecheck-20260924.log)、[Rust 格式](../output/f4-attempt-rustfmt-20260924.log) 和 [结构检查](../output/f4-attempt-effective-lines-20260924.log)。编译仍报告既有 `validation/service.rs` 未读字段及测试 `unused_mut` 警告。测试使用隔离项目和假 app-server，不消耗真实模型额度，未替代完整原生或受管引擎验收。

定向 [Prettier](../output/f4-attempt-prettier-20260924.log) 和 [Git 差异检查](../output/f4-attempt-diff-check-20260924.log) 均退出 0。36 个相关源文件（含 24 个未跟踪源码）、2 个文档及结构配置通过 UTF-8 无 BOM 与行尾空白核验；结构报告中 893 个源文件的规范化 SHA-256 均与当前内容一致，文档相对链接及章节锚点均有效。历史 baseline 与 HEAD 一致，见 [源码与文档核验](../output/f4-attempt-source-proof-20260924.log)。

## 执行查询与显式中断增量

`objectTask.attempts` 返回完整执行身份和紧凑状态，区分 `active`、`recoveryRequired` 和 `finished`。Desktop 只从已打开项目取得对应 Store 和 Scheduler；查询不领取、不唤醒执行。Scheduler 关闭后仍可读取持久化记录和重放已完成回执，重开项目不会恢复启动能力。当前查询仍要求 run 保留对象所有权，F5/F7 释放所有权前需扩展历史查询；已完成中断回执重放不受这一限制。

`objectTask.interrupt` 要求稳定 request ID、预期 task revision，以及 `taskId`、`fineTaskId`、`objectId`、`runId`、`attemptId`、`owner`、`claimToken`、`generation`、`threadId`、`turnId` 全部目标字段。thread/turn 尚未绑定时必须显式传 `null`，不能省略。Core 校验项目和全部身份、实际 writer、Store 实例及请求冲突，先持久化中断意图，再通知本次执行者；意图已记录时不能继续绑定新的 RPC thread/turn。

收尾继续先确认本次进程树全部停止，再捕获输出；最终事务同时采样持久化意图和取消标记，原子冻结检查点、attempt/任务状态及不可变回执。冻结前接受的中断意图生效；冻结后到达的请求返回真实最终结果，可能仍为 `AwaitingGate` 或 `Failed`。中断不会覆盖已冻结结果，不会自动重试、推进后继 fine、批准或发布。`Interrupted` 和 `Failed` 在任务/run/queue 层均显示 `failed`，attempt 保留两者差异，medium 继续持有对象。

medium 列表新增“执行记录”入口。前端校验 project/run/object/medium/fine 归属和重复 attempt ID，按项目维护控制器；只有 `active` 记录显示中断操作。首次确认后固定目标和 request ID，并克隆发送载荷；遇到结果不明的失败，刷新或重试仍沿用原请求。只有精确的 `OBJECT_ATTEMPT_STALE_TARGET` / `OBJECT_ATTEMPT_REVISION_CONFLICT` 才清除待重试请求，要求重新查询、确认。收到匹配回执后，工作区刷新失败也保留回执，用户可单独重试刷新。关闭或切换项目使旧响应失效，同步关闭通知也能阻止后续派发。

### 执行控制的源码与验证

本次结构检查为 **908 sources，17 unchanged legacy files，0 violations**，历史 baseline 未提高。新增职责分别落在以下文件；相关测试同时覆盖 Core 事务、Scheduler 控制、Desktop 异步路由和前端竞态。

| 文件                                                                                  | 职责                              | 有效行数 |
| ------------------------------------------------------------------------------------- | --------------------------------- | -------- |
| [object_attempt_view.rs](../native/core/src/object_attempt_view.rs)                   | 完整身份和查询视图校验            | 146      |
| [object_attempt_control.rs](../native/core/src/object_attempt_control.rs)             | 持久化意图、回执与幂等复核        | 187      |
| [object_attempt_store.rs](../native/core/src/object_attempt_store.rs)                 | 检查点、最终状态和回执原子收尾    | 170      |
| [scheduler_object_control.rs](../native/core/src/scheduler_object_control.rs)         | writer/Store 匹配、查询和中断     | 149      |
| [object_attempt_runtime.rs](../native/desktop/src/object_attempt_runtime.rs)          | 已打开项目的异步业务路由          | 42       |
| [object-attempts.ts](../src/shared/object-attempts.ts)                                | 严格身份、视图、请求与回执 schema | 69       |
| [object-task-execution.ts](../src/ui/object-tasks/object-task-execution.ts)           | 固定请求、精确重试和生命周期防护  | 191      |
| [ObjectTaskExecutionDialog.tsx](../src/ui/object-tasks/ObjectTaskExecutionDialog.tsx) | 执行状态、中断确认、回执和刷新    | 123      |

| 验证范围                         | 结果                           | 日志                                                                             |
| -------------------------------- | ------------------------------ | -------------------------------------------------------------------------------- |
| Core `object_attempt`            | 27 passed，0 failed            | [Core 执行控制](../output/f4-control-core-final-20260924.log)                    |
| Core `scheduler`                 | 23 passed，0 failed            | [Scheduler](../output/f4-control-scheduler-20260924.log)                         |
| Desktop `object_task`            | 17 passed，0 failed            | [对象任务直接依赖](../output/f4-control-desktop-dependencies-final-20260924.log) |
| Desktop `object_attempt_runtime` | 3 passed，0 failed             | [执行路由](../output/f4-control-desktop-runtime-final-20260924.log)              |
| Desktop `business_catalog`       | 11 passed，0 failed            | [业务目录](../output/f4-control-catalog-final-20260924.log)                      |
| 前端执行控制与直接依赖           | 45 passed，0 failed，0 skipped | [前端测试](../output/f4-control-ui-tests-final-20260924.log)                     |

Core 两组过滤有交集，不相加为独立总数。27 项 attempt 测试包含 2 个由行为测试再次启动的子进程入口；回归覆盖意图与 RPC 绑定/最终冻结竞态、目标和 revision 冲突、回执完整性、已完成回执重放、Scheduler 停止后的查询及对象保权。Desktop 使用临时项目覆盖失败后查询、迟到中断、重放和项目边界。前端新增 20 项（查询 6、显式中断 5、生命周期 5、静态界面 4），45 项总范围还包括撤销、队列、定义修订界面和工作区依赖。

实际命令：

```powershell
rtk proxy cargo test --locked -p beaver-core object_attempt --lib
rtk proxy cargo test --locked -p beaver-core scheduler --lib
rtk proxy cargo test --locked -p beaver-desktop object_task
rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime
rtk proxy cargo test --locked -p beaver-desktop business_catalog
rtk proxy npx tsx --test tests/object-task-execution-query.test.ts tests/object-task-execution-interrupt.test.ts tests/object-task-execution-lifecycle.test.ts tests/object-task-execution-ui.test.ts tests/object-task-cancellation-ui.test.ts tests/object-task-queue-ui.test.ts tests/object-task-revision-ui.test.ts tests/object-task-workspace.test.ts
rtk proxy npm run typecheck
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
rtk proxy npx prettier --check src/shared/object-attempts.ts src/ui/object-tasks/object-task-execution.ts src/ui/object-tasks/ObjectTaskExecutionDialog.tsx src/ui/object-tasks/ObjectTaskList.tsx src/ui/object-tasks/ObjectTasksPanel.tsx tests/fixtures/object-attempts.ts tests/object-task-execution-query.test.ts tests/object-task-execution-interrupt.test.ts tests/object-task-execution-lifecycle.test.ts tests/object-task-execution-ui.test.ts docs/FRAMEWORK-IMPLEMENTATION-PLAN.md docs/FRAMEWORK-F4-QUEUE-LIFECYCLE.md
rtk proxy git diff --check
```

类型、Rust 格式和有效行检查均退出 0，见 [typecheck](../output/f4-control-typecheck-final-20260924.log)、[Rust 格式](../output/f4-control-rustfmt-final-20260924.log) 和 [结构检查](../output/f4-control-effective-lines-final-20260924.log)。首次类型检查发现新测试的未检查数组访问，已补断言或可选链；Desktop 旧用例将全部 `objectTask.*` 误当作同步数据方法，已按实际异步路由边界修正，并新增空输入拒绝覆盖。原失败日志保留为 [类型检查初次结果](../output/f4-control-typecheck-20260924.log) 和 [Desktop 初次结果](../output/f4-control-desktop-dependencies-20260924.log)。编译仍有既有 `unused_mut` 和 `validation/service.rs` 未读字段警告。

定向 [Prettier](../output/f4-control-prettier-final-20260924.log)、[Git 差异检查](../output/f4-control-diff-check-final-20260924.log) 及 [源码与文档核验](../output/f4-control-source-proof-20260924.log) 保存本批最终格式、未跟踪源码覆盖、UTF-8 无 BOM、行尾空白、报告 SHA-256、相对链接和 baseline 核验结果。本批开发证据使用隔离项目和假 app-server，未进行真实模型、浏览器视觉、原生界面或用户接受验收。

## medium 暂停派发增量

`objectTask.setPaused` 已接通 Core、Desktop 和任务列表，支持暂停派发与解除暂停。控制记录按项目和 run 持久化，使用独立 control revision；请求同时绑定 project/task/object/run、预期 task revision 和稳定 request ID。控制与不可变回执在同一事务内提交，control revision 的 CAS 冲突不会产生部分写入。该操作不修改 task/run/attempt revision、owner、claim token 或 generation，已有执行凭据继续有效。

领取在事务内读取暂停状态，并先为队首保留对象位置再跳过暂停项。因此同对象后项不能越过暂停队首，独立对象仍可领取。暂停可作用于 planned、queued、running、awaitingAcceptance 和 failed 的 medium；已领取的准备或执行继续完成，需要立即停止时使用已有显式中断入口。解除暂停只恢复队列领取资格，不重建已消费的 `PreparationClaim`，不恢复孤立执行、不自动重试、不批准输出，也不推进后继 fine。暂停状态在项目重开后保留；coarse 后代范围的暂停尚未接通。

精确回执重放先于当前生命周期校验：同一请求在解除暂停、领取或撤销之后仍返回原回执，不重写最新控制状态；复用 request ID 并改变载荷会被拒绝。缺失控制记录才表示默认未暂停、revision 为 0。已持久化记录必须匹配 medium/run 身份且 revision 位于 `1..=9_007_199_254_740_991`，请求的预期 control revision 必须小于该上限。损坏记录令快照和领取失败，不能被解释为默认未暂停。

Desktop 继续经 `ProjectStorageRouter` 取得已打开项目的 runtime，严格拒绝跨项目、未知字段和无效 revision；写操作通知项目变化并唤醒 Scheduler。前端快照新增 `dispatchControls`，校验项目、medium、run、对象归属和唯一性。任务列表保留生命周期标签，另行显示“派发已暂停”；控制弹窗展示已领取工作继续完成的边界，首次确认前重新查询状态。

前端首次确认后冻结目标、预期 revision 和 request ID，发送克隆载荷；结果不明时即使刷新得到新快照，仍精确重试原请求。只有明确的 task/control revision 冲突或不可控制状态才要求刷新后重新确认。回执确认与工作区刷新分开，刷新失败保留回执；关闭窗口或切换项目使旧响应失效。历史回执明确表示该次操作的记录，最新状态以任务列表为准。

### 派发控制的源码与验证

最终结构检查为 **919 sources，17 unchanged legacy files，0 violations**，历史 baseline 未改动。本批 31 个相关 Rust/TypeScript 源码及测试文件均被报告覆盖，最大为 335 有效行。主要职责如下。

| 文件                                                                                 | 职责                              | 有效行数 |
| ------------------------------------------------------------------------------------ | --------------------------------- | -------- |
| [object_task_dispatch.rs](../native/core/src/object_task_dispatch.rs)                | 持久化控制、CAS、回执与完整性校验 | 176      |
| [object_task_queue.rs](../native/core/src/object_task_queue.rs)                      | 队列事务领取和受阻队首顺序        | 329      |
| [data_dispatch_object_tasks.rs](../native/desktop/src/data_dispatch_object_tasks.rs) | 项目本地任务和派发控制路由        | 137      |
| [object_task_catalog.rs](../native/desktop/src/object_task_catalog.rs)               | 对象任务业务目录及严格输入 schema | 253      |
| [object-task-dispatch.ts](../src/shared/object-task-dispatch.ts)                     | 控制、请求与回执的共享 schema     | 50       |
| [object-task-dispatch.ts](../src/ui/object-tasks/object-task-dispatch.ts)            | 状态复核、精确重试和生命周期防护  | 192      |
| [ObjectTaskDispatchDialog.tsx](../src/ui/object-tasks/ObjectTaskDispatchDialog.tsx)  | 暂停确认、回执呈现和独立刷新      | 106      |

| 验证范围                   | 结果                            | 日志                                                                  |
| -------------------------- | ------------------------------- | --------------------------------------------------------------------- |
| Core `object_task`         | 116 passed，0 failed            | [Core 最终回归](../output/f4-pause-core-final-20260924.log)           |
| Desktop `object_task`      | 19 passed，0 failed             | [Desktop 最终回归](../output/f4-pause-desktop-final-20260924.log)     |
| Desktop `business_effects` | 1 passed，0 failed              | [Scheduler 唤醒与通知](../output/f4-pause-effects-final-20260924.log) |
| Desktop `business_catalog` | 11 passed，0 failed             | [业务目录](../output/f4-pause-catalog-final-20260924.log)             |
| 前端对象任务与直接依赖     | 108 passed，0 failed，0 skipped | [前端回归](../output/f4-pause-frontend-20260924.log)                  |

Core 新增 10 项派发控制测试，覆盖暂停队首、同对象顺序与独立对象、重开持久性、原回执重放、并发 CAS 唯一胜者、回执写入失败时事务回滚、撤销后的历史回执、执行租约保留和解除暂停不能重建启动能力。Desktop 新增 2 项，覆盖暂停/解除暂停、严格输入和项目隔离。前端新增 14 项，覆盖归属与 revision、精确重试、载荷变更防护、错误回执、刷新失败、关闭竞态和历史回执界面；108 项总范围同时包含原有对象任务行为。

扩展损坏记录测试时，持久化 revision 为 0 且未暂停的记录曾被误当作默认状态，测试先失败，见 [修复前证据](../output/f4-pause-corrupt-control-before-20260924.log)。读取已改为分别处理缺失与已持久化记录，最终 Core 回归包含此修复，并覆盖身份错配和 revision 超过安全整数上限。Desktop 原有三个空快照断言已补充 `dispatchControls: []`。编译仍有既有 `unused_mut` 和 `validation/service.rs` 未读字段警告。

最终 [typecheck](../output/f4-pause-typecheck-final-20260924.log)、[Rust 格式](../output/f4-pause-rustfmt-final-20260924.log) 和 [有效行检查](../output/f4-pause-effective-lines-final-20260924.log) 均退出 0。[Prettier](../output/f4-pause-prettier-final-20260924.log)、[差异检查](../output/f4-pause-diff-check-final-20260924.log) 和 [源码与文档核验](../output/f4-pause-source-proof-20260924.log) 记录最终格式、源码报告覆盖与 SHA-256、UTF-8 无 BOM、相对链接和 baseline 检查。本批使用开发定向测试，尚未取得真实模型、原生界面或用户接受证据。

## 持久化恢复核验增量

`objectTask.recovery` 和 `objectTask.verifyRecovery` 已从 Core 接通 Desktop 及 medium 的“执行记录”。查询直接使用已打开项目的 Runtime，Scheduler 关闭后仍可读取持久记录；没有 preparation 时返回 `null`。核验请求固定 project/task/object/run、owner、claim token、writer generation、各记录 revision、独立 recovery generation 和 request ID。查询不扫描工作区，核验只写请求与报告，不取得执行凭据、不启动任务、不释放对象写入权。

核验在短事务内保存原请求并冻结 preparation、attempt、检查点、中断记录、队列及暂停控制等记录，锁外检查文件，再在短事务中复核记录并提交不可变报告。ProjectRuntime 的所有克隆共用核验互斥锁，并发扫描返回 `OBJECT_RECOVERY_BUSY`。已完成请求按精确载荷重放，优先于当前生命周期检查；同一 request ID 换载荷被拒绝。项目重开可重试未完成请求，重试期间记录变化或损坏会完成为 `recordsCurrent: false` 的报告，避免未完成的 head 永久阻塞新核验。最大安全 recovery generation 可查询和重放，创建新请求时拒绝溢出。

文件核验覆盖冻结基准、引用版本闭包、digest、blob、输入/输出检查点及共享工作区。活动 writer 不扫描文件；缺少停止记录时不能获得处置资格。报告分别记录 writer、内容、工作区、暂停状态和问题。`reportMatchesRecords` 只比较持久记录，记录变化后报告显示过期；需要重新检查文件时必须显式发起新核验。`canDispose` 要求报告与当前记录一致、停止已记录、冻结内容已验证、工作区匹配、未暂停且无问题，只提供资格提示，人工处置和恢复执行尚未开放。

Desktop 使用严格业务目录和项目路由，核验经 Scheduler typed API 判断当前 writer，回执完成后通知变化。生产 `ObjectTaskExecutionDialog` 内展示准备状态和核验结果，支持“核验当前记录与工作区”“重试同一核验请求”和“刷新核验记录”。前端校验完整归属，结果不明时保留原请求并发送克隆载荷；项目重开后可从持久记录恢复精确重试。回执确认与刷新分开，关闭窗口或切项目使旧响应失效，过期报告不会被显示为当前处置资格。

### 恢复核验的源码与验证

最终结构检查为 **935 sources，17 unchanged legacy files，0 violations**。27 个相关源文件均为 UTF-8 无 BOM，无行尾空白，最大为 350 有效行；935 个源码的规范化 SHA-256 均与最终报告一致，历史 baseline 与 HEAD 内容一致。独立 [结构报告](../output/f4-recovery-effective-lines-fixed-20260925-024431.json) 和 [源码核验](../output/f4-recovery-source-proof-final-20260925.json) 固定本批证据，不复用后续可能覆盖的通用报告。

| 文件                                                                              | 职责                           | 有效行数 |
| --------------------------------------------------------------------------------- | ------------------------------ | -------- |
| [object_run_recovery.rs](../native/core/src/object_run_recovery.rs)               | 类型、查询及核验编排           | 146      |
| [object_recovery_records.rs](../native/core/src/object_recovery_records.rs)       | 完整记录冻结及一致性复核       | 182      |
| [object_recovery_store.rs](../native/core/src/object_recovery_store.rs)           | 持久请求、代次、回执与精确重放 | 251      |
| [object_recovery_files.rs](../native/core/src/object_recovery_files.rs)           | 冻结内容和工作区核验           | 165      |
| [scheduler_object_control.rs](../native/core/src/scheduler_object_control.rs)     | writer 识别及 typed 控制接口   | 206      |
| [object_attempt_runtime.rs](../native/desktop/src/object_attempt_runtime.rs)      | 已打开项目的执行/恢复路由      | 62       |
| [object-recovery.ts](../src/shared/object-recovery.ts)                            | 目标、请求、报告与视图 schema  | 112      |
| [object-task-recovery.ts](../src/ui/object-tasks/object-task-recovery.ts)         | 查询、精确重试及生命周期控制   | 170      |
| [ObjectTaskRecoveryPanel.tsx](../src/ui/object-tasks/ObjectTaskRecoveryPanel.tsx) | 状态、过期报告及核验入口       | 144      |

| 验证范围                         | 结果                           | 日志                                                                                     |
| -------------------------------- | ------------------------------ | ---------------------------------------------------------------------------------------- |
| Core `object_recovery`           | 8 passed，0 failed             | [新核验回归](../output/f4-recovery-core-new-20260925-023754.log)                         |
| Core `object_run_recovery`       | 4 passed，0 failed             | [既有恢复边界](../output/f4-recovery-core-existing-20260925-023754.log)                  |
| Core `object_attempt`            | 27 passed，0 failed            | [执行直接依赖](../output/f4-recovery-core-attempt-20260925-023754.log)                   |
| Desktop `object_attempt_runtime` | 7 passed，0 failed             | [执行与恢复路由](../output/f4-recovery-desktop-runtime-fixed-20260925-024431.log)        |
| Desktop `object_task`            | 19 passed，0 failed            | [对象任务直接依赖](../output/f4-recovery-desktop-dependencies-fixed-20260925-024431.log) |
| Desktop `business_effects`       | 1 passed，0 failed             | [业务效果](../output/f4-recovery-effects-20260925-023754.log)                            |
| Desktop `business_catalog`       | 11 passed，0 failed            | [业务目录](../output/f4-recovery-catalog-20260925-023754.log)                            |
| 前端恢复与执行，9 个文件         | 43 passed，0 failed，0 skipped | [前端回归](../output/f4-recovery-frontend-20260925-023754.log)                           |

Core 新核验分为 verification 5 项和 retry 3 项，覆盖锁外 I/O、活动 writer、损坏内容、工作区漂移、并发核验、记录变化、持久重试和最大代次。27 项 attempt 包含 2 个由行为测试再次启动的子进程入口。Desktop 使用真实保存/提交/入队和 fake worker，覆盖 Scheduler 关闭后的查询、项目重开、严格输入、对象保权和项目隔离。前端覆盖契约、查询、精确重试、关闭/切项目竞态及生产 `ObjectTaskExecutionDialog` 的 SSR 集成。

实际命令：

```powershell
rtk proxy cargo test --locked -p beaver-core object_recovery --lib
rtk proxy cargo test --locked -p beaver-core object_run_recovery --lib
rtk proxy cargo test --locked -p beaver-core object_attempt --lib
rtk proxy cargo test --locked -p beaver-desktop object_attempt_runtime
rtk proxy cargo test --locked -p beaver-desktop object_task
rtk proxy cargo test --locked -p beaver-desktop business_effects
rtk proxy cargo test --locked -p beaver-desktop business_catalog
rtk proxy npx tsx --test tests/object-recovery-contract.test.ts tests/object-task-recovery-query.test.ts tests/object-task-recovery-retry.test.ts tests/object-task-recovery-lifecycle.test.ts tests/object-task-recovery-ui.test.ts tests/object-task-execution-query.test.ts tests/object-task-execution-interrupt.test.ts tests/object-task-execution-lifecycle.test.ts tests/object-task-execution-ui.test.ts
rtk proxy npm run typecheck
rtk proxy cargo fmt --all -- --check
rtk proxy npm run check:effective-lines
rtk proxy npx prettier --check src/shared/object-recovery.ts src/ui/object-tasks/object-task-recovery.ts src/ui/object-tasks/ObjectTaskRecoveryPanel.tsx src/ui/object-tasks/object-task-execution.ts src/ui/object-tasks/ObjectTaskExecutionDialog.tsx tests/fixtures/object-recovery.ts tests/object-recovery-contract.test.ts tests/object-task-recovery-query.test.ts tests/object-task-recovery-retry.test.ts tests/object-task-recovery-lifecycle.test.ts tests/object-task-recovery-ui.test.ts docs/FRAMEWORK-IMPLEMENTATION-PLAN.md docs/FRAMEWORK-F4-QUEUE-LIFECYCLE.md
rtk proxy git diff --check
```

类型、Rust 格式及有效行检查均退出 0，见 [typecheck](../output/f4-recovery-typecheck-20260925-023754.log)、[Rust 格式](../output/f4-recovery-rustfmt-fixed-20260925-024431.log) 和 [有效行检查](../output/f4-recovery-effective-lines-fixed-20260925-024431.log)。[修正批次退出码](../output/f4-recovery-fixed-checks-20260925-024431.json) 同时记录 Desktop 两组、Rust 格式及结构检查的退出 0；Rust 格式成功日志为空。首次 Desktop 失败来自非法输入的错误前缀断言与把异步恢复 API 归入同步分发的旧测试，已按目录校验、Serde、Core 及实际异步边界修正；原日志 [路由](../output/f4-recovery-desktop-runtime-20260925-023754.log)、[直接依赖](../output/f4-recovery-desktop-dependencies-20260925-023754.log) 和 [格式](../output/f4-recovery-rustfmt-20260925-023754.log) 保留。编译仍有既有 `unused_mut` 和 `validation/service.rs` 未读字段警告。

收尾复用了源码一致的功能测试结果，未重复运行功能测试。定向 [Prettier](../output/f4-recovery-prettier-final-20260925.log)、[Git 差异检查](../output/f4-recovery-diff-check-final-20260925.log) 和 [文档核验](../output/f4-recovery-document-proof-final-20260925.json) 记录最终格式、文档编码、相对链接及章节锚点。日志日期使用 UTC，实施日期按本地 2026-09-24 记录。没有运行真实模型、Godot/Blender、浏览器视觉或完整原生验收。

## 交付边界与下一入口

本批继续使用项目本地 object-task 实体，保留旧任务协议、已有草稿、历史记录及工作区其他改动。没有进行数据搬迁、版本升级、原生发布或完整原生验收；版本仍为 `0.1.19.36`，未构建或启动新的 Beaver.exe。当前证据覆盖 Core、Desktop 分发和前端逻辑，尚未取得本批运行界面或用户接受证据。

人工取消现在消费持久化核验报告，并在执行动作前重新核验当前记录和工作区。coarse 范围暂停已接通，队列重排请求回执、CAS 和相邻锚点拖动已接通；下一入口包括恢复执行和受管引擎会话；后继 fine 需要 F5 门槛协议。查询不重新扫描文件，解除暂停不授权恢复执行，取消不接受或发布对象。

F4.2 已接通队列请求回执、CAS、相邻锚点拖动及阻塞说明，见下方队列重排增量。F4.4 已提供执行查询、显式中断、medium/coarse 持久化暂停派发和持久化恢复核验入口，恢复执行、受管引擎会话仍未实现；重开项目后的人工取消及可选工作区删除已接通。本切片只覆盖普通文件加工及 Windows 进程树，其他平台启动前拒绝对象执行；交互回答、自动重试和后继 fine 尚未开放。F7 负责接受/发布衔接、正式导入提交及文件取消处置，成功 turn 不会提前发布或接受对象，metadata 取消不能证明文件处置已完成。F1-F3 的剩余迁移、规划交互和生产导航边界继续按总体计划跟进。

## 人工取消与可选工作区删除增量

2026-09-24：生产恢复面板支持取消并保留成果，以及显式确认后的取消并删除工作区。Desktop 的 objectTask.disposeRecovery 使用严格 choice 字符串枚举。完整请求与 requestId 用于响应丢失后的精确重试；成功回执和 attempt 历史在项目重开后仍可查询。取消释放对象占用并取消未启动 fine，不重写已冻结的执行结果。

Core 在删除前复核当前元数据、停止证据、内容与工作区；Running 和 Completed journal 以比较并交换方式持久化，再提交取消事务。删除后进程中断可在项目重开后完成提交。HOME、共享 blob 和其他任务工作区保留；路径穿越被拒绝。部分删除后的现场漂移会阻塞，必须检查现场并重新核验；Completed journal 遇到重新创建的工作区拒绝再次删除，保留原请求等待处理。已完成回执重放不会触碰新建文件。

本批验证：Core object_recovery --lib 16 passed；Desktop object_attempt_runtime 7 passed；前端恢复、处置及执行界面 29 passed。证据见 [Core](../output/f4-disposition-core-current.log)、[Desktop](../output/f4-disposition-desktop-current.log)、[前端](../output/f4-disposition-ui-current.log)。覆盖真实删除、保留取消、删除后中断重开、Running 状态重开、现场漂移、元数据变化、活动 writer、重新创建工作区、确认和请求重放。类型检查通过；未运行真实模型、Godot/Blender、浏览器视觉或完整原生验收。F4.1-F4.4 保持未勾选。

## coarse 范围暂停派发增量

2026-09-24：生产任务树及确认对话框支持粗修暂停/解除暂停，使用 objectTask.setCoarsePaused。独立控制记录与不可变请求回执保存在项目数据库，task revision 与 control revision 在同一事务内比较并交换；快照必须包含 coarseDispatchControls，缺失或身份不一致时拒绝消费。

领取事务读取中修当前所属粗修的暂停状态，覆盖暂停后新建的子任务；先保留对象队首再跳过暂停项，防止同对象后项越过。其他对象的独立任务继续运行。已领取的准备/执行可以收尾，粗修操作不重写中修控制、租约或任务状态；项目重开后保留控制，解除粗修暂停不能覆盖中修独立暂停，也不授权恢复执行。

前端展示所属粗修暂停提示。请求结果不明时保留完整原请求供精确重试，历史回执和当前快照分离；revision 冲突要求刷新后重新确认。Desktop 使用严格字段校验、显式项目路由、项目隔离和 scheduler 唤醒。

本批定向验证：Core object_task 136 passed；Desktop object_task 20 passed；business_effects 1 passed；前端契约、快照、派发确认、精确重试与生产 UI 28 passed。证据见 [Core](../output/f4-coarse-core.log)、[Desktop](../output/f4-coarse-desktop.log)、[业务效果](../output/f4-coarse-effects.log)、[前端](../output/f4-coarse-ui.log)。类型、Rust 格式、Prettier 和有效行检查通过，结构为 945 sources、17 unchanged legacy files、0 violations。首次回归发现测试依赖了错误的跨对象领取顺序，另两处旧断言未同步新增 API 数量及运行时分发边界，修正后通过。未构建或启动新 EXE，未运行完整原生验收；F4.1-F4.4 保持未勾选。

## 队列重排与生产插入预览增量

2026-09-24：F4.2 已接通。生产执行队列支持未入队中修入队、阻塞说明、原生拖动虚影与插入位置预览、键盘上移/下移，以及显式确认排序。执行与历史项不可排序，任务树中的计划位置不随执行队列重排改变。

`objectTask.queueView` 在事务内返回版本和阻塞原因；`objectTask.reorder` 使用完整请求、requestId、版本 CAS 和相邻 queued 锚点。独立顺序 revision 防止顺序变回原样后接受过期请求；队列、回执与 revision 原子保存，溢出回滚。已确认回执可在后续领取后精确重放，响应丢失保留原请求；并发变化要求刷新并重新确认。重开保留顺序，活动租约不变，受阻队头不能被同对象后项越过。

定向验证：Core object_task 142 passed（含 6 项重排回归）；Desktop object_task 20 passed；business_effects 1 passed；前端队列契约、预览、确认、重试、卸载隔离与派发相关测试 18 passed。证据见 [Core](../output/f4-reorder-core.log)、[Desktop](../output/f4-reorder-desktop-fixed.log)、[业务效果](../output/f4-reorder-effects.log)、[前端](../output/f4-reorder-ui.log)。首次 Desktop 检查发现目录校验器不支持 anyOf，已改为项目既有 nullable type 数组并通过复验；失败日志保留在 [首次 Desktop](../output/f4-reorder-desktop.log)。

类型检查、Rust 格式、定向 Prettier 和有效行检查通过；结构为 954 sources、17 unchanged legacy files、0 violations。见 [类型](../output/f4-reorder-typecheck.log)、[格式](../output/f4-reorder-format.log)、[Rust 格式](../output/f4-reorder-rustfmt.log)、[结构](../output/f4-reorder-structure.log)。未构建或启动新 EXE，未进行真实拖动视觉、真实模型或完整原生验收。F4.2 按开发切片完成勾选，F4.1、F4.3、F4.4 仍保持未勾选；下一入口为恢复执行及受管引擎会话，后继 fine 仍依赖 F5 门槛协议。
