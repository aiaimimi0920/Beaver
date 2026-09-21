# F0：契约与切换边界实施记录

日期：2026-09-16。所属计划：[实际框架实现计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) F0。

本批建立新任务身份、旧执行器阻断、只读项目状态及生产页面的数据边界，已通过相关契约、编译和结构检查。项目本地数据库、对象查询、对象队列和制造执行尚未实现；能力开关保持关闭。原生窗口及引擎验收尚未开展。

## 1. 本批实现

| 范围     | 实现及源码                                                                                                                                            |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| 身份契约 | `src/shared/object-framework.ts`、`native/core/src/object_framework.rs`；同一份 JSON 样本验证 TypeScript / Rust 接受与拒绝规则                        |
| 项目识别 | `native/core/src/object_framework_status.rs`；只检查登记路径和 `.beaver/project.json`，不创建目录、不打开数据库、不启动迁移                           |
| 原生入口 | `native/desktop/src/business_catalog.rs`、`data_dispatch.rs`；增加只读 `objectFramework.status`，将保留身份传到核心校验                               |
| 执行阻断 | 任务创建、计划、准备、执行器、完成/合入、继续/接受/回滚、派生关系、资产启用、回调、框架操作与交付入口拒绝新身份                                       |
| 生产页面 | `ObjectFrameworkWorkspace.tsx`、`use-object-framework-status.ts`、`FrameworkAvailability.tsx`；真实读取状态，分别显示加载、未选项目、错误、未接通原因 |
| 演示边界 | `src/ui/App.tsx` 使用生产入口；`src/ui/main.tsx` 的显式 `?preview=objects` 入口保留已确认的 UI 演示。工具栏的展示部分不依赖 Mock 对象                 |

没有数据查询能力时不显示演示人物，也不将“查询未实现”显示成“项目没有对象”。导入、生成、标注、任务展开等依赖能力未满足的按钮不可执行。项目切换及查询重叠使用项目 ID、请求序号和销毁标记拦截晚到响应；订阅通知后重新读取。

## 2. 身份、基准与权限契约

新框架任务使用任务记录中的 `objectFramework` 字段，版本为 `schemaVersion: 1`。这是协议版本，与应用内部版本 `0.1.19.36`、项目存储版本及旧回调协议版本分别管理。

| layer    | 必要字段                                                        | 语义                                                      |
| -------- | --------------------------------------------------------------- | --------------------------------------------------------- |
| `coarse` | `schemaVersion`                                                 | 粗修目标，后续关联拆分计划和必要工作项                    |
| `medium` | `schemaVersion`, `objectId`, `baseline`                         | 一个对象的一次修改建议；独立中修允许无粗修父任务          |
| `fine`   | `schemaVersion`, `objectId`, `mediumTaskId`, `runId`, `stageId` | 属于一个中修及制作轮次的精修任务，阶段和任务仍使用不同 ID |

身份 ID 长度为 1-128 个 ASCII 字符，仅接受字母、数字、`_ . : -`。这些 ID 不能直接充当文件路径。字段严格校验，不允许把不同层级的字段混合；未来版本、未知层级、缺字段、额外字段均拒绝。

`baseline` 是三选一的结构：`{basePolicy: "latestAccepted"}`、`{basePolicy: "pinnedVersion", selectedVersionId}`、`{basePolicy: "empty"}`。只有指定版本策略携带 `selectedVersionId`。Rust 使用空结构变体，避免 serde 的无字段枚举变体忽略额外字段；共享样本覆盖这一协议差异。

F4 实现领取时，再保存实际解析的 `resolvedBaseVersionId`、领取时的当前接受版本和写入代次。排队时不解析默认最新基准；切换查看版本不更改工作基准。已有尝试的基准或提示词修改必须产生新尝试，保留旧输入及证据。

需求编辑锁只限制编辑；对象写入权由事务领取并由中修持有。暂停、追问、失败待恢复、等待验收均不释放写入权。精修继承该中修的同一工作区并串行写入。发布或完成取消处置后才交权。对象内容、引擎内拾取 ID、任务、阶段、尝试、候选和版本不得互相借用身份。

### 旧数据与执行隔离

- 字段不存在时维持旧协议；`assetTask: true` 本身不代表新框架任务。
- 只要存在 `objectFramework` 字段，即使其值为 `null` 或格式错误，也不能降级到旧执行器。错误身份返回含 `INVALID_OBJECT_FRAMEWORK` 的核心错误；有效新身份返回含 `OBJECT_FRAMEWORK_DISABLED` 的核心错误。
- 创建检查发生在项目读取及文件准备之前；执行器检查发生在模型调用日志及进程启动之前；回调检查发生在事务内、旧回执重放之前。
- 旧计划不能把新任务作为已完成依赖来领取，不能为新身份父任务准备旧工作副本，也不能借旧父任务的自动审批更新新身份子任务。
- 只读查询、历史观察和已存在长操作的取消仍可用于诊断与收尾。开放执行需要 F4-F6 的队列、轮次身份和阶段门槛；不能通过删除标记或修改应用版本来启用。
- 存储迁移不自动赋予旧任务粗中精语义。旧资产子工作映射到统一精修由 F5 显式转换；转换前不由新调度器执行，不同时使用两套执行路径写相同文件。

## 3. 项目存储与路由契约

目标目录固定为 `.beaver/`，清单为 `project.json`，数据库为 `project.sqlite`。清单当前识别字段为 `schemaVersion: 1`、`storageVersion: 1`、`projectId`；探测只读，因此不会丢弃历史未知字段。F1 的读写及迁移必须保留未知字段。

| 子目录        | 用途                                             |
| ------------- | ------------------------------------------------ |
| `content/`    | 不可变内容、版本清单及引用内容                   |
| `workspaces/` | 制作工作副本、检查点、冻结任务上下文             |
| `previews/`   | 可重建预览缓存和索引                             |
| `evidence/`   | 验收报告、原始画面、冻结标注；不能按预览缓存清理 |
| `operations/` | 迁移、发布、导入的持久准备与恢复记录             |
| `cache/`      | 可重建缓存                                       |

宿主保留项目位置登记、偏好、凭据和工具安装配置。项目登记解析为绝对位置后核验清单 ID 和版本，再选择项目数据库。项目内部路径使用相对路径。未知版本、ID 冲突、迁移未提交及无效控制目录必须阻断写入，不能悄悄回落到全局库。外部项目导入读取器不登记、不迁移、不执行源项目脚本。

F0 的 `objectFramework.status({projectId})` 返回以下状态；这些状态只说明探测事实：

| storage.state | 含义                                                   |
| ------------- | ------------------------------------------------------ |
| `offline`     | 登记路径不可用或非绝对目录                             |
| `legacy`      | 项目没有 `.beaver` 目录，需要后续初始化或副本迁移      |
| `detected`    | 清单版本及项目 ID 已识别；未验证数据库或读取对象       |
| `invalid`     | 清单缺失、过大、格式错误、版本/ID 不符或文件类型不合法 |

探测拒绝符号链接控制目录及清单，限制清单为 64 KiB。三个 capability：`objectsRead`、`manufactureRead`、`execution` 当前均为 `false`。返回的阻塞码为 `PROJECT_STORAGE_NOT_ROUTED`、`OBJECT_QUERIES_UNAVAILABLE`、`OBJECT_FRAMEWORK_DISABLED`。F1 后续批次为响应增加 `storage.routed`；当桌面分派通过项目路由取得项目 Store 时不再返回 `PROJECT_STORAGE_NOT_ROUTED`，见 F1 实施记录。状态不创建数据库，也不将 `detected` 等同于数据库可用。

### 现有数据到项目的搬迁清单

以下是 F1 的实现输入，尚未执行搬迁。当前实体、事件和调用仍在宿主 `beaver.sqlite`；现有迁移包中的 `data/` 目录也没有自动变成项目 `.beaver`。

| 现有数据                                             | 归属及目标处理                                                             | 源码依据                                              |
| ---------------------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------- |
| `project`、根 `beaver.project.json`                  | 清单/项目元数据迁到项目；宿主保留位置索引；迁移保留原件                    | `projects.rs`, `store.rs`                             |
| `task`、父子/依赖关系、快照引用                      | 按 `projectId` 归属；保留 ID、未知字段和历史语义，跨项目关系列为冲突       | `task_create.rs`, `task_plan.rs`, `task_relations.rs` |
| `asset-task`、阶段/子工作/候选/审批                  | 由任务 ID 解析项目；嵌入式记录完整保留，F5 才进行语义转换                  | `asset_task.rs`, `asset_work*`, `asset_delivery*`     |
| `asset-reference`                                    | 从已验证的项目/任务/会话关联解析；关联不足列为待处理，不能猜归属           | `asset_reference*`                                    |
| feature 状态、journal 文件操作                       | 核对项目及任务引用；未完成日志和备份一同复制，启用前核对恢复状态           | `features.rs`, `journal.rs`                           |
| framework 操作、检查、评价、轨迹及恢复               | 包括动态 kind，按任务关联搬迁；保留 request ID、revision、取消/恢复事实    | `framework*`                                          |
| callback 回执及 revision                             | 按任务 ID 及动态 receipt kind 搬迁；保留原始请求、响应、thread/turn        | `task_callback.rs`                                    |
| `events`、`calls`                                    | 按 `events.task` 及 calls 的 task/project 关联；项目无关宿主调用留全局     | `store.rs`, `call_log.rs`                             |
| `blobs/<sha256>`                                     | 根据快照及内容引用收集完整闭包，复制和复核 hash；不删除全局原件            | `files.rs`                                            |
| `workspaces/<task-id>`                               | 工作区及检查点完整复制到项目，路径读写器同时切换                           | `task_plan.rs`, `task_create.rs`                      |
| `codex/<task-id>/sessions/*.jsonl`、`state_5.sqlite` | 按任务归属迁移可恢复会话；凭据和宿主安装不随会话复制                       | `codex_home.rs`, `migration_bundle`                   |
| 工作区 `.beaver-context`                             | 保留父任务上下文、反馈、修复、媒体及 source 冻结副本                       | `task_plan.rs`, `validation/feedback_context.rs`      |
| 结构规则文件和快照基线                               | 保留项目规则、任务原始快照及迁移来源，不更新结构 baseline 接受新增超限代码 | `code_structure/task.rs`, `codex_home.rs`             |

验收领域不能只按 task ID 筛选；无任务的项目检查和发布记录也属于项目：

| 实体                                        | 可验证归属与依赖                                                                                                                                                                             |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `validationRun`                             | 必须有 `projectId`；`taskId`、`releaseId`、flow、baseline 可空；保留 snapshot、checks、judgments、confirmations。证据位于 `validation/<run UUID>`，目标为项目 evidence，由路径适配器统一切换 |
| `validationCoverage`                        | `projectId` + `taskId`；保留 `snapshotId`、`codeRunId`、`flowIds`、`runIds`，无独立外部文件                                                                                                  |
| `validationFlow` / `validationFlowRevision` | 均携带 `projectId`；revision key 为 `flow.id:flow.revision`，flow ID 来自 project ID 和 definition key 的摘要；`definition.taskIds` 可以为空                                                 |
| `validationManifest`                        | 以项目 ID 为 key，保存成功导入的 hash/time；还要保留项目 `beaver.validation.json` 及快照中的历史文件                                                                                         |
| `validationBaseline`                        | `projectId`、`flowId`、`runId`、`snapshotId`、`evidenceIds`、`previousId`；证据依赖原 run 目录，不能只搬 baseline 行                                                                         |
| `validationFeedback`                        | `projectId`、`runId`、`snapshotId`，可有 `flowId`；`taskId` 创建反馈任务后才回填。保留选择与来源，冻结副本随所属任务工作区搬迁                                                               |
| `validationRelease`                         | `projectId`、snapshot、`flowIds`/flows、`scopeId`、`runIds`、preset/policy/exports；没有直接 task 关联，不能漏掉游戏导出入口的同种记录                                                       |

依据：`validation/model.rs`、`repository.rs`、`coverage.rs`、`manifest.rs`、`feedback.rs`、`feedback_context.rs`、`release.rs`，以及 `native/desktop/src/game_runtime.rs`。F1 需在副本中验证引用闭包、证据摘要、未知字段和源文件不变，再通过持久 pending 回执切换路由；未归属或矛盾记录保留并报告，不能丢弃。切换后只允许一个权威写入位置。

## 4. 传输、事件、幂等及冲突

`objectFramework.status` 已进入原生业务目录；桌面、HTTP、MCP 复用业务路由和核心查询。本批没有新增对象创建/执行/审批 API。`task.create` 的保留 `objectFramework` 参数可传到核心，但核心必定拒绝执行。JSON Schema 根节点显式为 object，内部层级仍由核心严格校验，不能依赖原生目录的浅层类型校验。

旧 `task.callback` 协议仍为 5。宿主绑定 task/thread/turn；事务中先核验身份，再查回执，同 request ID、同内容及执行身份返回原响应，不同内容拒绝；新操作要求匹配 revision。记录报告不代表文件已验证、阶段已接受或任务已完成。新增 object/run/attempt/写入代次绑定在 F5 落实后才更新执行协议。

当前通用传输错误及错误消息沿用原协议；`INVALID_OBJECT_FRAMEWORK` / `OBJECT_FRAMEWORK_DISABLED` 是核心诊断前缀，尚不是所有传输层统一的顶层 error.code。UI 不靠解析错误文本获得状态，使用已实现的 capability/blocker 结构。

后续写命令按下面约束实现，不提前在目录中公布空接口：

- 使用稳定 `requestId`、目标身份及 `expectedRevision`；传输重试复用原请求 ID。请求与身份相同返回原回执，复用 ID 提交不同内容返回冲突。
- 队列排序使用相邻持久 ID 和 queue revision；发布使用领取时记录的接受版本作比较；旧代次、已取消尝试及过期 revision 拒绝提交，保留草稿/成果。
- 冲突响应需要可机器识别的原因、当前 revision 和重查目标；不支持 force 绕过技术检查或写入权。准确的公共字段在对应批次实现时加入共享类型及目录测试。
- 数据库事实和领域事件同事务保存；事件带项目、实体及 revision。UI 通知只触发重新读取权威状态，断线重连、切换项目和轮次均不靠本地进度猜测。F0 复用已有订阅，无新增未落库的制造进度事件。

## 5. 验证与剩余项

本批只运行相关编译、契约和结构检查。日志位于仓库 `output/`，没有执行游戏创作全流程，没有构建或启动新 EXE，没有变更内部版本。

| 命令/检查                                                                                                                                                            | 结果与证据                                                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `rtk proxy npm run typecheck`                                                                                                                                        | 通过；`output/f0-typecheck.log`                                                                                                               |
| `rtk proxy npx tsx --test tests/object-framework.test.ts`                                                                                                            | 4 项通过；`output/f0-ui-contract-test.log`                                                                                                    |
| `rtk proxy npm run build:native:ui`                                                                                                                                  | 通过，生成原生目录所需 schemas；`output/f0-build-native-ui.log`                                                                               |
| `rtk proxy cargo test --locked -p beaver-core --test object_framework_boundary --test object_framework_status --test object_framework_executor --test task_callback` | 14 项通过；`output/f0-native-core-tests.log`。覆盖新旧身份、无副作用拒绝、父子/依赖混用、回执重放、只读状态及不启动模型                       |
| `rtk proxy cargo test --locked -p beaver-desktop business_catalog::tests`                                                                                            | 5 项通过；`output/f0-native-catalog-tests.log`                                                                                                |
| `rtk proxy npm run check:native`                                                                                                                                     | 通过；`output/f0-check-native.log`                                                                                                            |
| `rtk proxy npm run check:effective-lines`                                                                                                                            | 通过，591 个文件、0 个违规；`output/f0-effective-lines.log`、`output/effective-code-lines.json`                                               |
| 修改范围内 Prettier / rustfmt                                                                                                                                        | 已执行；`output/f0-format-ts*.log`、`output/f0-format-rust*.log`、`output/f0-format-baseline.log`；实施文档检查见 `output/f0-format-docs.log` |

本批修复了共享样本发现的 Rust baseline 额外字段接受问题。TypeScript 检查同时暴露并修正了新页面图标名及演示组件的空值读取，未扩展演示功能。桌面测试最初把 Zod 判别联合输出写成 `anyOf`，已按实际生成的 `oneOf` 校验三个层级并重跑通过。

主要新模块有效行数：核心身份 104、只读状态 86、共享 TS 契约 60、生产工作区 93、查询 hook 53、通用工具栏 196。桌面目录为 255 行，继续聚合业务 schema 与浅层传输校验；没有新增超限文件，没有修改历史 baseline。

尚未验证桌面窗口、HTTP/MCP 的真实运行轮次及引擎显示；核心用例和目录用例不能替代这些证据。F1 下一步须先完整阅读 [现有迁移契约](DATA-MIGRATION.md)，在本清单基础上实现项目数据库路由、备份/副本迁移及启用恢复；不提前开放对象执行。F1-F9 仍未完成。
