# NPR 原生展示界面与 Windows EXE 验收

2026-09-10。已将 RoleNPR 自带的实验室 UI 和场景接入白发草帽女孩，并通过 Beaver 导出 Windows EXE。导出包完整性校验和实际 EXE 界面冒烟检查通过。

## 可运行产物

- [Game.exe](../release/Beaver-game-Q486qv/Game.exe)：双击启动原生 NPR 展示界面，87,200,864 字节。
- [导出目录](../release/Beaver-game-Q486qv/)：运行时保留同目录的 `spoutlibrary.dll`；资源包已嵌入 EXE。
- [导出清单](../release/Beaver-game-Q486qv/export-manifest.json)：包含入口、DLL 和导入日志的文件大小与 SHA-256。
- [完整验收证据](../output/validation/npr-lab-export-proof.json)：场景检查、导出验证、实际运行及已知问题。

EXE SHA-256：`e5c3f0b7391c18ed4e83b9ae583ce7dbe56720130b821b87423027f6463904cc`。

## 场景复用与导出

沿用已完成角色的 R15 工程，作为同一轮的后续交互，没有创建替代模型。任务 `23587a7f-7483-4bfc-8388-4431077a762a` 通过 Beaver API 创建，现为 `completed`、`accepted = true`。

RoleNPR 主项目的 `showcase/npr_lab.tscn` 提供此次使用的完整界面。通用 addon 原本不包含实验室 UI，因此 Beaver 管理的 Codex 将所需场景、脚本和输出 shader 复制到当前工程，替换角色适配器和标题，设置为默认启动场景。RoleNPR 作为只读参考来源使用。

保留全身、半身、面部视角、旋转工具栏、自动转台、拖动、拾取缩放、15 项 NPR 参数和恢复预设功能。项目中的 `artifacts/lab/checks.json` 为 `ok = true`，全部列出的交互检查通过；验收操作者经 Beaver API 查看实际默认界面截图。

Windows Desktop 预设显式使用 `C:/Users/Public/nas_home/godot/export/godot.windows.template_release.x86_64.exe`，与绑定引擎版本 `4.8.dev.custom_build.db1af1e99` 匹配。预设嵌入 PCK，并排除编辑源 Blend、制作文档及验收 artifacts 等内容。

通过 `game.export` 导出，返回 `bundleVerified = true`。随后调用 `game.verifyExport`，确认 3 个清单文件共 87,451,104 字节。实际运行检查结束后再次调用同一验证接口，结果一致。导出日志未发现 ERROR、SCRIPT ERROR 或 WARNING 行。

## 实际 EXE 检查

另经 Beaver API 创建任务 `55c0aa27-ff83-48a9-826a-28d8c766316d`，由应用内 Codex 启动导出的 `Game.exe`。验收操作者没有直接启动 Codex、Godot 或 Blender，也没有补写模型、场景或游戏代码。

实际 EXE 窗口 PID 为 38692。白发草帽女孩、蓝色裙装、中文 NPR 参数 UI 和地面阴影正常显示；通过窗口鼠标消息检查了全身、半身、面部切换、右转 15 度及恢复预设。直接捕获目标窗口，截图经 Beaver API 读取并检查。

截图和检查报告保存在本轮任务工作副本的 `artifacts/export-smoke/`：

```text
C:\Users\Public\nas_home\Beaver\output\validation\fresh-round-20260910-063500-8e40a3bcd84140c9883edb4a2d6e030d\data\workspaces\55c0aa27-ff83-48a9-826a-28d8c766316d\artifacts\export-smoke
```

`02-window.png` 为实际 EXE 全身界面，`03-face.png`、`04-half.png`、`05-rotate.png`、`06-reset.png` 分别记录面部、半身、旋转和恢复。`review.md`、`baseline.json`、`process.json`、`result.json` 保存检查过程。

启动前后的既有项目文件和导出文件哈希未变，证据目录以外未增加项目文件。本次测试进程已通过 CloseMainWindow 关闭，最终进程检查仅保留 R15 Beaver。退出码未能取得，因此不声明 exit code 0；空运行日志也不作为引擎内部无错误的证明。

## 已记录的问题与范围

审查任务保存截图和报告后触发了 Beaver 的只读审查限制，最终状态为 `conflict`，提示“审查任务修改了文件，未合入原项目”。新增内容仅为验收证据，已保留在任务副本。该问题没有改变或阻止 EXE 交付；后续可改进只读审查任务的独立证据存储。

第一次区域截图受其他桌面窗口遮挡，改用 PrintWindow 后获得有效截图。近景鼻部描边较重、后发细线偏密属于既有模型的视觉局限，本次没有重做美术。

Beaver 的导出清单仍按现有契约记录 `runtimeVerified = false`，没有手工篡改这一字段；实际 EXE 启动与界面检查由上述独立运行证据证明。EXE 冒烟没有覆盖全部参数、自动旋转、拖动缩放和长期稳定性；项目级界面检查覆盖了这些基础交互。
