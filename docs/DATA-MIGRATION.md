# 数据迁移：完整备份与恢复契约

当前实现阶段：Windows 完整归档、恢复、新副本路径和凭据转换、文件日志恢复已接通；真实 Codex 0.153.4 的跨目录旧会话续跑已验证，并新增目标工具探测和 `activate-import` 显式启用。原生设置页已接入迁移对话框；全量用户数据兼容、原生界面集成与完整回退验收仍待完成。

## 备份输入与一致性

输入必须是用户明确指定的旧应用数据目录；Beaver 和外部项目/工作副本写入工具须先停止。Windows 对 SQLite 主库及存在的 WAL/SHM 以仅共享读取的句柄打开，拒绝活动写入者。源目录不通过 Store 或 SQLite 打开，不执行 checkpoint、建表、凭据转换或任务恢复。

为读取项目清单，先将主库、WAL 和 SHM 复制到本次独立临时目录，再在副本上以只读 SQLite 执行完整性检查和项目查询。这样避免只读 WAL 查询在原目录创建或更新 SHM。

完整备份要求数据库中每个登记项目都有独立目录及 `project.godot`。项目缺失、项目目录重叠、项目与应用数据目录重叠、目标位于源数据或任何项目中，均拒绝执行。嵌套项目暂不通过重复复制来改变其目录关系。

目录内的全部普通文件和空目录都进入清单，包括 `.git`、`.godot`、`.beaver-context`、blob、工作副本、Codex 会话、工具安装和原始密文。目录联接、符号链接及不受支持的文件名/类型明确失败。范围是这些目录的内容；目录外绝对资源引用、Git worktree 的外部 gitdir、网络资源和系统元数据没有因此被自动打包。

每个文件按流复制，使用排他创建并同步。复制前后的源清单、整个批次完成后的源清单，以及目标清单均校验大小和 SHA-256。没有为外部项目施加内核级写入锁，需遵守离线前提；不宣称操作系统目录快照、抵御恶意并发替换或断电容灾。

## 归档格式

```text
BACKUP/
  BEAVER-MIGRATION.json
  application/
    BEAVER-BACKUP.json
    data/                       原应用数据完整副本
  projects/
    <project UUID>/              原项目完整副本
```

外层清单记录每个项目的 ID、原始登记路径、规范源路径以及完整文件摘要。校验时从归档内 SQLite 副本再次查询项目，与清单做精确集合比较；即使同时从项目清单和 projects 目录移除一个项目，也不能绕过数据库覆盖检查。校验与恢复不要求原目录仍然存在。

完成清单在复制及复核结束后创建。任何错误都返回失败；部分目录保留供检查，再次创建或恢复不能覆盖它。现有发布与用户数据也不会被当作清理对象。

## 恢复输出与启用保护

```text
NEW_RECOVERY/
  .beaver-migration-pending
  RESTORE.json
  data/                         原应用数据字节副本
  projects/<project UUID>/      项目字节副本
```

恢复先验证归档，再创建全新目标和 pending 标记，最后生成回执。回执包含原数据目录、新数据目录及各项目的原/新路径；`paths_rewritten`、`credentials_converted`、`ready_to_activate` 均为 false。

本阶段不修改数据库里的路径、任务状态、threadId、turnId、未知字段或凭据。完整原生导入尚未实现，不能删除 pending 标记后将这些副本直接投入工作。新原生宿主会在获取实例锁、打开数据库及恢复文件日志之前，检查所选数据目录及其祖先目录是否存在 pending 标记；误选恢复根目录或其子目录也会拒绝。旧 Electron 不认识该标记，不能将它配置为读取恢复目录。

## 原生业务入口

原生统一业务 API 已提供以下方法；桌面桥、经授权的 HTTP 和 MCP 共用目录及参数校验。路径必须是绝对路径，且不能位于运行中宿主数据目录内或包含该目录。

原生设置页顶部的“数据迁移”打开对话框：选择已完成归档并检查，填写全新目标后准备副本，再显式确认启用。更改归档必须重新检查；准备失败保留诊断并要求新的目标目录。已完成分区的副本可选择“启用已有的已分区副本”恢复流程，不重新覆盖恢复目录。启用失败可修正工具 JSON 路径后重试，成功显示 `BEAVER_DATA_DIR` 指向的数据目录，不自动切换环境。处理中阻止重复请求和关闭对话框；不要退出应用或在外部修改迁移目录。旧 Electron 不显示此入口。

- `migration.inspect {backup}`：只读检查已完成归档的项目归属及引用，不启用副本。
- `migration.prepareProjects {backup, destination}`：目标父目录必须存在，目标必须全新；先完整恢复，再按项目分区生成 `.beaver/project.sqlite` 和 `PROJECT-MIGRATION.json`。此流程不同于下述旧 `prepare-import`，准备成功仍保留 pending。
- `migration.activate {backup, prepared, toolPaths?}`：仅接受已有项目分区回执的副本；复用路径、凭据、日志恢复及工具探测流程，成功后才移除 pending。可选 `toolPaths` 是工具路径 JSON 文件的绝对路径。

同一宿主一次只允许一个迁移操作；耗时操作在阻塞线程执行，不长期占用宿主 Store 锁。准备失败保留部分目录，重试须选择新目标；启用失败保留 pending。源目录和归档不作为写入目标，当前宿主及默认数据目录不自动切换，不启动迁移任务或模型请求。工具探测成功及完整切换、回退仍需另外验收。

## 单 EXE 命令

新版原生程序在初始化桌面、WebView 和用户数据库之前处理命令：

```powershell
.\Beaver.exe --migration-bundle create "旧应用数据目录" "全新备份目录"
.\Beaver.exe --migration-bundle verify "备份目录"
.\Beaver.exe --migration-bundle restore "备份目录" "全新恢复目录"
.\Beaver.exe --migration-bundle prepare-import "备份目录" "全新导入目录"
.\Beaver.exe --migration-bundle activate-import "备份目录" "已准备目录" "工具路径.json"
```

目标的父目录须已存在。成功返回 JSON，失败返回非零退出码。没有默认猜测用户生产数据目录，也没有静默选择部分项目的模式。

开发验证入口为 `npm run verify:native-migration -- <Beaver.exe 路径> [Godot.exe 路径]`，只创建隔离测试数据。对应核心模块是 `native/core/src/migration_bundle.rs`，可复用已有 `data_backup` 的目录复制、摘要与数据库句柄保护。

## 导入准备与字段转换

`prepare-import` 先复用完整恢复流程，再获取新数据目录的 `.beaver-native.lock` 排他锁。只打开新副本的 Store，原归档始终保持不变。此命令不接受已有恢复目录，也不会启动桌面、执行器或工具安装。

- `project.path`：从原登记目录映射到恢复项目目录。
- `task.workspace`：验证任务 ID 为 UUID，路径恰好属于旧数据根的 `workspaces/<任务 ID>`，且归档中的工作副本存在，再映射到新数据根。原路径按 Windows 组件比较，支持盘符大小写、`\\?\` 与 UNC 前缀；拒绝路径穿越、ADS、尾随点/空格与前缀碰撞。旧目录可以已经不存在。
- `operation.taskAfter.workspace`：同步转换，防止文件日志恢复时写回旧路径。持久化键是 `taskAfter`，不是 Rust 字段名 `task_after`。
- `settings.tools.codex/godot/blender/node`：仅对旧数据根 `tools` 内且归档文件存在的托管工具重定位；外部工具与命令名原样保留，在回执中列出待复核的工具类型，不执行它们。
- `toolSetup.steps[].result.path`：同步转换托管路径，并将旧探测结果的 `available` 设为 false。运行中的环境准备转为 cancelled，不自动续装。
- Codex 的配置在任务准备时重新生成，历史会话 JSONL 保持不透明，threadId 保留。复制的 `codex/<任务>/state_<数字>.sqlite` 中，已核对的 `threads.rollout_path/cwd`、`project_roots.path` 和 `rollout_migration_skipped_rollouts.rollout_path` 按字段重定位。索引里的 rollout 必须存在于对应新 home，cwd 必须属于新数据/项目目录；未知必需表结构或缺失历史明确失败。该兼容边界已用 Codex 0.153.4 验证，不宣称兼容所有未来或历史版本。
- 旧凭据通过归档应用数据根内的 `Local State` 解密，再转换为原生 DPAPI；原生 DPAPI 密文也必须在当前 Windows 用户下解密成功。任一凭据验证失败均不提交实体路径或密文变更。跨用户无法解密时明确失败，不降级为明文。原始密文同时保留在 `secret_backup`。

Beaver 实体路径更新与凭据替换、原密文备份在同一个 SQLite Immediate 事务中提交。Codex 索引各自使用独立 SQLite 事务；预检之后、Beaver 实体事务之前执行，失败副本保持 pending，没有跨多个数据库的原子提交承诺。随后只对重定位后的项目执行文件日志恢复，再检查是否仍有 applying/aborting 操作；存在阻塞则停止。日志恢复成功后，遗留 running/queued 任务转为 interrupted，不消费队列。任务的 threadId、turnId、未知字段及不透明 Codex 会话文件继续保留。

成功生成 `IMPORT.json`，记录转换、日志恢复、任务中断数量、Codex 索引更新数量，以及原归档两份清单的 SHA-256。`RESTORE.json` 保留最初字节恢复阶段的回执。准备回执的 `ready_to_activate` 为 false，pending 保留至显式启用成功。

路径/凭据提交后的文件日志恢复可能失败或只完成部分文件操作，此时没有成功 `IMPORT.json`，新副本保留 pending 和诊断状态；原归档与原目录不受影响。重新尝试须使用新的目标目录。当前不声称整个文件系统与数据库拥有跨资源原子事务。

开发验证入口为 `npm run verify:native-import -- <Beaver.exe> <已标记的 Electron 测试凭据目录>`，使用隔离旧 Store WAL、真实 Electron safeStorage 测试密文、应用副本和项目副本。`npm run verify:native-session-migration -- <Beaver.exe> <Godot.exe>` 使用真实 Codex 进程和本地 Responses 测试服务，验证同一 thread 的历史保留、新 cwd 工具执行及新项目合入；这不代表真实模型质量验收。

## 显式启用

`activate-import` 必须提供同一个完整归档和尚未移动的准备目录。可选工具 JSON 包含 `codex`、`godot`、`blender`、`node` 四个字符串路径；省略时沿用准备副本里的配置。命令获取排他锁，验证归档清单关联、项目/工作副本归属、Codex 历史路径、凭据可解密性及日志终态，拒绝遗留 running/queued 任务。

接着运行目标机器实际工具的版本探测。Codex 和 Godot 必须可用；启用 Godot MCP 时还要求 Node。其他工具失败保留在 setup 结果中，不下载、不安装、不执行模型请求。成功后写入工具配置和探测状态，同步 `ACTIVATION.json`，最后删除 pending。若写回执后删除 marker 失败，命令返回失败并保留保护，需检查既有结果，不能当作已成功启用。

启用验证检查当前副本的路径与结构，不要求字节仍等于原始归档：凭据转换、日志恢复和实际续跑本身会修改副本。清单不是签名，不提供对恶意篡改的真实性证明。原始归档仍单独进行完整哈希校验并保留。

启用不修改默认数据目录，也不自动启动任务。PowerShell 中显式设置 `$env:BEAVER_DATA_DIR = "已准备目录\data"` 后运行本版 Beaver。旧生产目录继续保留；不要将旧 Electron 指向原生转换后的副本。导入 UI 和返回旧环境的完整验收仍需继续实现。
