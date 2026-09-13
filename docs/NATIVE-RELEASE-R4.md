# 原生发布与离线备份进度

日期：2026-09-08。原生完整目录打包与应用数据离线备份阶段已实现；完整迁移和产品计划仍未完成。

## 可运行发布目录

程序位于 `release/Beaver-native-0.1.19-preview-r4b-win32-x64/Beaver.exe`。目录共 **594 个文件，14,836,722 字节**，约 14.84 MB，包含应用、启动检查脚本、使用说明、第三方许可证与完整性清单，低于 **50,000,000 字节**。

发布目录只有一个应用 EXE，大小 11,281,920 字节，SHA-256 为 `f230dadf0ac49580b226d1d166bb4a2112e5bd5d126269d4325ccd6580627e5c`。不会从 target 目录顺带复制历史 `beaver-media.exe`，没有附带 Electron 或 Node 后台。

新增 `npm run package:native -- <新的目录名>`，先构建原生 UI 和 Rust release，再创建仓库 release 根内的全新目录。目录名必须通过白名单校验；已有目录、被重定向的 release 根和缺失许可证会使打包失败。失败候选保留供检查，不删除或覆盖已有发布。

`npm run verify:native-release -- release/Beaver-native-0.1.19-preview-r4b-win32-x64` 会核对完整文件集合、每个文件的大小和 SHA-256、单 EXE 约束，以及包含 RELEASE.json 自身的总大小。额外文件、内容变化、额外运行时和超限均有实际拒绝测试。完整性检查不自动将 `runtimeVerified` 改为 true；运行结果单独保存，避免将哈希检查等同于运行证明。

许可证目录包含保守的 Windows Cargo 依赖集合（含构建依赖）、6 个前端依赖，以及 Rust 标准库的版权与许可文本。crate 未附带文本时，从包记录的固定上游提交取得；selectors 使用 Mozilla 官方 MPL 2.0 文本并记录确切源码地址。没有把只有 SPDX 名称的清单充当许可证全文。

## 外部运行依赖

实际检查 PE 导入表确认，除了 Windows 系统组件，还需要 `VCRUNTIME140.dll` 和 `VCRUNTIME140_1.dll`。发布包明确采用外部 **Microsoft Visual C++ v14 x64 Redistributable** 与 **Microsoft WebView2 Evergreen Runtime**。`Start-Beaver.ps1` 在启动前检查 VC DLL 和 WebView2 注册信息，缺少时提示微软下载地址，不自动安装或修改执行策略。

依赖策略按 [Microsoft WebView2 分发文档](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution) 和 [Visual C++ Redistributable 文档](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)记录。检查存在性不证明运行库完整或版本匹配；当前没有在卸载这些运行库的干净机器上验收。应用目录体积不包含这些系统运行库，也不包含另外安装的 Codex/Godot/Blender、用户数据或生成游戏。

## 本轮发布验证

- 新鲜 Rust release 构建成功，TypeScript 类型检查与修改文件的官方 Prettier 检查通过。
- 新增发布门禁测试通过，覆盖目录内容变化、注入第二个 EXE、更新清单后仍拒绝额外运行时，以及清单计入体积后的超限。
- 发布目录中的确切 EXE 通过 **48 项**真实 Tauri/WebView2 桌面检查，**0 页面错误**：`output/validation/native-shell-1788842721153/proof.json`。
- 桌面检查覆盖真实项目/资料/任务 IPC、任务创建与补充/中止/继续、人工提问、回退、功能块，以及官方 Godot 4.6 已安装模板的实际导出、成品启动和真实场景执行。任务模型部分仍使用原生协议夹具，不代表真实付费模型创作质量验收。
- 从该发布 EXE 复制出的单文件隔离目录通过 **16 项**媒体 MCP stdio/HTTP 互操作检查：`output/validation/native-media-1788842719359/proof.json`。这是固定服务响应验证，不代表真实媒体供应商验收。
- 旧 Electron 发布目录和默认 `package` 入口未替换；当前产物仍标记为未签名的原生迁移预览。
- 运行后再次核对完整发布目录，594 个文件与摘要均保持一致；本次桌面/媒体测试进程残留为 0。启动检查脚本通过 PowerShell 语法解析，缺失运行库的真实机器交互尚未验证。汇总证据：`output/validation/native-release-20260908-r4/proof.json`。

## 后续推进：应用数据离线备份

新增 `native/core/src/data_backup.rs`，提供原生 create、verify、restore，开发入口为 `native/core/examples/data_backup.rs`。此模块在上述发布包验证之后添加，**尚未接入桌面导入 UI，也不作为该发布 EXE 已有的迁移功能交付**。

备份保留应用数据目录中的全部普通文件和空目录，包括 SQLite 主库、存在的 WAL/SHM、blob、任务工作副本、Codex 会话、Local State 与不透明凭据。不沿用项目快照的忽略列表，因此 `.beaver-context` 等目录不会丢失。源目录仅以文件方式读取，不调用可写的 Store::open，不主动 checkpoint、解密或恢复任务。

Windows 通过只共享读取的数据库文件句柄拒绝已有和后续写入者。应用和其他工作副本写入工具须先停止；创建清单、复制与复核仍属于文件级操作，不宣称内核级目录快照或断电容灾。所有复制文件同步后才写入完成清单。失败保留部分目标，后续操作拒绝把它当成有效备份或覆盖它。

restore 先校验备份，再复制到全新目录；拒绝已有目标、路径越界、目录联接/符号链接、篡改与额外文件。备份范围在清单中明确标记为 **application-data-only**：外部 Godot 项目目录必须另行备份。恢复保持旧的绝对路径、任务状态和密文字节，**不要将恢复目录直接当作已经完成路径与凭据迁移的原生数据目录启动**。

验证入口：`npm run verify:native-backup`。本轮使用真实旧 TypeScript Store 写入后立即退出，明确保留已提交 WAL，再由 Rust 备份与恢复，最后由旧 Store 读取恢复数据。

- Rust 针对性测试 **2 项通过**：活跃数据库拒绝、原字节恢复、已有目标拒绝、篡改拒绝、真实 Windows junction 拒绝。
- 新旧互操作 **7 项通过**：`output/validation/native-backup-1788844409972/proof.json`。确认 WAL 中提交的任务与事件可读，工作副本/blob/profile 与未知字段保留，源目录内容完全不变；测试没有使用真实用户数据或凭据。
- 新增模块已通过 `cargo check --locked -p beaver-desktop`、`cargo fmt --all -- --check` 和 TypeScript 类型检查。

后续顺序是：外部项目备份与恢复证明；从已校验备份导入全新原生目录；重写任务 workspace、日志中 task_after 及应用内部工具路径等已知字段；在副本中转换旧凭据；持有目标独占锁后按文件日志恢复、任务恢复顺序启动；验证真实会话继续与返回旧环境。不得通过递归替换所有 JSON 字符串改写用户内容，也不得自动处理用户生产目录。

仍未完成的整体门槛包括完整用户数据迁移、真实任务/安装中的托盘退出、物理标题栏拖动、真实模型创作质量、干净 Windows 安装、签名/升级，以及平台账户和结算后台。其他平台与已明确延期的集市、云执行、渠道发行继续按原计划推进。
