# Beaver 原生预览 R7：真实会话迁移与显式启用

日期：2026-09-09（Asia/Shanghai）。本轮接通真实 Codex 旧会话跨目录续跑、目标工具版本探测和显式启用，并从发布目录打开启用后的数据副本。导入 UI、完整回退和正式产品验收尚未完成。

## 发布产物

`release/Beaver-native-0.1.19-preview-r7-win32-x64/Beaver.exe`

完整目录 **594 个文件、15,084,683 字节**，约 15.08 MB，包含许可证、启动脚本和清单自身，低于 50,000,000 字节门禁。EXE 为 **11,526,144 字节**，SHA-256 为 `b4dd3cd6d0a3c41bc83fef87ef9e2987b44bae6091c22486446c2b88aae01234`。Release 构建耗时 4 分 15 秒，运行后载荷再次通过完整摘要校验。

保留单 EXE、系统 WebView2 和 Visual C++ x64 依赖。旧发布、用户生产目录和默认 Electron 构建入口未替换。发布清单的 `runtimeVerified=false` 继续保留；本报告说明实际覆盖，避免将部分验收解释为完整产品已交付。

## 修复与实现

真实 Codex 0.153.4 首次续跑返回 `no rollout found for thread id`。检查隔离测试副本后确认，JSONL 已完整复制，但 `state_5.sqlite` 中的 `threads.rollout_path` 和 `cwd` 仍指向旧目录。R7 在准备阶段对复制的 `state_<数字>.sqlite` 已核对字段做重定位，保留 thread ID、JSONL 和其他未知字段。未知必需表结构、索引指向缺失历史或目录外的路径明确失败，不删除历史绕过问题。

Codex 索引使用各自数据库事务，Beaver 实体路径/凭据继续使用同一个事务。跨多个数据库与文件恢复没有全局原子性承诺；中途失败的副本仍保留 pending 和诊断材料，原始归档不受影响。

新增命令：

```powershell
.\Beaver.exe --migration-bundle prepare-import "完整备份目录" "全新导入目录"
.\Beaver.exe --migration-bundle activate-import "完整备份目录" "已准备目录" "工具路径.json"
$env:BEAVER_DATA_DIR = "已准备目录\data"
.\Beaver.exe
```

可选工具 JSON 包含 codex、godot、blender、node 四个字符串字段；省略时沿用准备副本中的配置。启用要求原始归档完整且清单摘要与准备回执匹配，准备目录未被移动，排他锁可获取，项目和工作副本归属正确，凭据可解密，文件日志没有未完成项。随后执行实际版本探测；Codex、Godot 必须可用，启用 Godot MCP 时还要求 Node。不会安装工具、发送模型请求或修改默认数据目录。

成功时保存探测结果，写入并同步 `ACTIVATION.json`，最后删除 pending。准备阶段的 `IMPORT.json` 保持原始阶段记录。启用复核当前目录结构和路径，不与备份原字节强制相等，因为凭据转换、日志恢复和续跑均会合法修改副本。该机制不构成防恶意篡改签名系统。

运行验证还发现，Windows 对自身持有的排他锁文件拒绝读取；原先对整个 data 目录做摘要会导致启用报 os error 33。已改为检查目录结构、链接和文件类型，避免读取锁文件。桌面验证最初读取 `.txt` 时被 Markdown 资料接口正确拒绝，测试文件随后改为 `.md`，没有放宽产品接口。

## 新鲜验证

- Rust 迁移测试 **9 项通过**：完整归档 2 项、导入 5 项、启用 2 项；锁文件修复后启用 2 项再次通过。覆盖未知 Codex schema、缺失历史、原路径拒绝、归档关联和排他所有权。
- TypeScript 类型检查、相关 Prettier 和 Rust 格式检查通过。
- 发布程序的真实旧会话及启用 **9 项通过**：`output/validation/native-session-migration-1788903268642/proof.json`。真实旧 TypeScript 执行器创建 Codex 持久会话，旧源目录随后不可用；发布 EXE 负责归档、准备、启用，原生 launch/executor/task_finish 通过测试入口运行真实 Codex。相同 thread 恢复，旧助手历史进入新请求，真实 exec_command 在新 cwd 写入，原生合入只修改新项目。
- 上述运行使用 Codex **0.153.4** 与本地固定响应服务，未使用生产密钥或付费模型。真实进程/持久化/工具执行得到验证，真实模型创作质量没有因此被证明。
- 缺少 Codex 的启用失败保留 pending；随后实际探测 Codex 0.153.4、Godot 4.6、Blender 5.2.1 LTS、Node 22.22.2，显式启用成功。原归档、旧应用和旧项目内容保持不变。
- 启用副本真实桌面 **4 项通过，0 页面错误**：`output/validation/native-activated-1788903337502/proof.json`。导入项目及旧 thread 可见，已完成任务没有重新运行，合入 Markdown 文件可通过原生资料 API 读取。
- 旧凭据/WAL 导入准备 **8 项通过**：`output/validation/native-import-1788903278825/proof.json`。
- 常规真实桌面 **48 项通过，0 页面错误**：`output/validation/native-shell-1788903308477/proof.json`；包含官方 Godot 模板导出及程序运行，模型任务仍使用协议夹具。
- 媒体 MCP **16 项通过**：`output/validation/native-media-1788903284675/proof.json`。相关 Beaver/Godot/Codex 测试进程残留复核为 **0**。

汇总证据：`output/validation/native-release-20260909-r7/proof.json`。完整命令与约束见 [数据迁移契约](DATA-MIGRATION.md)。

## 剩余工作

下一阶段接入导入 UI 与可审查的目录选择/错误展示，再完成返回旧环境的全流程验收。Codex 私有索引兼容性目前仅有 0.153.4 的真实证据；有缺失历史目录的用户数据会明确失败，需要进一步提供诊断和处置流程。

真实任务或安装过程中的托盘退出、物理标题栏拖动、全新 Windows、签名/更新和真实模型游戏创作质量仍未完成。当前可显式启用经过校验的新副本，但不能据此将整个项目标记为完成。
