# 通用工作流框架

日期：2026-09-14。当前实现为原生 Beaver 的通用框架代码，回调协议版本 5。
本轮不包含 NPR 制作模板、具体第三方插件适配器、专用 Skill 调用链或新的原生交付验收。
验证结果见 [本轮完成记录](WORKFLOW-FRAMEWORK-COMPLETION.md)，历史批次见
[开发计划](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md)。

## 状态归属与入口

Beaver 持久保存任务、线性阶段、同工作副本串行子任务、attempt、冻结候选、检查和 owner 决定。
Codex 理解需求、补充或追问、拆分当前阶段工作、选择工具和插件、执行与解释结果。
一个资产任务保持一个执行者；这些子任务不会触发普通代码 `task_plan` 的独立工作副本展开。

- `beaver_task`：任务工具，查询状态/回执、报告、阶段计划、子任务与候选提交。
- `beaver_workflow`：任务工具，通用状态、长操作、取消、插件准备、检查、导出和模型评价。
- `task.framework`：桌面 / HTTP / 业务 MCP 的 owner 入口，参数为 `{id, request}`；共享同一核心业务。
- `assetTask.deliveryDecide`：沿用 owner 阶段决定入口，仍需 candidate、revision 和请求 ID。

动态工具绑定当前 task / thread / turn；旧执行不能变更新执行的状态。
owner API 拥有本机业务权限；其凭据不注入受管理 Codex 提示词。
`configure` 只提供给 owner，模型不能更改适配器或削弱规则；`judge` 根据调用来源保存 model 或 owner。
任务暂停等待审批时，允许对应执行读取状态、回执和已有长操作；不会解锁新的制作。
回调协议升级时建立新线程并保存旧身份，避免复用缺少新工具的注册。

## 长操作协议

`request.operation` 支持 `state`、`inspect`、`cancel`、`start`、`configure`、`judge`。
`state` 返回 configuration、operations、traces、checks、judgments、observations、recovery 和 coverage。
`inspect` / `cancel` 使用 `operationId`；`start` 使用稳定 `requestId` 及以下 job：

| kind         | 额外字段       | 作用                                               |
| ------------ | -------------- | -------------------------------------------------- |
| callback     | request        | 异步文件回调；只允许绑定执行身份的调用             |
| plugin       | plugin, action | 对已登记插件执行 probe / install / enable / reload |
| check        | candidateId    | 对冻结候选和上游执行版本化检查                     |
| inputExport  | attemptId      | 单独导出该次执行的历史输入字节                     |
| dependencies | command, paths | 扫描已保存 `.blend` 的文件引用                     |

每个任务最多一个未结束 operation。启动及请求去重持久化；同一来源、执行身份、请求 ID 和内容
的重试返回原 operation，冲突内容或新 turn 复用请求 ID 被拒绝。恢复后先 inspect，再决定新操作。
终态为 `succeeded`、`failed`、`cancelled`、`interrupted`、`stale`。`succeeded` 表示 job 执行完成；
插件仍需读取 `ready`，检查仍需读取 `passed`。其他 job 的内容在 `result.value`，callback 返回原回执。
成功 callback 的 `paused: true` 同时提升到 operation 视图，执行器立即结束 turn 并等待 owner。

文件捕获、哈希、检查和适配器进程在数据库锁外执行。提交重新核对 task 身份/状态、asset revision、
Blender session 和配置；旧结果保留为 stale，不能发布为有效检查。检查失效与 operation 创建原子提交，
检查报告与 operation 终态原子提交。取消、turn 结束、宿主关闭会停止拥有的进程；重启把未决工作记为
interrupted，不重放副作用。若 callback 已提交而外层 operation 回执未写完，启动恢复复用已提交回执。

## 适配器与插件约定

owner 使用 `configure` 传入完整 configuration：当前 `revision`、`plugins`、`rules`、`semanticRequired`。
Beaver 以 revision 作 CAS，成功后递增。首次 revision 为 0；每类最多 32 个条目。配置变化使旧检查和
视觉评价失效，必须重新检查。运行中 owner 插件操作要求先中断执行器，受管理 Codex 可在自己的 turn 使用插件。

每个插件字段为 `id`、`host`（godot / blender / codex）、精确 `version`、精确 `hostVersion`、
必需 `probe` 和可选 `install` / `enable` / `reload`。缺少某 action 的适配器会明确失败。
每条规则为 `id`、正整数 `version` 和 `checker`。

每个命令字段为：

- `executable`：本机绝对路径；Beaver 直接执行，不拼接 shell 命令。
- `sha256`：真实可执行文件的 64 位小写 SHA-256。
- `args`：最多 64 项，每项最多 4000 字节；不得放凭据。
- `timeoutSeconds`：1..600。
- `files`：至多 32 个脚本/资源的绝对路径到 SHA-256 映射，可省略为空。

启动前和返回后核验可执行文件及显式资源哈希；单文件不超过 512 MiB，拒绝链接和非普通文件。
解释器模式必须同时固定脚本及其实际依赖资源。框架不会自动解析所有传递依赖。
安装来源、下载版本、校验和、授权要求由具体适配器定义并回报；本轮没有选择或下载任何第三方插件。

进程工作目录为任务工作副本，检查器则为冻结候选导出目录。`BEAVER_CONTEXT` 指向临时 UTF-8 JSON 文件：
通用 context 包含 protocolVersion 1、taskId、workspace、dataRoot、threadId、turnId、sessionId、
checkpoint、attemptId 和 configurationRevision。插件额外收到 plugin 与 action。
适配器退出码必须为 0，并输出一行 `BEAVER_RESULT=<JSON object>`，JSON 最大 64 KiB。
Beaver 保存结构化报告、命令和资源摘要；其余进程输出只保留形状/摘要，不作为明文日志保存。
结构化报告同样不能包含凭据；应报告精确故障类型、版本和恢复建议。

插件 probe 必须返回 `pluginId`、`host`、`pluginVersion`、`hostVersion`，以及布尔值
`installed`、`compatible`、`enabled`、`callable`、`restartRequired`。
install / enable / reload 后会独立运行 probe；只有身份、精确版本匹配、前四项为 true 且
restartRequired 为 false 才标记 ready。`callable` 应通过该插件最小真实能力调用验证。
网络、登录、付费、兼容性或重载阻塞需要保留，不可用安装成功替代。
适配器应限于任务工作副本和上下文，不修改全局安装；这是可信适配器契约，当前无操作系统沙箱强制隔离。

## 检查、评价与最终门槛

检查器 context 包含 candidate、configuration、snapshot、workspace、attemptInputs。
snapshot 按上游到当前候选叠加；每个 attemptInputs 项带 attemptId、manifest 和独立 path，
同名输入在不同尝试中有不同字节时分别保留，不能合并成一个目录掩盖历史版本。
检查器返回 `ruleId`、`ruleVersion`、`verdict`（pass / fail），以及具体问题和覆盖项。
标识或版本不符、进程失败、输出无效均使规则失败。检查后重新核验全部导出输入；检查器修改输入会失败。
报告绑定候选摘要、配置摘要/revision、runnerVersion 和时间，过期结果保留但不解锁审批。

`judge` 传入 candidateId、configurationRevision、verdict、note 和 1..16 个冻结候选 PNG paths。
Beaver 解码验证图片并保存图像哈希，不使用未保存的实时帧。owner 与 model 的评价分别保存且保留历史；
开启 semanticRequired 后只有当前 owner pass 可满足视觉门槛，Codex 的 pass 仅作参考。
阶段批准仍须独立 owner 决定，技术失败不能被人工接受、autoAccept 或普通 continue 绕过。

最终合入复核全部已批准输出、规则和 owner 评价、检查器资源固定值，以及候选引用的 attempt 输入。
后续已批准候选明确覆盖同路径依赖时接受新版本；未审批漂移被拒绝。历史冻结输入始终可验证和导出。
返工保持候选历史并失效受影响批准；当前场景中的修改不能凭历史通过报告直接合入。

## 证据与恢复界面

`framework_evidence` 从 Codex 真实事件保存工具轨迹，独立于有界 calls 摘要表。
started 事件固定 attempt、thread / turn、session、checkpoint；迟到 completed 不归入新的 attempt。
只有 completed 时保留空 attempt 和缺失开始事件说明；未完成轨迹在恢复时记录 interrupted。
工具输入输出保存安全形状和摘要，完整模型声明沿用回调报告；不记录私有推理或虚构 Skill 执行。
提供 Skill、模型声明使用 Skill、观察到工具调用和适配器版本回执分别表示不同事实。

资产窗口的框架面板展示配置、插件状态、operation/取消、检查、固定候选评价、输入导出、依赖扫描、
轨迹和恢复历史；原交付面板继续处理 owner 决定。实时场景观察与冻结交付文件独立。
任务切换、卸载或晚到轮询结果不能覆盖新任务状态，操作期间避免旧轮询覆盖更新。
当前新增面板已通过 TypeScript 检查及相关组件/业务测试，未在本轮进行新原生窗口交互验收。

## Skill 接入顺序

1. 读取 beaver_task 与 beaver_workflow state，确认当前阶段、待解决反馈、配置及有效输入。
2. 补充低成本细节；重要歧义使用既有提问机制，不默默改动已确认设计。
3. 在当前阶段登记有恢复价值的子任务，声明目标与 acceptance；begin 显式输入清单。
4. 比较已配置能力，优先合适且实际 ready 的插件；缺少适配器时报告需求，由 owner 配置。
5. 执行、检查场景和保存文件；报告 Skill 引用/版本、工具、插件及观察限制；finish 当前 attempt。
6. 提交候选后停止制作；运行所需检查和 owner 评价，等待 owner 决定后由 Beaver 恢复。
7. 失败/取消/重启后先核对场景与回执，再以新 attempt 和 recoveryNote 重试，禁止盲目重放相对修改。

运行时会注入 [协议指令](../resources/instructions/task-callback.md)。具体 NPR 小步骤的目标、
Skill 文本、插件选择、工具顺序和质量标准在后续单独开发，本轮不会写死人物模板。

## 已知边界

固定哈希验证不提供文件系统原子隔离，存在外部写入的 TOCTOU 窗口；可信进程仍有当前用户权限。
取消不能撤销已发生的插件/场景副作用，单次有界文件 IO 可能延迟响应取消。
Blender 扫描使用 factory startup 和禁用 autoexec，覆盖 `bpy.utils.blend_paths` 的已保存外部引用；
packed 资源、运行时生成依赖及自定义插件引用不能据此宣称完整。
扫描至多 32 个源文件、256 个返回路径和 512 MiB 依赖；缺失、外部、链接或不可捕获文件记入 unresolved。
扫描结果不会自动代填或放宽 begin 的 32 项输入清单；较大资产需显式拆分或后续调整容量。
operation 查询当前扫描持久记录；大量历史数据的分页和性能优化留待实测需要。
本轮没有运行新的 Beaver.exe、真实第三方插件或完整 NPR 流程，历史原生证据不能替代新增能力的产品验收。
