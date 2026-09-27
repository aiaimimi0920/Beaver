# Beaver 文档

2026-09-25 最新增量：[F5.3 尝试内只读回调](FRAMEWORK-F5-CALLBACK-EVIDENCE.md)。生产 Codex 执行链可发现当前能力、查询冻结定义并读取冻结输入，同一轮继续加工后冻结输出并等待验收。Core 42 项通过，1 项真实 Godot 用例按条件忽略；受管工具回调、细粒度操作输入输出与正式原生验收仍待完成。

2026-09-25 最新增量：[F5.3 操作通知轨迹](FRAMEWORK-F5-TRACE-EVIDENCE.md)。执行记录可查询持久化操作类别、阶段和结果，保留最先收到的 256 条通知并明确显示截断；项目存储重开、身份隔离、过期 writer 和迟到响应已有定向证据。Core 3 项、Desktop 3 项、前端 8 项及类型、格式、有效行检查通过；完整回调、能力发现与正式原生交互验收仍待完成。

2026-09-25 最新增量：[F5.3 冻结文件正文](FRAMEWORK-F5-FROZEN-CONTENT-EVIDENCE.md)。执行记录可查看冻结输入/输出正文，工作区变更后仍可读；损坏、二进制与超限明确显示，迟到结果不覆盖当前选择。Core 2 项、Desktop 3 项、前端去重 18 项及类型、格式、有效行检查通过，正式原生交互验收仍待完成。

2026-09-25 最新增量：[F3.3 Codex 标题建议](FRAMEWORK-F3-TITLE-EVIDENCE.md)。生产草稿编辑器可请求候选并明确采用，仅修改目标标题；过期或失败结果保留草稿，后台请求限制并发、超时并参与退出清理。Core 8 项、Desktop 1 项及前端 21 项通过；真实提供方与正式原生交互验收仍待完成。

2026-09-25 最新增量：[F3.3 稳定插入锚点](FRAMEWORK-F3-INSERTION-EVIDENCE.md)。前后插入跟随任务 ID 并保留最新草稿编辑，删除锚点时明确报错；保存后重开顺序保留。新增 4 项及相邻 22 项测试通过，真实 Codex 标题建议和正式交互验收仍待完成。

2026-09-25 最新增量：[F3.3 提交草稿锁定与下一份草稿](FRAMEWORK-F3-DRAFT-LOCK-EVIDENCE.md)。提交后保留只读计划，重开可恢复；显式开始下一份草稿后恢复编辑，原任务与历史保留，精确重试不会覆盖后续编辑。Core 26 项、Desktop 9 项和前端 24 项通过；标题 AI 建议、稳定插入锚点和正式交互验收仍待完成。

2026-09-25 最新增量：[F3.4 生产任务泳道](FRAMEWORK-F3-BOARD-EVIDENCE.md)。全层级真实任务按状态分列，支持责任树展开、阶段事实计数及执行记录入口；此前已接通 [对象与制造导航](FRAMEWORK-F3-NAVIGATION-EVIDENCE.md)。必要工作项进度语义与交互验收仍待完成，尚未生成新原生程序。

当前开发计划：[Beaver 实际框架实现开发计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)，创建于 2026-09-16，状态更新至 2026-09-24。基于已确认 UI 和现有原生代码，按 F0-F9 推进项目存储、对象与任务、串行制造、工具验收、发布导入、真实预览及迁移收口。F0 已实现并通过定向检查，F1、F2、F3、F4 进行中，F3.1 已完成，F5-F9 待实施。[F2 导入准备记录](FRAMEWORK-F2-IMPORT-PREPARATION.md) 记录来源冻结、内容校验和准备边界；[F3.1 任务持久化记录](FRAMEWORK-F3-TASK-PERSISTENCE.md) 记录对象任务、项目草稿、事务提交和真实查询；[F3.2 人工规划与撤销记录](FRAMEWORK-F3-TASK-PLANNING.md) 保留前批草稿编辑、责任范围撤销、冲突恢复及 60 项测试证据；[F3.2 定义修订记录](FRAMEWORK-F3-TASK-REVISIONS.md) 记录未启动任务编辑、复核、不可变历史、重试恢复及 100 项定向测试。F3.2 的 Codex 追问/提案已接通，正式原生验收仍待完成。最新 [F4 队列生命周期记录](FRAMEWORK-F4-QUEUE-LIFECYCLE.md) 记录领取、收尾、metadata 取消和前端状态接入，以及本批定向验证结果。

当前 UI 基准：[Beaver.exe，内部版本 0.1.19.36](../release/Beaver-native-0.1.19.36-win32-x64/Beaver.exe)。采用“创作”“对象”“制造”“测试”入口，保留“资料”“游戏”“创作环境”“设置”等页面；“素材”和“界面浏览”入口已移除。该版本的对象与制造主要使用 Mock 数据，测试页已接原有业务 API。当前源码中的对象目录、详情、外部项目导入准备及对象任务列表已接入真实查询；对象任务支持项目草稿编辑、Codex 规划、保存提交、撤销未启动项、定义修订与历史查询，以及保留本地输入的冲突恢复。队列领取、待验收和失败状态已接入共享快照及界面，实际调度和资源生命周期待 F4 后续完成。本批尚未生成或发布新的原生程序。初版 [预览及审阅说明](ui/OBJECT-UI-PREVIEW.md) 保留为历史记录，不代表当前导航。

产品依据：[对象创作框架开发设计](OBJECT-CREATION-FRAMEWORK-DESIGN.md) 与 [对象导入和项目存储规范](OBJECT-IMPORT-AND-PROJECT-STORAGE.md)。任务层级、对象串行队列、固定引用、实际预览和人工介入规则继续适用；实施顺序统一以当前 F0-F9 计划为准。

上一轮开发计划：[Beaver 与 Codex 双向工作流](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md)，2026-09-14。通用框架代码已完成，包括双向调用、线性审批、持久长操作、插件适配协议、版本化检查、owner 视觉评价、历史输入与真实工具轨迹。具体 NPR 小阶段、第三方插件适配器和工具链留待后续开发；新增能力的原生交付验收按该轮范围暂未执行。

上一轮交付：[通用框架契约](WORKFLOW-FRAMEWORK.md)与[完成及定向验证记录](WORKFLOW-FRAMEWORK-COMPLETION.md)，当前回调协议版本 5。前批记录依次为 [源文件与依赖输入清单](ASSET-WORK-INPUTS.md)、[子任务与执行尝试](ASSET-WORK-ATTEMPTS.md)、[原生联动验证](ASSET-DELIVERY-NATIVE-VALIDATION.md)、[阶段候选文件与审批](ASSET-DELIVERIES.md)、[任务回调](TASK-CALLBACKS.md)。

设计文档：[游戏代码验收与画面流程开发设计](GAME-VALIDATION-DESIGN.md)，2026-09-13。保留 GUT 代码验收、独立 Tab、四类可重播画面流程、对话反馈及正式发布诊断开关的需求和讨论记录。

实现记录：[游戏验收功能实现与验证](GAME-VALIDATION-IMPLEMENTATION.md)，2026-09-13。功能已在独立分支实施，记录使用方式、真实引擎证据、集成状态及尚未执行的原生交互验收。

设计及实现记录：[资产制作任务窗口开发设计](ASSET-TASK-WINDOW-DESIGN.md)，2026-09-13。记录实时 Blender 预览、三种修改定位方式、流程干预及与人物 skill 的交接；已包含实现与部分真实验证记录，具体完成边界以该文为准。后续对象创作面板在此基础上调整入口和归属。

当前正在进行 Electron 到 Tauri/Rust 的迁移。最新 R8 原生预览完整目录为 15.66 MB，新增共用桌面逻辑的 44 项业务 API/MCP 操作，并通过真实程序验证；导入 UI、完整回退与产品验收仍未完成。下列 0.1.x 文档保留各轮历史契约，不作为最新完成状态清单。

- [最新选项交互修复与两轮真实交互验收](NATIVE-RELEASE-R9.md)。
- [R8 原生发布与 API/MCP 验证证据](NATIVE-RELEASE-R8.md)。
- [R7 会话迁移与显式启用证据](NATIVE-RELEASE-R7.md)。
- [完整数据备份、恢复格式和迁移边界](DATA-MIGRATION.md)。
- [业务 API、MCP 接入、共享执行逻辑与当前边界](BUSINESS-API.md)。
- [原生迁移的完整进度与剩余门槛](NATIVE-MIGRATION.md)。

- [使用与开发入口](../README.md)。
- [0.1.8 引擎能力功能包规划](./UPDATES-0.1.8.md)。
- [全部引擎能力包、来源键和交互含义](./ui/ENGINE_PACKAGES.md)。
- [0.1.7 项目规划交互原型](./UPDATES-0.1.7.md)。
- [项目规划控件含义与未来交付契约](./ui/PROJECT_BLUEPRINT.md)。
- [0.1.6 本机 API 与游戏创作目录](./UPDATES-0.1.6.md)。
- [游戏类型、题材与功能块目录](./GAME_CATALOG.md)。
- [0.1.5 侧栏图标固定位置](./UPDATES-0.1.5.md)。
- [0.1.4 正式图标「一句成游」](./UPDATES-0.1.4.md)。
- [0.1.3 统一通知与图标第一轮](./UPDATES-0.1.3.md)。
- [0.1.2 Loom 壳层对齐](./UPDATES-0.1.2.md)。
- [0.1.1 界面与工具发现修复](./UPDATES-0.1.1.md)。
- [产品需求与首版范围](./PRODUCT.md)。
- [本地架构、文件恢复和安全边界](./ARCHITECTURE.md)。
- [AI 服务接入与协议](./AI_SERVICES.md)。
- [验证记录](./VALIDATION.md)。
- [后续目标](./ROADMAP.md)。

- [UI 设计文档入口](./ui/README.md)：Neuro / Loom 设计资料副本、Beaver 适用边界和阅读顺序。
- [Beaver UI 设计规范](./ui/DESIGN_SYSTEM.md)：继承的视觉规则与本项目语义映射。
- [UI 实现提示词](./ui/AI_PROMPT.md)：供后续界面开发与审阅使用。

`ui/references` 保留上游原始副本，不随应用实现改写或污染 Neuro/Loom 来源。
