# F7 普通文件正式导入证据

日期：2026-09-25。范围：普通文件/文件夹准备记录的确认、正式提交、恢复、中止和对象导航。内部版本保持 0.1.19.36；本轮没有发布、提交或推送。

## 可用流程

在对象页打开导入弹窗，准备普通文件或文件夹并保存分组；从准备历史打开该记录，确认目标布局后执行“确认正式导入”。每个分组生成一个新对象，未分组文件各自生成对象。成功结果提供“打开导入对象”，清除目录过滤、重新加载目标目录并选择对象；详情显示“导入待验证，尚未接受”。项目已切换时要求先切回原目标。

重开记录只查询状态，不自动提交。关闭弹窗、丢失响应或重启后，使用原 preparationId 查询、继续或中止。该 ID 同时是稳定操作身份；成功重试不生成重复对象，已中止记录必须重新准备才能再次导入。

## 持久化与安全边界

- Desktop API：`object.commitFileImport`、`object.fileImportOperation`、`object.abortFileImport`，参数均为 `{ projectId, preparationId }`。操作仅路由至目标项目 runtime。
- 首次提交在冻结前后核对来源快照，将内容存入目标 blob 存储，全部成功后才创建 applying 日志。此后继续与中止不依赖源文件。冻结失败可能留存未引用 blob，但不会登记对象或开始文件写入日志。
- 目标路径为 `imports/{preparationId}/root-{sourceRootIndex}/{relativePath}`。保留单个来源根目录内的布局，不改写内容、绝对路径或跨根引用；禁止来源与目标根重叠。
- 每个文件先持久化写入意图，再校验 blob，通过临时文件及不覆盖提交落盘。未归属于本日志的已存在目标，即使字节一致，也阻止导入。
- 文件成功后在一个 SQLite 事务内登记所有新对象、版本、目录投影与完成状态。失败不部分登记，保留 applying 状态和错误供恢复。
- 中止先持久化 aborting，仅清理日志记录且内容仍匹配的文件；外部修改或新增目录归属会阻止清理。冲突解除后可重试。空目录、冻结 blob 和准备历史保留。
- 新版本使用 `importedPendingValidation`，不进入已接受版本引用列表，也不能通过现有 Captured 版本接受入口直接批准。原准备记录不可变，其 `readyToCommit: false` 不代表当前执行状态；正式操作记录是提交状态来源。

## 验证结果

所有命令在正常 PowerShell 环境执行，外部程序使用 `rtk proxy`；没有运行全量测试或原生发布验收。

- `cargo test --locked -p beaver-core --lib object_file_import -j 1`：11 通过，0 失败，包括 6 项正式导入回归和 5 项准备测试。覆盖分组与路径、源只读、冻结后源离线、prepared/intent/written/beforeCommit 恢复、源漂移、目标冲突、中止保留外部修改、最终事务回滚及幂等重试。日志：`output/file-import-core.log`。
- `cargo test --locked -p beaver-desktop object_import -j 1`：14 通过，0 失败。覆盖目标隔离、源离线重试、查询、错误目标与终态拒绝中止。日志：`output/file-import-desktop.log`。
- `npx tsx --test tests/object-file-import.test.ts tests/object-import-history.test.ts tests/object-catalog.test.ts`：14 通过，0 失败。覆盖只读重开、响应丢失重试、中止冲突、迟到响应和跨身份回执拒绝、待验证状态显示与引用排除。日志：`output/file-import-ts.log`。
- 首次 typecheck 发现结果面板 nullable operation 的 TS18047；改为直接检查 `operation?.state` 后，`npm run typecheck` 通过，并重跑正式导入前端测试：4 通过，0 失败。日志：`output/file-import-typecheck-recheck.log`、`output/file-import-ts-recheck.log`。
- 修改的 Rust 文件定向 rustfmt、TS/TSX 文件 Prettier 通过；修正后的面板 Prettier check 通过。日志：`output/file-import-rustfmt.log`、`output/file-import-prettier.log`、`output/file-import-panel-format.log`。
- `npm run check:effective-lines`：1027 个源文件，0 违规；未修改历史基线。日志：`output/file-import-lines.log`。
- `git diff --check` 通过；新增源文件为 UTF-8 无 BOM。

独立只读 Core 审查未发现实质恢复或事务缺陷。UI 审查提出的“导入到已有对象、同一对象新增版本”场景不适用于此实现：目标身份必须未占用，导入仅创建新对象；目录刷新具备 loading/error 状态，完成提示描述已持久化的导入。实际浏览器点击和原生窗口导航未执行，不能据静态渲染测试声称完成原生 UI 验收。

## 剩余范围

本轮限制为每次 512 MiB、1024 个对象及每对象 1024 个文件；不运行 Godot、资源兼容性检查或导入批准。待验证版本的验证与提升流程仍需后续实现。

已登记 Beaver 项目和外部 Beaver 项目的自有内容/固定依赖闭包正式导入、跨来源统一提交机制、未保存表单草稿恢复，以及生成对象弹窗完整导航尚未完成。因此 F7.4、F7.5 保持未勾选。本证据只关闭普通文件来源的正式提交路径，不代表 F7 全部完成或发布验收通过。
