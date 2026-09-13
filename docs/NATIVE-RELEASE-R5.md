# 原生 R5：完整项目备份与恢复材料

日期：2026-09-08。本轮把上一批的应用数据备份扩展为应用数据与所有登记项目的完整目录归档，并接入单 EXE 命令。完整用户数据迁移仍未完成；恢复结果不能直接启用。

## 发布程序

`release/Beaver-native-0.1.19-preview-r5a-win32-x64/Beaver.exe`

完整发布目录共 **594 个文件、14,945,512 字节**，约 14.95 MB，通过包含清单自身的 50,000,000 字节门禁。应用 EXE 为 **11,389,440 字节**，SHA-256 为 `df1e1a48ab170c6500860d9dd7c1f44ac3acb483ced7cd2c7383f3a74326cd24`。运行后再次验证，发布目录清单和摘要保持一致。

本版继续使用一个应用 EXE，保留系统 WebView2 与 Visual C++ x64 依赖、许可证、启动检查和使用说明。没有增加 Node/Electron 后台或独立备份工具 EXE。旧发布目录、用户数据及默认 Electron 发布入口未替换。

## 已实现的操作

```powershell
.\Beaver.exe --migration-bundle create "旧应用数据目录" "全新备份目录"
.\Beaver.exe --migration-bundle verify "备份目录"
.\Beaver.exe --migration-bundle restore "备份目录" "全新恢复目录"
```

命令在初始化桌面、WebView 和用户数据库之前分派，成功输出 JSON，失败返回非零退出码。先退出 Beaver 并停止项目及工作副本写入工具；目标父目录须已存在，目标目录必须全新。

从隔离的数据库副本读取已登记项目，源数据库不执行 SQLite 查询或 checkpoint。归档同时包含完整应用数据和每个项目目录的普通文件与空目录，保留 WAL、工作副本、会话、原密文，以及项目中的 `.git`、`.godot` 和素材。项目缺失、根目录重叠、链接、文件变化或覆盖已有目标均拒绝；不会静默忽略登记项目。

校验会再次读取归档中的项目记录，与清单逐项比较，因此同时删除清单和项目目录也不能冒充完整归档。原目录不可用时，归档仍可独立校验并恢复。目录外的绝对资源引用、Git worktree 外部 gitdir、网络依赖和系统权限等不属于此目录内容备份的范围。

恢复输出包含应用数据副本、项目副本和原/新路径映射。数据库和凭据保持原始字节，`RESTORE.json` 明确 `paths_rewritten=false`、`credentials_converted=false`、`ready_to_activate=false`。pending 标记在开始恢复时写入，失败残留目录也不能误启用。

原生桌面在实例锁和数据库之前检查 pending。只检查名为 data 的目录曾存在遗漏：误选恢复根目录可能创建新数据库。当前已改为检查规范路径及祖先目录，并覆盖尚不存在的子目录；针对性测试通过。完整契约和下一阶段的字段边界见 [数据迁移说明](DATA-MIGRATION.md)。

## 验证结果

- 最新代码的 Rust 针对性测试 **2 项通过**，覆盖完整恢复、原目录不可用、原字节保留、缺失/重叠项目拒绝、伪造遗漏项目拒绝、已有目标和篡改拒绝，以及 pending 根目录/子目录保护。
- TypeScript 类型检查通过；修改的 TypeScript 文件通过官方 Prettier 检查，Rust 已运行官方格式化及格式检查。
- 新版 Release 完整构建和打包成功；以下运行验证均针对 r5a 发布 EXE，媒体测试另将它复制到单文件隔离目录。
- 完整归档/恢复 **9 项检查通过**：`output/validation/native-migration-1788847586728/proof.json`。使用真实旧 TypeScript Store 留下已提交 WAL，恢复后仍可读取事件、密文和 threadId；源目录内容不变。
- 同一验证实际让测试原目录不可用，再从归档恢复两个项目；两个项目均由真实官方 Godot 4.6 执行验证脚本，退出码 **0**。这证明恢复项目可被引擎读取和运行，不代表整款游戏玩法或艺术质量验收。
- 真实 Tauri/WebView2 桌面回归 **48 项通过，0 页面错误**：`output/validation/native-shell-1788847591804/proof.json`。保留任务控制、合入/回退、设置、工具准备及官方模板导出/游戏运行检查；模型任务仍使用协议夹具。
- 媒体 MCP **16 项通过**：`output/validation/native-media-1788847590613/proof.json`。固定服务响应测试不等于真实供应商质量验收。
- 发布与本轮测试目录相关进程残留复核为 **0**。汇总证据：`output/validation/native-release-20260908-r5/proof.json`。

## 尚未完成

本轮交付的是可独立校验和恢复的材料，尚未执行 `project.path`、`task.workspace`、`operation.taskAfter.workspace` 及托管工具路径的重定位，也未把旧凭据转换与导入激活串接。Codex session 文件和 threadId 保留原样，真实 app-server 在新目录继续旧会话的兼容性仍需验证。不要手动移除 pending 标记来跳过这些步骤。

后续继续在新副本上完成字段级转换、凭据失败保护、正确的文件日志/任务恢复顺序和实际续跑，再接入显式导入 UI。干净 Windows 安装、签名/升级、真实任务中的托盘退出、真实模型创作和其他既定产品门槛仍按原计划推进。
