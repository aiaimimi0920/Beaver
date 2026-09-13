# 游戏验收功能实现与验证

设计日期、实际开发开始日期及功能实现日期：2026-09-13，Asia/Shanghai。

需求原文及讨论保留在[开发设计](GAME-VALIDATION-DESIGN.md)。用户授权在
`C:\Users\Public\nas_home\beaver1` 的 `feature/game-validation` 分支独立开发，
功能完成后合入原 Beaver 主分支。本文分别记录实现、验证及集成，避免用某一项通过代替其他结果。

功能实现、核心回归、真实引擎适配器验证及原生构建已完成；完整桌面/API 操作验收尚未执行，见末节。

## 实现与使用

打开游戏项目，在 Beaver 导航中选择“测试与画面”。页面包含 GUT 结果、漫游、功能、
UI、任务四类流程、运行历史、截图/视频、认可基准、任务覆盖和正式发布检查。
任务详情中的入口可以定位关联流程；执行阶段、判断结果及候选版本分别展示。

1. 开发任务收到随 Beaver 分发的[游戏验收约定](../resources/instructions/game-validation.md)，
   维护有行为断言的 GUT 用例及 `beaver.validation.json`。已有项目也可以在 Tab 中新增或编辑流程。
2. “运行代码验收”检查当前冻结副本；“运行流程”执行选中流程；“重跑全部流程”启动日常代码和画面检查。
   这些日常操作不产生正式发布凭据。
3. 同一路线可重复运行。漫游流程另有“生成新随机路线”，新探索保留为新流程，原路线及认可基准继续保留。
4. 选择历史运行查看截图或播放 WebM；查看采集点、步骤状态、来源脚本、场景和资源。
   代码入口同时提供当时快照及当前文本，避免把历史截图指到后来移动的行号。
5. 可以框选截图区域，或者选择视频时间区间后输入修改意见。反馈可创建新任务、子任务、
   后续任务或只读 AI 检查任务。修复任务完成后重新执行反馈对应的原始流程，保留修复前后关联。
6. 用户逐项确认完整证据后才更新认可基准。AI 判断通过不推进用户基准，浏览图片也不产生认可。

流程编辑器支持结构化基本字段和 JSON 动作配置。首版操作包括 `wait`、`action`、`key`、
`click`、`waitFor`、`capture`；按键和点击走真实 Godot 输入，等待条件读取实际节点状态。
入口场景、初始存档、种子、语言、尺寸、帧率、动作、采集点及对比参数均随流程版本保存。
视频录制整个流程，并保存事件和定期关键帧。

## 任务、代码与发布规则

新任务默认自动审批，同时保留显式手动策略。涉及游戏运行内容的任务，先对预计合入的候选执行
GUT，再校验候选未变化并安全合入。父任务在全部子任务已完成并审批后检查集成结果。
并发编辑、文件冲突、待回答问题和恢复状态仍受原有约束。旧任务不被静默改成新审批策略。

GUT 失败项显示用例、断言和日志，自动修复最多尝试 2 次；相同失败指纹再次出现或没有可用引擎时停止。
验证期间的新用户指令保留到下一轮修改，修改后重测候选。没有真实断言、报告缺失、必需测试被跳过、
目录覆盖不完整等情况不能标绿。文档等无运行内容变动的任务不要求编写无意义的测试。

普通任务交付后登记持久化画面待办，后台准备和运行流程。缺少可运行流程会显示原因。
排队、画面红项、缺少基准或用户未查看均不会撤销任务完成。正式发布则会检查缺失的任务画面覆盖。

导出对话框区分“内部开发 / 调试验证”和“游戏正式发布”，默认仍选择内部用途：

- 内部导出沿用既有导出验证，记录 `internal`，不要求 GUT 与画面全部通过。
- 正式导出记录 `formal`，冻结候选、导出预设、代码范围、流程版本及策略，执行本次新检查。
- “正式发布前强制运行一次画面诊断”按项目保存，默认开启。开启时全部必需代码及画面项目通过才放行。
- 关闭时只取消强制画面运行和画面绿灯限制；正式候选的代码检查仍必需。已有红项保持原判断。
- 用户确认只作用于具体候选的完整画面，不能批准失败代码或不完整证据。UI、业务 API 和 MCP 使用同一后端规则。
- 已检查的固定候选可继续导出；切换到修改后的项目需要新候选。导出记录包含检查范围及证据摘要，
  导入的验收声明保留为来源信息，不转成当前项目的本地放行凭据。

## 运行依赖与比较边界

Beaver 内嵌固定 GUT 9.4.0 归档，在临时副本中校验 SHA-256 后解压，包含上游许可证；
来源和摘要见[运行依赖说明](../resources/validation/README.md)。面向 Godot 4.4 及之后的 4.x，
代码验收无界面运行，画面流程使用真实渲染。运行各自拥有输出目录和隔离的 `user://`。
取消及超时只清理本次持有的进程；应用重启将未完成运行记为中断。

录制先由 Godot Movie Maker 生成真实帧，再由 FFmpeg 编码为 WebM。FFmpeg 从项目设置的路径或
`PATH` 解析；未打包个人 Godot/FFmpeg 环境，也不自动安装。缺少工具、截断、采集点缺失和编码失败
会保留原因及已经取得的证据。

自动比较方法标识为 `pixel-tiles-and-event-keyframes-v1`：对齐稳定采集点和事件关键帧，
比较整图与局部分块、场景标识及时间容差。默认阈值 0.98，可在 0.8 至 1.0 之间设置；
允许变化区域显式配置且不能遮住一半或更多画面。流程和配置签名不兼容时要求新基准。
此方法用于提示变化，不能证明玩法语义正确；需要语义判断时创建带冻结证据的 AI 检查任务或由用户查看。

PNG 与视频都保存作者指定及运行时取得的代码/场景/资源引用；来源分别标识为 `author`、`runtime`、`ai`。
视频另有时间范围、动作及对应关键帧。历史媒体具有内容摘要，运行快照、基准及反馈记录不会被重跑覆盖。

## 模块与接口

- [native/core/src/validation](../native/core/src/validation/)：流程、运行隔离、GUT、采集、比较、
  基准、反馈、任务代码关卡、异步覆盖及正式发布校验；通过原生存储和快照机制持久化。
- [validation_runtime.rs](../native/desktop/src/validation_runtime.rs)：异步队列、工具定位及共享业务调度。
- [validation_catalog.rs](../native/desktop/src/validation_catalog.rs)：UI/API/MCP 共用操作目录。
- [src/ui/validation](../src/ui/validation/)：Tab、流程列表及编辑器、媒体、代码上下文、反馈和发布状态。
- [ExportGameDialog.tsx](../src/ui/ExportGameDialog.tsx)：内部/正式用途、候选创建、状态和导出入口。
- [resources/validation](../resources/validation/)：固定 GUT 依赖和 Godot 输入/采集脚本。

业务操作包括 `validation.list`、`validation.flow.list/save/explore/run`、`validation.code.run`、
`validation.run.all/rerun/get/cancel`、`validation.source`、`validation.evidence.confirm`、
`validation.feedback.create`、`validation.settings.save`、`validation.release.start/get`。
具体必需参数以操作目录为准。写操作带 `requestId`，编辑和确认还校验预期版本。
人工确认入口只接受 UI 的真实操作来源；API/MCP 不得把 AI 调用登记为人工认可。

## 已有验证证据

所有下列命令从独立目录执行，未使用原 Beaver 的运行数据。日志和媒体保存在该目录的 `output/`，
这些运行产物不提交到源代码历史。

| 检查                                     | 结果与证据                                                                                    |
| ---------------------------------------- | --------------------------------------------------------------------------------------------- |
| `cargo test --locked -p beaver-core`     | 130 项库测试和 4 项集成测试通过；`output/validation-core-tests-final.log`                     |
| `cargo check --locked -p beaver-desktop` | 通过；`output/validation-desktop-check-final.log`                                             |
| `npm run typecheck`                      | 通过；`output/validation-typecheck.log`                                                       |
| `npm run build:native:ui`                | 通过；`output/validation-ui-build.log`                                                        |
| `npm run format:check`                   | 通过；`output/validation-format-check.log`                                                    |
| `npm run check:effective-lines`          | 404 个源文件，18 个未修改历史超限文件，0 项违规；`output/validation-effective-lines.log`      |
| 真实 Godot 适配器合同                    | 通过；`output/validation-adapter-sources.log` 及 `output/validation-adapter-20260913-135950/` |
| `npm run build:native`                   | 通过，退出码 0；`output/validation-native-build.log`，生成 `target/release/Beaver.exe`        |
| 集成分支 `npm run build:native`          | 通过，退出码 0；`output/validation-integration-build.log`，包含结构检查、UI 打包及原生编译    |

最新适配器合同使用引擎
`Z:\project\godot-4-4-1\bin\godot.windows.editor.x86_64.exe`，其实际版本输出为
`4.5.2.rc.custom_build.2890667c8`，不能根据目录名把本次结果记成 Godot 4.4.1 验证。
真实 GUT 用例先以 1 个断言通过，再故意引入错误并得到 1 个失败断言及退出码 1。
真实点击与移动产生不同 PNG；10 个截图采集点和 1 段 WebM 均带有 `res://main.gd` 的运行时引用。
FFprobe 确认视频为 VP9、320 x 180、30 FPS、49 帧、1.641 秒、6722 字节。
隔离存档路径位于本轮运行目录内。

当前集成构建产物为 15,823,872 字节，生成时间为 2026-09-13 14:29:52，Asia/Shanghai。
SHA-256 为 `36fbfdb31386bf2b84e23c12480d262864738c88b1f0428d4df251c6cddb53fa`。

该合同位于 [validation_contract.rs](../native/core/examples/validation_contract.rs)，使用专用 fixture，
直接验证引擎适配器。它不构成经 Beaver API 创建游戏的原生应用验收。
完整核心测试之后补充的视频代码引用修复，已由最新合同重新编译和真实运行验证。

任务回归覆盖代码通过但画面未完成、显式手动审批、验证中追加指令、并发项目变化、冲突保护、
父任务集成检查及自动修复停止条件。基准/发布回归覆盖不完整证据、来源、旧候选、开关两种模式及
导出声明保留。上述通过结果不代替真实桌面操作和目标平台导出运行验证。

## 集成及剩余验证

开发副本最初为独立快照仓库，和原 `main` 的根历史不同。实现提交为
`f3f34bc531da948b44f68c7876fd7e89665d52f4`，保留在 `feature/game-validation`。
在同一独立目录创建 `integration/game-validation-main`，基于原 `main` 的
`393e2aa1c345312d1f6df85c9d1abe71c59d8f81` 承接为
`98fbf2ed82d9682ebc6a241d0dec5732dd9accfc`，避免合并无关根历史。

承接后的功能提交与原实现提交只在原主分支已更新的两个人物 skill 文件上存在差异，功能内容完全保留。
集成构建通过；主分支合入使用快进方式。合入前后提交、工作区状态及并发 skill 文件摘要记录在
`C:\Users\Public\nas_home\beaver1\output\validation-main-merge-proof.json`，运行证据不进入源代码历史。

原生验收启动前的保护检查发现另一条开发任务正在运行
`C:\Users\Public\nas_home\beaver2\target\release\Beaver.exe`，因此停止启动，未关闭该实例。
原生验收脚本会关闭旧 Beaver 实例，本轮尚未启动编译后的 Beaver 做完整桌面/API 验收。
后续应在可独占测试的时段遵守
[全新测试流程](FRESH-TEST-WORKFLOW.md)，通过 Beaver API 从新空白项目完成创建、失败修复、
截图/视频反馈、人工认可、正式开关两种模式、重启恢复及备份恢复的实际闭环。
不得使用本轮 adapter fixture 冒充那一轮游戏内容，也不得关闭正在使用的无关 Godot 或 Blender。

首版截图框选在松开指针后显示选区；流程编辑使用 JSON 动作配置，尚无图形化动作录制器。
这些交互边界不改变四类流程、录像、代码上下文、反馈和发布规则，但需要在桌面验收时实际查看。
