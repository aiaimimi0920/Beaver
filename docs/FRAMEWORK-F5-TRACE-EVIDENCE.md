# F5.3 操作通知轨迹

2026-09-25：完成 Core 持久化、Desktop 查询与生产执行记录界面的有界操作通知轨迹。本轮没有生成或发布新的原生程序，F5.3 及整体开发目标仍未完成。

## 用户可见结果

打开任务执行记录，在某次尝试下展开“操作轨迹”，点击“读取或刷新操作轨迹”。界面按接收顺序展示命令执行、文件修改、MCP 工具、动态工具、网页搜索的开始或结束通知，以及模型回合结束和模型服务错误状态。查询不重启执行、不调用模型、不读取活动工作区。

轨迹存入项目 SQLite 的 `object_attempt_trace`，通过 `objectTask.attemptTrace({projectId, runId, attemptId})` 查询。完成后的尝试以及重新打开项目存储后的历史查询使用同一份持久化证据。旧版本执行和没有收到支持通知的执行显示没有采集到轨迹，不从任务状态推造操作。

## 数据与生命周期边界

- 仅接收已绑定 thread/turn 的白名单通知；在同一写入事务内校验当前 lease、代次和对象任务所有权，过期执行者不能追加。
- 每次尝试最多保留最先收到的 256 条通知，达到上限后的下一条通知设置 `truncated`。界面明确提示后续通知未保存。
- 仅保存顺序、固定操作类别、阶段与固定状态；不保存任意 params、命令正文、模型文本、工具参数、输出或 stderr。
- 当前展示通知级元数据，不包含时间、原始 item ID 或单个工具的详细输入输出，也不将开始与结束通知推断成配对的操作。未知状态显示“未提供结果”。
- 操作通知结束不代表对象验收通过；没有结束通知时结果未知。实际尝试状态和输出检查点继续使用原有执行记录。
- 历史查询校验 project/run/attempt；前端校验响应身份、条数与连续顺序，刷新和关闭后忽略迟到响应。
- 持久化失败会让当前对话走原有失败收尾，保留已保存证据；不会静默报告完整轨迹。

## 定向验证

本轮通过以下检查，未扩大到全量原生验收：

- `cargo test --locked -p beaver-core --lib object_attempt_trace`：2 项通过。覆盖跨 thread/turn 拒绝、白名单、秘密文本排除、256 条上限、结束后保留、关闭 runtime 后重新打开存储的一致性、错误项目/run/attempt 和过期 writer。
- `cargo test --locked -p beaver-core early_completion_waits_for_entire_writer_tree_and_freezes_prompt`：1 项通过。真实子进程 RPC fixture 在 turn/start 响应前发送开始、结束与回合通知；三条通知经生产消费链按顺序持久化，并保留原有整个 writer 树关闭与冻结输出约束。
- `cargo test --locked -p beaver-desktop --bin Beaver object_attempt_runtime::tests`：3 项通过，包含 Desktop 编译、查询身份与空轨迹结果。
- `npx tsx --test tests/object-attempt-trace.test.ts tests/object-task-execution-files.test.ts`：8 项通过，覆盖展示、截断、空结果、错误、响应身份、乱序与关闭/刷新后的迟到响应，以及相邻生产执行界面。
- `npm run typecheck`、修改文件的 Prettier/Rustfmt 检查、`git diff --check` 通过。
- `npm run check:effective-lines`：1064 个源文件，0 项违规；没有更新历史基线。
- 17 个本轮修改的源文件均为有效 UTF-8，无 BOM。

重开存储用例第一次因旧 runtime 仍持有项目宿主锁而失败；调整测试按实际生命周期先释放旧 runtime 后，2 项 Core 测试重新通过。未修改项目锁实现。

日志位于 `output/f5-trace-{core,core-reopen,rpc,desktop,ui,typecheck,format-check,rustfmt-check,reopen-format,structure,diff-check}.log`；后续修改仅涉及测试重开步骤时，仅重跑受影响 Core 测试、该文件格式与结构检查。

## 后续范围

F5.3 的完整 Codex -> Beaver 工具回调、能力发现、细粒度操作输入输出与正式原生交互验收仍待推进。此轮使用确定性 RPC 子进程和组件渲染验证，没有运行真实提供方、真实窗口交互或正式发布验收。输入输出文件正文继续使用 [冻结文件正文查询](FRAMEWORK-F5-FROZEN-CONTENT-EVIDENCE.md)。
