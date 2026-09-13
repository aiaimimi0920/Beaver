# Beaver 原生预览 R6：导入准备

日期：2026-09-08。本轮实现从完整备份到全新原生副本的路径、凭据和恢复流程。准备完成后仍保留 pending，完整迁移与启用尚未结束。

## 发布产物

`release/Beaver-native-0.1.19-preview-r6-win32-x64/Beaver.exe`

完整发布目录为 **594 个文件、15,018,479 字节**，约 15.02 MB，包含清单自身、许可证和启动脚本，低于 50,000,000 字节门禁。EXE 为 **11,461,120 字节**，SHA-256 为 `2d08ce9ad098c1c4f8201ea6ed4aab752ce08301885182435c8f3fe74ac324a7`。Release 构建耗时 4 分 19 秒；完成运行检查后，完整目录再次通过文件集合、大小与摘要校验。

继续只分发一个应用 EXE，依赖系统 WebView2 和 Visual C++ x64 运行库。旧发布目录、用户生产数据和默认 Electron 构建入口均未替换。发布清单的 `runtimeVerified` 保持 false，具体运行覆盖与未完成项由本报告及独立 proof 说明，避免把部分验收标记为全部完成。

## 新命令与恢复顺序

```powershell
.\Beaver.exe --migration-bundle prepare-import "已校验的完整备份目录" "全新导入目录"
```

命令先恢复完整应用数据和全部登记项目，创建 pending，再获取新数据目录的排他锁。源目录和原归档不通过 Store 打开。目标必须全新；已有恢复目录也不能原地覆盖。

字段级转换覆盖 `project.path`、`task.workspace`、`operation.taskAfter.workspace`、settings 中的托管工具路径及 setup 探测结果路径。工作副本必须属于旧数据根下当前任务 UUID 的固定目录。旧路径按 Windows 组件校验，支持命名空间和 UNC，拒绝路径穿越、ADS 与字符串前缀碰撞。外部工具不执行、不猜测迁移，保留原值并列入待探测项。

旧 Electron safeStorage 凭据使用归档中的对应 `Local State` 解密后转换为原生 DPAPI；已有 DPAPI 密文也验证当前用户能否解密。所有路径变更、凭据替换和原密文备份在同一个 SQLite Immediate 事务中提交。任一凭据或路径预检失败，不提交这些变更；原密文保留。

之后在新项目副本上执行文件日志恢复。仍有 applying/aborting 日志时返回失败，保留 pending，不生成成功回执。日志恢复完成后，旧 running/queued 任务才转为 interrupted，不自动消费队列。日志中的 taskAfter 同步重定位，避免恢复提交覆盖成旧 workspace。threadId、turnId、不透明 Codex 会话文件和未知任务字段保留。

成功时生成 `IMPORT.json`，列出转换和恢复计数；`ready_to_activate` 仍为 false，pending 不删除。文件恢复可能在数据库转换提交后失败，此时新副本包含诊断状态或部分恢复工作，原备份保持不变；重新尝试应从原备份导入另一个新目录。详细格式与边界见 [数据迁移契约](DATA-MIGRATION.md)。

## 验证结果

- Rust 导入测试 **4 项通过**：离线重定位与日志提交顺序、损坏凭据/非法工作副本拒绝、未完成日志保留 pending、Windows 路径边界。旧凭据模块 **1 项通过**，完整备份/恢复模块 **2 项通过**。
- TypeScript 类型检查、相关官方 Prettier 检查和 Rust 格式检查通过。
- 发布 EXE 的导入准备 **8 项检查通过**：`output/validation/native-import-1788867927116/proof.json`。使用真实旧 TypeScript Store 的已提交 WAL，以及此前由真实 Electron 生成且明确标记的测试凭据；没有读取生产凭据。
- 同一测试让旧应用和项目目录不可用，再运行发布命令。日志只改动新项目，旧 queued 任务转为 interrupted，恢复后的旧 Store 能读取数据；原归档、原应用、原项目和测试 Local State 的字节保持不变。
- pending 桌面入口验证保留新副本字节。首轮 smoke 的“拒绝启动后不产生 WebView 目录”断言失败：Tauri 会在 setup 保护前初始化运行库元数据。随后将无 WebView 初始化检查放在无界面命令结束处；桌面拒绝检查聚焦于应用数据不变，复跑通过。该修正仅涉及验证脚本，发布 EXE 未变。
- 真实 Tauri/WebView2 桌面 **48 项通过，0 页面错误**：`output/validation/native-shell-1788867881727/proof.json`。包含真实官方 Godot 模板导出、导出程序运行和游戏预览；任务的模型响应仍使用协议夹具。
- 媒体 MCP **16 项通过**：`output/validation/native-media-1788867879353/proof.json`。固定响应服务覆盖协议与错误处理，不代表真实供应商质量验收。
- 工作区内相关 Beaver/Godot 测试进程残留复核为 **0**。本轮汇总：`output/validation/native-release-20260908-r6/proof.json`。

## 后续门槛

真实 Codex app-server 在重定位 cwd 下继续旧 thread 的行为仍未验证，当前不改写 session JSONL 或清空 threadId 来绕过。目标工具重新探测、显式启用与导入 UI 尚未接入，不能手动删除 pending 使用准备副本。

下一步应完成真实旧会话续跑证据，再完成工具探测、启用回执及导入 UI。返回旧环境的端到端验收、真实任务和安装过程中的托盘退出、物理标题栏拖动、全新 Windows、签名升级及真实模型创作质量仍保留在既定计划中。
