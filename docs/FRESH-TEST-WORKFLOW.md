# 全新工程验收入口

每轮大测试使用 `scripts/start-fresh-native-test.ps1`。脚本先关闭旧 Beaver
实例，再执行可选编译；为新实例分配独立数据目录和 WebView 目录，确认项目和任务
都为空，然后通过 Beaver API 创建 `blank` 工程。最后核对唯一运行实例的路径、
PID 和 EXE 哈希，保存启动证据。旧工程保留，不复制到新一轮。

调用进程需要提供随机 `BEAVER_API_TOKEN` 环境变量，不把凭据作为命令行参数。
使用已有编译包：

```powershell
rtk proxy npm run test:native:fresh -- -Executable release/Beaver-native-0.1.19-preview-r10d-win32-x64/Beaver.exe -Port 4322
```

关闭旧实例、重新编译并开始全新一轮：

```powershell
rtk proxy npm run test:native:fresh -- -Build -Executable target/release/Beaver.exe -Port 4322
```

脚本输出项目 ID、端口和独立目录。后续导入本机服务配置、设置工具路径、提交短提示词、
回答问题和继续制作均调用该实例的 Beaver API。允许复用工具配置和已安装的软件，
不复用游戏工程内容。一个大测试内的后续回答、制作、恢复沿用本轮项目。

空白工程包含 Beaver 自带的 `project.godot`、`main.tscn` 和
`export_presets.cfg` 初始文件。它没有上一轮生成的游戏代码、素材或设计文档。

Windows 扩展路径前缀 `\\?\` 会在目录归属检查前规范化。
无法识别旧进程路径、仍存在旧实例、空数据断言失败或新实例启动失败时，脚本报错，
不假报成功。测试 API 令牌不写入启动证据。

NPR 验收可额外传入 `-NprGodot <定制引擎目录或编辑器路径>`。脚本先创建普通空白
工程并保存 `start-proof.json`，再通过 `project.npr.install` 安装功能包，成功后保存
`npr-install-proof.json`。安装失败时保留空白证明、项目和调用日志；修正路径后可在
同一轮通过 Beaver API 重试。不要把失败的安装标记为角色制作测试通过。
