# R14：NPR 功能包与调用日志

记录日期：2026-09-10。本轮完成了接入代码、调用日志和可运行安装包；标准角色用例尚未完成全流程验收。当前指定引擎缺少 NPR 所需接口，已保留失败证据，并询问是否切换到找到的配套引擎。尚未收到该选择，未提交角色制作任务。

## 已实现

Beaver 继续作为用户操作项目、任务、素材和验收的应用。Codex 可以在所有阶段参与决策，选择 Beaver 提供的标准工作流、读取规范、制作内容并修复错误。此次没有增加 AI 调度器，也没有实现此前仅用于举例的绿色贴图或 A 模块。

原生新建项目界面增加 NPR 模块开关和定制 Godot 路径。已有项目的“游戏 → 功能包”页面可安装、查看失败原因和重试。RoleNPR 插件、AI 使用说明、资产规格及来源哈希已嵌入 EXE；安装后保存项目级引擎绑定，供任务、试玩与工作流使用。

标准人物工作流提供 `inspect`、`validate`、`preview`，分别用于读取实际规格、检查角色定义与初始化、生成正侧背三视图。Codex 的任务内 MCP 可以发现和调用这些工作流，并按照用户要求使用 Blender MCP 制作角色。实际角色、素材与场景继续进入原有任务合并和审批流程。

调用日志写入现有 SQLite 数据库，覆盖进入业务分发层的界面、HTTP API、业务 MCP 调用，以及 Codex 回合和相关工具调用。记录来源、方法、项目/任务关联、时间、状态、耗时、参数和结果摘要及脱敏错误。`logs.query` 支持分页和项目/任务/方法过滤，功能包页面也可以读取日志。原始参数内容和生成素材不复制到调用表；详细任务叙述仍保存在原有任务事件中。

公开业务操作共 51 个，新增 `project.npr.install`、`workflow.list`、`workflow.run`、`logs.query`。任务内媒体 MCP 共 6 个工具，其中 2 个是工作流工具。接口与边界详见 [BUSINESS-API.md](BUSINESS-API.md) 和 [NPR-INTEGRATION.md](NPR-INTEGRATION.md)。

## 已执行的验证

- TypeScript 类型检查通过；此次初始完整 TypeScript 测试 91 项通过。
- 此次初始完整 Rust core 测试 95 项通过；后续日志的 2 项针对性测试和 Codex 隔离配置的 1 项测试通过。
- 最新 Rust desktop 编译检查通过；R14 release 编译成功，用时 4 分 01 秒。
- 改动范围的 Prettier、Cargo fmt、两个工作流脚本的 gdformat 检查通过；检查的 30 个实现及文档文件均为有效 UTF-8，无 BOM。
- RoleNPR 共 98 个文件、380,504 字节，逐个核对上游、仓库副本、来源哈希与 EXE 嵌入输入，差异为 0；另有 1 个嵌入的来源清单。
- 打包后真实 HTTP/业务 MCP 验证 13 项通过，包括 51 个操作的发现、共享状态、文档写入、冲突保护、未安装工作流拒绝，以及 API/MCP 失败和成功日志的项目关联。
- 打包后媒体/工作流 MCP 验证 16 项通过，包括 6 个工具的发现、空工程工作流状态及既有媒体接口回归。这些是传输层验证，未调用真实模型。
- 真实 R14 窗口已检查 NPR 失败提示、保留的引擎路径、安装重试按钮和日志读取入口。

证据文件：

- [Rust 初始测试日志](../output/validation/npr-core-initial.log)
- [TypeScript 初始测试日志](../output/validation/npr-ts-initial.log)
- [最终编译与新工程证明输出](../output/validation/npr-r14-build.log)
- [HTTP/业务 MCP 验证](../output/validation/native-business-1789016737166/proof.json)
- [媒体/工作流 MCP 验证](../output/validation/native-media-1789016747564/proof.json)
- [打包后 NPR 失败及调用日志证据](../output/validation/npr-r14-packaged-proof.json)
- [R14 窗口：失败说明及日志](../output/playwright/npr/r14-failure-and-logs.png)

## 安装包与当前工程

安装包：`release/Beaver-native-0.1.19-preview-r14-win32-x64/Beaver.exe`。

EXE 为 13,328,384 字节，SHA256：

```text
F0A52FFBA0E95B89D2DF6A760824D6D3FD19C741AEE01F1E435FD2B76295DC87
```

安装包共 606 个文件、16,940,629 字节，包含许可证清单。Godot 和 Blender 不包含在安装包内。旧 R13 安装包及所有旧测试工程均保留。

每次编译前通过标准启动脚本关闭旧 Beaver；关闭前检查当前 API 中没有活动任务。传输测试结束也确认没有遗留 Beaver 进程。最后使用打包后的 R14 EXE，经 `start-fresh-native-test.ps1` 创建了新一轮：

```text
output/validation/fresh-round-20260910-050701-da09248d0e4f4ae999bcac57af8ce548
projectId: 63c5daae-031e-485e-91cb-5d20e0a4eab0
API port: 4326
```

启动证明记录旧项目 0 个、旧任务 0 个。NPR 安装前仅有 `project.godot`、`main.tscn`、`export_presets.cfg` 三个空白模板文件。最终复核只有该 R14 EXE 的桌面实例运行，PID 为 49456。测试令牌未写入证明文件。

## 角色用例的实际状态

标准输入保持为：`二次元动漫女性角色，带草帽`。

先通过 Beaver API 在上述新工程安装 NPR。用户指定的 `C:\Users\Public\nas_home\godot\export` 实际为 `4.8.dev.custom_build.69903a894`，安装探测失败，缺少：

- `TriangleMesh.update_from_indexed_surfaces`
- `RenderingServer.mesh_surface_get_geometry_arrays`

R14 将项目保存为 `npr.status = failed`，工作流显示未启用；失败调用持久化为 `source = api`、`status = failed`，关联正确项目，耗时 964 ms。项目未丢失，重试入口可用。没有直接从外部调用 Godot、Blender 或 Codex 来替 Beaver 完成角色，也没有手工补写游戏内容。

RoleNPR 的交付文档使用配套构建 `db1af1e99`，现有归档位于：

```text
C:\Users\Public\rolenpr_delivery\engine-ssr-boundary-db1af1e
```

已询问用户是否改用这份引擎，并保留原 `godot\export` 目录。尚未切换。后续确认后可在当前轮次通过 `project.npr.install` 重试，再通过 `task.create` 提交上述短输入。实际 Blender MCP 调用、角色文件产出、NPR 校验、三视图与任务审批仍需在该轮记录证据。

## 仍需验证或完善的边界

配套引擎上的安装与渲染脚本尚无本轮成功运行证据，不能据此宣称人物全流程已跑通。Blender 专用会话的启动目前由任务内 Codex 按 skill 检查和准备，尚无独立的程序化隔离启动器。定制导出模板的自动选择也不在此次角色接入中。

公共工作流调用目前持续到完成或应用退出，尚无单独的工作流取消接口。项目清单记录安装状态，未持续验证用户可编辑文件的完整性。三视图和初始化通过仍需配合完整美术验收。
