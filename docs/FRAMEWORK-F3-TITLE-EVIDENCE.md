# F3.3 Codex 任务标题建议

2026-09-25，源码开发切片完成；未生成或发布新的原生程序。

## 可用流程

生产任务草稿编辑器提供“Codex 建议标题”。请求只发送目标任务的说明与验收要求，返回候选后由用户明确选择“采用标题”或“放弃建议”。采用只修改目标标题，保留其他任务字段和请求期间发生的无关编辑；随后沿用草稿保存流程。空标题与已有标题均可请求建议。

生成期间或采用之前，目标任务被删除，或其标题、说明、验收要求发生变化时，候选不能覆盖当前草稿。生成失败、返回格式错误及关闭界面后的迟到结果均不修改草稿。已锁定、忙碌或未就绪的工作区不能采用候选。

## 实现边界

- `objectTask.suggestTitle` 经 Desktop 目录及异步路由接入，先校验严格请求与已注册项目，再使用宿主 code 能力配置启动 Codex。
- Core 复用规划执行的隔离启动配置：临时 HOME、空工作目录、只读沙箱，禁用 shell、MCP、网络搜索与子代理等入口；仅开放 `beaver_suggest_task_title` 动态工具。
- 协议核对 thread/turn 身份、工具名、命名空间及参数结构。候选最长 48 个 Unicode 字符，禁止控制字符；普通文本不能替代结构化结果。
- 服务最多同时持有 2 个请求，每个请求限时 60 秒。调用者离开不会丢失进程 owner；超时、协议失败和宿主退出均执行关闭并等待，退出停止接受新请求。提供方错误经过凭据脱敏。
- API 返回候选，不写项目草稿。采用与保存仍由现有工作区控制器负责，不增加后台自动保存或任务执行行为。

## 定向验证

本轮以下检查均成功，日志保留在 `output/f3-title-*.log`：

- `cargo test --locked -p beaver-core object_task_title`：4 项通过，覆盖结构化协议纠错、错误身份与非法工具、空结果与提供方失败、超时、进程/临时目录清理、调用者离开、并发上限及退出等待。
- `cargo test --locked -p beaver-core object_task_planning_executor`：4 项通过，保护共享启动提取所相邻的规划执行协议。
- `cargo test --locked -p beaver-desktop --bin Beaver object_task_title_runtime`：1 项通过，覆盖请求契约、无效/旧项目在启动前拒绝及只读目录提示；同时完成 Desktop 测试目标编译。
- 前端 title、insertion、workspace、draft-lock、cancellation-ui 五个测试文件：21 项通过。其中标题 5 项覆盖生产入口、显式采用、保留无关编辑、保存重开、过期候选、删除/放弃后的迟到响应和失败保留草稿。
- `npm run typecheck`、本次修改文件的 Prettier 与 Rustfmt 检查、`git diff --check` 均通过。
- `npm run check:effective-lines`：1054 sources、17 unchanged legacy、0 violations；工作区控制器 492 effective lines，未提高历史基线。
- 本次 19 个源码文件均为有效 UTF-8，无 BOM。

## 验收范围与剩余项

协议测试使用 fake app-server 子进程，前端保存重开使用 fake API。它们证明请求、采用、错误恢复与进程所有权的定向行为；本轮未调用真实模型提供方，未执行原生窗口交互或完整游戏创建验收。共享启动配置已复用并检查，但上述 fake 子进程测试不构成真实 Codex 配置兼容性证明。

F3.3 的 Codex 标题建议实现已接通；真实提供方及正式原生交互验收仍待执行，计划项保持未勾选。后续正式验收应按 `scripts/start-fresh-native-test.ps1` 创建全新测试轮次，不复用此前项目成果。
