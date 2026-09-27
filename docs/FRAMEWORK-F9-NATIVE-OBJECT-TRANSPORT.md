# F9：原生对象创建的 HTTP 与 MCP 接入验证

日期：2026-09-26。状态：本次接入验证完成；F8、F9 总体仍未完成。

## 结果与范围

新增 [原生传输验证脚本](../scripts/native-object-transport-smoke.ts)，在新编译的 Beaver.exe 上运行生产 ObjectGenerationSession，通过真实 HTTP API 保存草稿并提交对象，再经该 EXE 的 stdio MCP 查询和重放同一提交。数据库、Core 和传输均使用实际实现，没有模拟服务。可观察完成条件是：新空白项目中只产生一个对象、两个 planned 任务和一个 planned 轮次，重复提交返回同一回执且不改变快照，本地会话重建后恢复该回执。

实际结果满足以上条件。HTTP 和 MCP 查询快照一致；不存在的项目在 HTTP 返回 409，MCP 返回相同错误信封。真实 HTTP 拒绝缺少凭据以及携带 Origin 的请求，状态码均为 401。MCP 协商协议版本 2025-11-25，并发现 commit、publicationPreview、deferCandidateFeedback、publishCandidate 命令；发现命令不代表已执行其发布流程。

本轮仅创建任务计划，没有调度制作、调用模型、运行 Godot/Blender 或发布对象。会话恢复使用脚本内存存储中的持久化字符串重建生产会话；不覆盖浏览器 localStorage 或关闭原生进程后的恢复。本轮没有执行 Tauri UI 调用，也不构成全流程或外部发布验收。

## 原生证据

- 使用 `scripts/start-fresh-native-test.ps1 -Executable target/release/Beaver.exe -Port 4327 -Build` 完成开发版构建，release 编译耗时 10 分 46 秒。见 [构建日志](../output/f9-native-build.log)。保留已有的 unused fields、unused imports 和 unused_mut 警告，本批未扩大修改范围。
- 最终成功轮次由启动脚本创建独立数据目录、WebView 目录和 blank 项目；开始时项目数、任务数均为 0。见 [启动证明](../output/validation/fresh-round-20260926-150505-abd645ac6aaa491cbfea431c05a63c77/start-proof.json)。
- [传输证明](../output/validation/fresh-round-20260926-150505-abd645ac6aaa491cbfea431c05a63c77/object-transport-proof.json) 保存 5 组通过条件、调用方法、回执和快照；[运行日志](../output/f9-native-transport.log) 对应脚本退出码 0。
- 实际 EXE：[target/release/Beaver.exe](../target/release/Beaver.exe)。SHA-256：`8ED90A38823BF3A73E096F0CB1E20CDE7D0FF26375B294F39C16158BBFCAEA40`。运行后重新计算与证明一致，未变更版本元数据或执行安装发布。
- 脚本退出时关闭自身 MCP 子进程，并按启动证明的 PID 与 executable 匹配关闭本轮宿主；最终检查 Beaver 进程数为 0。认证令牌只通过环境变量传入，没有写入证明。

首次探测的自定义草稿 ID 不符合生产恢复格式 `generation-*`，导致恢复断言失败。修正验证脚本 ID 并增加恢复错误断言后，在全新项目重跑通过，未放宽生产校验。中间一次启动后，外层命令误用 `$LASTEXITCODE` 判断 PowerShell 脚本结果；改用 `$?` 后重新创建轮次。旧轮次保留，没有复用其生成数据。重跑复用同一已编译 EXE，没有重复编译。

## 定向检查

- `npm run typecheck`：退出码 0，见 [日志](../output/f9-native-typecheck.log)。
- `npx prettier --check scripts/native-object-transport-smoke.ts`：通过，见 [日志](../output/f9-native-format.log)。
- `npm run check:effective-lines`：1179 个源文件、17 个未改动历史文件、0 项违规，见 [日志](../output/f9-native-lines.log)。
- `git diff --check`：通过，见 [日志](../output/f9-native-diff.log)。

## 剩余工作

F9.4 本批仅证明对象创建与幂等重放的 HTTP/MCP 共享规则，以及有限的认证、项目边界。Tauri 实际交互、模型工具权限、发布流程跨入口一致性和插件/预览能力总表仍需核对。F9.3 的旧数据取消恢复与迁移启用后重开证据、F9.1/F9.2 的其他生产路径和 F8.5 的旧拓扑重新定位映射仍未完成。原生开发版可用于真实数据操作，但不能据本轮局部证据声明全部框架完成。见 [实施计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)。
