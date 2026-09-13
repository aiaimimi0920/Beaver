# R15 NPR 人物制作验收记录

2026-09-10。按用户更正切换到本地模型服务后，本轮 NPR 角色交互与制作全流程通过：带选项的澄清、四个子任务、Blender MCP 建模、NPR 材质、展示场景、标准验证、合并与审批均已完成。此前错误来自旧服务地址。本次恢复沿用已打包的 R15，修改运行配置和验收文档，没有重新编译 EXE。

标准输入：`创建一个npr 3d 白发可爱动漫女孩模型，戴着草帽。`

## 新引擎与实现

通过 Beaver API 在全新空白工程安装 NPR 已成功。用户指定的 `C:\Users\Public\nas_home\godot\export` 当前返回 `4.8.dev.custom_build.db1af1e99`，项目状态为 `npr.status = ready`。此前缺少几何接口的问题已解决，没有改用其他引擎目录。

预检查工程：`output/validation/fresh-round-20260910-060945-01b22f771c1b4028a4242895a242834c`。对应 `start-proof.json` 与 `npr-install-proof.json` 保存了空白工程和安装结果。这一预检查使用已有 R14 EXE。

本轮将 Blender 专用会话的准备与回收固化到 Beaver：为 NPR 执行任务预留独立端口，使用空白场景和独立用户配置目录启动 GUI，加载已安装的 MCP addon，验证实际 ping，再启动任务内 Codex。父任务的规划阶段不启动 Blender。退出、暂停和中断后仅清理本次拥有的进程树，并记录启动和关闭调用日志。

已通过原生编译检查、端口预留/释放测试、隔离配置测试及 8 项调度契约检查。Python 引导脚本已做语法检查，并在打包后的真实 Beaver 任务中成功运行。

R15 release 编译成功，用时 3 分 32 秒。安装包共 606 个文件、17,005,653 字节，路径为 `release/Beaver-native-0.1.19-preview-r15-win32-x64/Beaver.exe`，EXE SHA256 为：

```text
6D1A2687C646AC8C04F9B1A6EA973BF0EF98577B6D54F33578B751198A144C8D
```

## 测试过程

打包后的 EXE 经标准脚本再次创建全新空白工程：

```text
output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d
projectId: 3860ea06-59c9-4114-9580-52e28f93f876
API port: 4326
```

安装 NPR 前仅有空白模板的 3 个初始文件，旧项目与旧任务均为 0。之前的工程、安装包和证据保留；编译与启动新 EXE 前均关闭旧 Beaver。最后运行的是 R15 安装包的桌面实例。

通过 Beaver API 导入本机 Codex 配置、检测工具、设置 30% 询问档位和子任务自动审批，然后提交标准输入。角色任务 ID 为 `b21b9ddb-db3b-46d3-a497-af948f8ca64b`，默认启用任务拆分。

1. 初次规划请求遇到 HTTP 503。Beaver 自动恢复一次，最终保留失败状态和请求错误。
2. 在同一轮通过 `task.continue` 输入“请继续。”，模型服务随后明确返回 HTTP 403：`This account only allows Codex official clients`。角色规划未生成，尚无角色子任务和资产产出。
3. 另建短输入“检查 NPR 和 Blender 制作环境是否就绪。”作为环境检查任务。Beaver 的 Blender 专用会话在 6,200 ms 内完成启动和实际 addon ping，就绪证据包含本次 PID 53724、端口 56679 和日志路径。该任务也被模型服务拒绝，随后专用 Blender 在 770 ms 内清理完成；启动与关闭日志均为 `succeeded`。
4. 另建环境取消测试，在专用会话准备期间调用 `task.interrupt`。接口在 1,304 ms 内完成，任务状态与准备日志均为 `interrupted`。取消前后均未发现遗留 Blender 进程。

上述动作均由 Beaver API 发起，测试操作者没有直接运行 Codex、Godot、Blender 或补写角色代码、模型和贴图。准备阶段的 addon ping 证明服务可响应，不代表已发生 AI 建模的 Blender MCP 工具调用。

用户要求“再试试”后，于北京时间 2026-09-10 20:06 和 20:07 在本轮原角色任务中分别通过 `task.continue` 输入“请继续。”和“请重试。”。两次模型调用分别持续 12,931 ms 和 10,801 ms，均返回 `405 Method Not Allowed`，响应正文为 nginx 的 HTML 错误页，请求地址为 `https://api.zzzcoding.org/v1/responses`。错误已从此前的 403 变化，但仍未获得模型输出；该结果不能证明账号接入限制已经解除，也不足以确定 405 的具体服务端原因。

上述两次重试结束时 NPR 状态仍为 `ready`，角色任务和环境检查任务为失败，取消测试为已中断，没有活动任务。恢复沿用原 R15 工程，没有重新编译，也没有生成角色计划、子任务或资产。

## 指定本地服务后的继续测试

用户更正子 Codex 的服务配置为 `http://127.0.0.1:8317/v1`。通过 Beaver `settings.save` 更新 code、review、translation 三个 Codex 能力的地址和认证，保持模型 `gpt-6-astra`；回读三个配置均为指定地址且 `hasKey = true`。认证由应用凭据库保存，不写入验收文件。

此前 403 和 405 来自旧地址，不能据此判断用户指定的本地服务不可用。继续原角色任务后，模型已经正常响应，读取 NPR 工作流成功，并生成三个带选项、推荐答案和自定义回答能力的澄清问题。验收操作者通过 `task.answer` 选择推荐的“日系可爱比例”“清新夏日”“静态完整角色”。

Beaver 随后创建四个按依赖执行、自动审批的子任务：

- `8e2ccf08-29f6-4caa-8081-117bdf5ea9b2`：用 Blender 制作完整角色与草帽。
- `5c97df0c-2a27-4aa0-818a-27f7e87adfaa`：制作 NPR 贴图材质并接入角色定义。
- `901cf4ca-6202-43c1-adf1-6eba1f922194`：搭建角色展示场景并完成外观调整。
- `2ee005d1-1b14-4a81-a1c9-6469bdfbb70e`：执行最终标准验证并整理模型交付。

首个子任务的专属 Blender 在 5,832 ms 内就绪，PID 30368、端口 57555。调用日志确认 `blender.get_scene_info` 和 `blender.execute_blender_code` 成功，建模已实际开始。空工程尚无 assets 目录时，子 Codex 的一次 `rg` 检查返回目录不存在，随后自行恢复并继续制作；已记录为非阻断问题。

四个子任务最终均为 `completed`、`accepted = true`、`approvalSource = automatic`，每一步合并后自动启动下一步。主任务汇总完成，验收操作者检查结果后通过 `task.accept` 完成最终审批，来源为 `user`。四个专属 Blender 会话的启动和关闭均记录成功；最终进程检查仅有 R15 Beaver PID 27556，没有 Blender 或 Godot 遗留进程。

模型、材质和展示均由 Beaver 管理的 Codex 产出。Beaver 内的 Codex 使用标准工作流对真实 definition 执行 validate 和 preview，均返回 `ok = true`。验收操作者随后通过 Beaver API 对合并后的项目独立执行 `workflow.run` 的 preview：`runId = f07164fc-9e0e-457d-a6b5-ccc211899e5e`，结果为 `initialized = true`、`errors = []`、`engineErrors = false`，生成三张 PNG。通过 API 查看了最终子任务的正侧背图和展示头部近景，白发、草帽、蓝色裙装均可见，前期深色刘海与手掌问题已修复。

实际读取的交付证据确认：源文件与 GLB 一致性检查 `ok = true`，126 个文本文件通过 UTF-8 无 BOM 检查，输入检查 `ok = true`，有界运行 `exit_code = 0`、`timeout = false`。展示支持全身、头部、正侧背、拖动旋转、滚轮缩放及四种主光。

最终工程为本轮目录下的 `Fresh game`。关键交付包括：

- `assets/characters/straw_hat_girl/straw_hat_girl.blend`：23,149,509 字节，可编辑源文件。
- `assets/characters/straw_hat_girl/straw_hat_girl.glb`：11,043,104 字节，引擎模型。
- 同目录 `npr_definition.tres`、`npr_materials.tres`、`textures/npr/` 中的 13 张 PNG。
- `main.tscn`：展示场景；`artifacts/gallery/`：全身、头部与光照检查图。
- `docs/art/straw-hat-girl-delivery.md`：Beaver 生成的交付说明、字段映射和实际证据路径。

## 验收证据

- [编译与安装输出](../output/validation/npr-r15-build.log)
- [打包后的启动输出](../output/validation/npr-r15-packaged-start.log)
- [角色任务、会话生命周期、取消与错误记录](../output/validation/npr-r15-acceptance-proof.json)
- [最新两次重试与 HTTP 405 证据](../output/validation/npr-r15-retry-20260910-1206-proof.json)
- [本地服务配置、五个任务最终状态、会话日志与完整成功证据](../output/validation/npr-r15-local-service-success-proof.json)
- [本轮空白工程证明](../output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/start-proof.json)
- [本轮 NPR 安装证明](../output/validation/fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d/npr-install-proof.json)

## 可固化步骤

已落实并验证：Blender 空白会话、端口隔离、准备完成检查、取消和退出清理。Codex 无需每轮自行排查默认端口、手工启动 MCP addon 或关闭残留进程。暂停和继续会获得新会话，工作流 skill 要求先保存、再通过 MCP 打开本任务已保存的 `.blend`。

R14 已提供的规范读取、角色定义校验和三视图预览工作流已在真实角色上验证。反向几何岛、源文件修改器求值、贴图采样方向和数据导入设置是本轮有价值的通用检查候选；角色形状、比例和服装继续由 Codex 决策。候选步骤尚未追加到产品实现。

## 非阻断问题与验收范围

制作中遇到空场景没有 World、检查命令未产出报告、补丁格式拒绝和 MCP 调用未重新导入 Python 模块等问题，均由 Beaver 内的 Codex 检查状态后恢复，没有由验收操作者补写资产。反向几何和眼部浮起也在后续子任务中修复，源文件与导出保持同步。

仍有可优化项：`task.resourceBytes` 的 500 KB 上限无法读取本轮较大的 Blender PNG，标准 NPR PNG 则可正常下载查看；标准三视图中角色占画面比例偏小，展示场景已有近景；MCP 调用日志目前未从嵌套事件中提取 workflow/action/ok，可从事件和工作流报告读取实际值。近景编织和后发线条偏密、鼻部轮廓较明显，已记录为视觉局限。本轮没有为这些问题中断或重启制作流程。

本次通过范围是 NPR 静态角色的交互、制作、展示与交付工作流。未执行独立游戏软件包导出或包验证，也不据此宣称整个 Beaver 产品所有功能已经完成。

后续按用户要求复用 RoleNPR 原生实验室界面，并完成 Windows EXE 导出、包验证及实际界面冒烟；见 [NPR 原生展示与导出验收](NPR-LAB-EXPORT-ACCEPTANCE.md)。以上角色制作轮记录保留其当时的范围。
