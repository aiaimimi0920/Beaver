# 0.1.0 验证记录

以下保留初版历史证据。最新 UI 与工具发现修复见 [0.1.1 更新与验证](./UPDATES-0.1.1.md)。

日期：2026-09-06。平台：Windows x64。本次区分源代码检查、真实本机运行、协议固定应答和用户质量验收，不将它们混为一谈。

## 已取得的证据

| 验证 | 结果与边界 |
| --- | --- |
| TypeScript | `npm run typecheck` 通过，包含核心、UI、测试与 TS 验证脚本 |
| 自动测试 | `npm test` 20/20 通过：二进制快照、独立并发合入、同文件冲突拒绝、保留文件回退、路径/链接防护、锁、接管已有项目、密钥路由、重启记录、合入/回退崩溃恢复、外部修改保留、中止时序、功能块基线与导出目标 |
| 媒体 MCP | 真实 stdio 子进程 + 本机 HTTP fixture；图像参考 multipart、自定义路径、音频、翻译、非法输出在请求前拒绝通过。不是供应商生成质量测试 |
| Codex | 实际 CLI 0.144.5；独立 HOME、app-server 初始化、真实 thread/turn、同项目两个并行任务、steer、中止、同 thread 续跑完成均通过。上游是明确标注的固定 Responses 应答，不调用真实模型 |
| 桌面 | 真实 Electron：模板创建、重复接管、素材枚举、图片加载/框选/任务引用、OBJ 渲染、WAV 解码、功能块、工具检测、保存设置、缺少 AI 时报错、关闭到托盘、完全退出后记录恢复 |
| GDScript | 本机 `gdscript-post-check --format` 通过，未发现格式/lint 问题 |
| Godot | 实际 4.6 引擎导入项目；三种配方、重复提交防重、三位客人剧情、存读档均通过 |
| 成品 | Windows Desktop 真正导出 Game.exe，成品 headless 启动通过；另行正常窗口启动并截取游戏窗口，中文 UI 可见 |
| 依赖审计 | `npm audit` 与生产依赖审计均为 0 vulnerabilities（只表示当时注册表审计结果，不是安全保证） |
| UI 来源 | 保留 Neuro 13 个文件 + Loom 3 个文件的原始副本与 sources.json；未修改来源目录 |

## 证据文件

每次真实运行脚本创建独立目录，不覆盖旧测试项目：

- `output/validation/codex-*/protocol-proof.json` 与 `lifecycle-proof.json`。
- `output/validation/desktop-*/desktop-proof.json`、界面 PNG、`standalone-window.log`。
- `output/validation/godot-*/godot-proof.json`、导入/玩法/成品运行日志。
- `output/validation/godot-*/Beaver-game-*/export-manifest.json` 与实际 `Game.exe`。
- 打包后 `release/Beaver-win32-x64/RELEASE.json` 记录文件清单和 SHA-256。`runtimeVerified` 初始为 false；只有打包产物通过桌面脚本并且逐文件摘要验证通过后，验证脚本才写为 true。

用于真实导出的官方 Godot 4.6 模板来自 [Godot 官方发布](https://github.com/godotengine/godot-builds/releases/tag/4.6-stable)，完整下载归档 SHA-256 已核验为：

```text
3b30ac8c1772f25f5dfa5f65922cab0a90e7b960176891237c5772515ebccc46
```

模板提取到 Beaver 工作区的验证目录，并通过仅本次测试的 APPDATA 环境传入；没有覆写个人 Godot 导出模板目录。验证程序结束会清理它自己创建的游戏/任务进程，不按名字杀掉其他工作。

## 实测发现并修复的问题

- 文件合入 intent 原本没有启动恢复；现已实现可重放日志、冲突保留和恢复失败阻断。
- 退出时异步收尾可能继续访问已关闭数据库；现先终止并等待任务持久化和受管理操作。
- 启动期间中止、没有 turn 终止事件、steer 与完成通知竞争均有明确处理。
- 免 Key 本机服务不应写必填 env_key，实际 Codex 协议测试发现并修复。
- 参考图不能覆盖用户设置的自定义媒体 route。
- 导出文件后缀必须来自预设目标，不能来自宿主操作系统。
- Windows 试玩不能使用隐藏后台工具窗口的启动参数；真实窗口截图测试发现并修复。
- 素材更新刷新版本、OBJ 带版本查询参数、弹窗焦点和重复提交等交互边界已处理。

## 尚未取得的证据

- 真实付费/自配 AI 服务生成完整游戏的质量、费用和长时间无人值守稳定性。
- Blender 安装、add-on 自动启用及真实 3D 生成到游戏摆放的闭环。
- 无开发环境的全新 Windows 机器上所有安装按钮、权限提示和导出模板获取。
- macOS/Linux 安装包、系统权限和真实玩家导出验收。
- 云端商业余额、支付结算、集市与渠道发行。
- 断电、损坏磁盘、外部程序在检查/写入瞬间竞争、超大项目素材库的压力与容灾。

因此交付名称是**本地预览版**，不是“全部产品目标已经完成”。原创起步游戏的最终美术、剧情和玩法品质仍由用户验收。
