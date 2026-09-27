# F5.3 尝试内只读回调与能力发现

2026-09-25：对象任务的生产 Codex app-server 执行链已注册 `beaver_object_attempt`，支持能力发现、冻结定义查询和冻结输入文件读取。F5.3 与整体开发目标仍未完成；本轮未生成或发布新原生程序。

## 可用流程

执行中的 Codex 可调用 `{"operation":"capabilities"}` 查看当前工具及不可用操作，调用 `{"operation":"context"}` 获取本次冻结定义和输入清单，再用 `{"operation":"inputFile","path":"hero.tscn","sha256":"清单中的哈希"}` 读取原始输入。宿主在同一个 thread/turn 内返回结果，模型继续文件加工；原有执行器关闭 writer 树后冻结输出并进入 `AwaitingGate`，不会启动后续阶段或完成对象交付。

此能力直接服务现有 Desktop 对象执行器启动的模型进程，无需新增界面入口。任务执行记录继续通过既有冻结文件查询查看最终结果。工作区中的输入文件允许被加工；回调读取的是不可变输入检查点，输出检查点记录加工后的工作区状态。

## 边界

- project/object/run/attempt 来自宿主持有的 lease；调用方不能指定其他目标，也不能取得 claim token。
- 仅接受已绑定且匹配的 thread/turn 和工具名。读取前后及回复前校验持久化所有权与中断状态，旧代次、已终止尝试和已请求中断的执行者不能取得成功回调。
- 参数拒绝未知字段和未知操作，序列化参数最多 8192 字节；路径最多 4096 字节，SHA-256 必须是 64 位十六进制。文件读取复用已有检查点路径、哈希和 blob 校验，正文上限为 1 MiB；二进制与超限沿用结构化状态。
- `managedTools`、`accept`、`publish`、`advance`、`outputCheckpoint` 明确列为不可用。没有接入旧资产 task callback，也没有增加自动重放、自动验收或越级完成路径。
- 已识别回调的参数或读取错误按动态工具失败结果返回；租约失效和中断终止对话。其他交互继续使用原有不可用处理。

## 定向验证

- `cargo test --locked -p beaver-core --lib object_attempt_`：42 项通过，1 项按原有条件忽略。覆盖新增的 2 项回调边界测试、1 项真实 RPC 子进程回调链，以及相邻执行生命周期、冻结文件、操作轨迹、取消和旧代次保护。忽略项需要 `BEAVER_TEST_GODOT_EXE` 的真实 Godot editor，本轮未运行。
- RPC 用例依次请求能力、上下文和冻结文件，在工作区同名输入已被改写后仍取得原内容，生成 `callback-result.txt`，确认冻结结果、完整 writer 树关闭、只启动一次 turn，以及后续任务仍为 `planned`。
- `cargo check --locked -p beaver-desktop --bin Beaver`：通过，确认实际 Desktop 消费者可编译。
- 修改源文件的 Rustfmt 检查通过；`npm run check:effective-lines` 检查 1066 个源文件，0 项违规，未修改基线；`git diff --check` 通过。

开发中先修正测试基线记录路径，再由负向测试发现 Serde 单元变体没有拒绝额外字段；改为带空字段的结构变体后，上述整批测试通过。未改变存储协议。

日志：`output/f5-callback-{attempts,format,lines,diff,desktop}.log`。早期失败记录保存在 `output/f5-callback-core.log`。

## 剩余范围

当前能力发现仅覆盖尝试内只读工具。受管 Godot/Blender 等工具调用、细粒度操作输入输出、阶段门槛与重做流程、真实模型提供方及正式原生交互验收仍待推进。确定性子进程证据不能替代真实提供方或发布验收。
