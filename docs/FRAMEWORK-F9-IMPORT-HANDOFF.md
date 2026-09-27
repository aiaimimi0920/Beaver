# F9 导入准备与正式提交衔接

日期：2026-09-26。范围：F9.1/F9.2 的对象导入路径；整体 F9 保持未完成。

## 可用结果

保存项目对象或普通文件的导入准备后，界面直接读取并展开该准备的冻结详情和正式提交面板。即使新记录不在历史第一页，也不必翻页查找。历史列表和分页游标保持服务端返回值，不插入虚构列表项。用户仍须核对冻结依赖和目录布局，勾选确认后才能正式提交。

已存在的正式导入入口继续分别调用 `object.commitImport` 和 `object.commitFileImport`；完成后的对象保持 `importedPendingValidation`，查询或重开不会重复提交。本批修正了项目导入仍显示“提交未接通”的过期文案，没有新增 Core 写入入口或改变持久化格式。

切换目标项目时重建历史组件，旧请求通过原有 generation 隔离；回执仍校验目标项目、准备 ID、请求 ID 和来源类型。被更新请求替代或取消的读取不能重新打开旧提交面板。

## 源码与验证

- `src/ui/object-preview/ObjectImportDialog.tsx`：从已保存回执构建稳定引用，并按目标项目隔离历史组件。
- `src/ui/object-preview/ObjectImportHistoryPanel.tsx`：接收准备引用并在刷新后打开详情，复用现有正式导入面板。
- `src/ui/object-preview/object-import-history.ts`：分页刷新与指定准备读取衔接，保留严格解析和迟到响应隔离。
- `tests/object-import-handoff.test.ts`：覆盖两种准备不在第一页时直接打开、显式提交、重新查询及取消/替代请求。

本批完成边界执行：

```powershell
npm run typecheck
npx tsx --test tests/object-import-handoff.test.ts tests/object-import-history.test.ts tests/object-file-import.test.ts
npx prettier --check src/ui/object-preview/object-import-history.ts src/ui/object-preview/ObjectImportHistoryPanel.tsx src/ui/object-preview/ObjectImportDialog.tsx tests/object-import-handoff.test.ts
npm run check:effective-lines
git diff --check
```

类型检查通过；13 项测试通过，0 失败；定向 Prettier、有效行检查及 diff 检查通过。日志：

- [类型检查](../output/import-handoff-typecheck.log)
- [定向测试](../output/import-handoff-tests.log)
- [有效行检查](../output/import-handoff-lines.log)

测试使用模拟 API 回执验证前端状态与路由选择。本批未运行真实文件复制、浏览器交互或原生 WebView 验收；不把前端测试计为新一轮 Core/原生验收。既有 Core 提交及恢复实现未修改，不重复运行其全套验收。无数据库迁移、版本变更、提交或发布。

## 后续入口

F9 仍需完成其余生产命令与数据源、正反导航、多项目隔离、迁移及协议核对，并交付真实数据的原生开发版。F8 的旧拓扑反馈重新定位仍缺明确的用户确认工作流，本批没有把模型提示说明计为该能力完成。参见 [实施计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)。
