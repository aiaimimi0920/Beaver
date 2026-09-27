# F7 导入准备历史恢复证据

日期：2026-09-25。

## 可用流程

对象导入弹窗支持发现目标项目已保存的准备记录，按记录标识分页，并选择记录读取冻结详情。关闭重开或准备响应丢失后，无需重新访问源项目即可找回记录；成功准备后列表自动刷新。历史读取独立于当前表单，不覆盖正在编辑的内容。

支持已登记 Beaver 项目、外部 Beaver 项目以及普通文件/文件夹。详情展示准备和请求 ID、目标项目、源身份、冻结文件及哈希、固定版本依赖、文件分组与目标身份映射。界面明确显示“正式导入尚未提交”和“未重新检查源文件”。

## 实现边界

- Core：目标项目数据库内分页读取两种准备记录，单页最多 50 项；验证目标存在，不打开源路径，不创建对象。
- Desktop：新增只读 `object.importPreparations`，详情复用 `object.getImportPreparation` 和 `object.getFileImportPreparation`。
- 前端：验证目标、请求、记录身份、分页顺序、冻结依赖闭包及映射；迟到响应不能覆盖新选择，关闭会话使在途响应失效。列表查询失败保留已有条目，可以重试。
- 生产入口：`ObjectImportDialog.tsx` 挂载 `ObjectImportHistoryPanel.tsx`；历史读取不调用 inspect、prepare 或写入接口。

## 定向验证

以下命令通过；原生检查在现有 PowerShell/MSVC 环境执行，未修改结构基线。

| 检查                                                                                                                                | 结果                       | 日志                                  |
| ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------- | ------------------------------------- |
| `cargo test --locked -p beaver-core --lib object_import_preparation -j 1`                                                           | 25 passed，0 failed        | `output/import-history-core.log`      |
| `cargo test --locked -p beaver-desktop object_import -j 1`                                                                          | 13 passed，0 failed        | `output/import-history-desktop.log`   |
| `npx tsx --test tests/object-import-history.test.ts tests/object-import-contract.test.ts tests/object-import-file-contract.test.ts` | 15 passed，0 failed        | `output/import-history-ts.log`        |
| `npm run typecheck`                                                                                                                 | 通过                       | `output/import-history-typecheck.log` |
| `npm run check:effective-lines`                                                                                                     | 1019 sources，0 violations | `output/import-history-lines.log`     |
| 本次 7 个 TS/TSX 文件 Prettier check、6 个 Rust 文件 rustfmt check                                                                  | 通过                       | 终端结果                              |
| `git diff --check`                                                                                                                  | 通过                       | 终端结果                              |

覆盖 Store 关闭重开后 53 条混合记录分页、跨目标隔离、源路径离线后的项目准备重开、源文件删除后的 Desktop 详情读取、查询重试、迟到响应、错误身份和映射拒绝，以及生产弹窗挂载和冻结详情 SSR。

13 个本次改动的源文件和测试文件检查均无 UTF-8 BOM。编译仅报告已有的 unused_mut/dead_code 警告。本轮未启动原生 WebView 或执行完整游戏验收；SSR 和会话测试不等同于原生交互验收。

## 未完成项

F7.4/F7.5 保持未勾选：正式导入仍需冻结内容复制、持久文件日志、多对象最终事务、导入待验证登记、提交重试与中止及结果导航。未保存的表单草稿尚不支持跨关闭恢复。本轮恢复的是已经持久化的准备记录，未提交导入、未自动重试准备、未发布对象，也未删除源文件。
