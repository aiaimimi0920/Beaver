# F9：新建对象到制造任务的衔接

日期：2026-09-26。状态：本次导航切片完成；F8、F9 总体仍未完成。

## 可用结果

生成对象确认成功后，点击“前往制造任务”可定位回执对应的对象、中修任务和制作迭代，并查看精修阶段。保留“打开对象”和制造页“返回此对象”入口。导航不自动入队、执行、调用模型或宣告生成验收成功。

两个入口均主动刷新任务快照和对象目录，即使订阅事件未送达，刚创建的任务仍能显示。沿用已有 Core 草稿提交、身份核验和本地回执恢复流程。不可用画面选择的提示已改为指向对象版本或尝试的场景预览。

## 改动与验证

改动 ObjectGenerateDialog.tsx、ObjectFrameworkWorkspace.tsx，以及适配必需导航回调的 object-generation.test.ts。

- `npm run typecheck`：退出码 0。[日志](../output/generation-handoff-typecheck.log)。
- `npx tsx --test tests/object-generation.test.ts`：7 项通过，0 失败，覆盖幂等、存储失败、错配回执和恢复。[日志](../output/generation-handoff-tests.log)。
- 三个改动文件的 `prettier --write` 和随后 `prettier --check`：通过。
- `npm run check:effective-lines`：1178 个源文件，17 个未改动历史文件，0 违规。[日志](../output/generation-handoff-lines.log)。
- `git diff --check`：通过。

浏览器使用真实生产工作区、生成弹窗和任务查询，配合独立模拟 Desktop 传输。订阅刻意不发送事件。实际点击完成：空目录 → 填写表单 → 确认创建 → 前往制造任务 → 显示中修与精修阶段 → 返回对象 → 页面重载 → 从恢复回执打开对象 → 再次前往制造任务。重载后的调用仅包含 object.list 和 objectTask.snapshot，没有重复 commit 或调度调用。见 [浏览器调用与任务内容](../output/generation-handoff-browser.log)、[制造页截图](../output/playwright/generation-handoff/manufacture.png) 和 [隔离夹具](../output/playwright/generation-handoff/fixture.tsx)。本轮浏览器和验证服务器均已关闭。

浏览器证据覆盖生产组件交互和事件缺失后的刷新，服务端由夹具模拟。本批没有运行原生 EXE、真实 Core 写入、模型制作或完整端到端验收。截图未载入完整产品样式，不作为视觉验收。

## 剩余工作

F9.1/F9.2 本次只覆盖生成对象后的正反导航、刷新与恢复。其他生产路径数据源、多项目隔离、断线边界、旧数据迁移、各调用入口规则和原生开发版交付仍未完成。F8.5 旧拓扑重新定位尚缺少用户审阅并持久化的明确映射，本批未处理。完整清单见 [实施计划](FRAMEWORK-IMPLEMENTATION-PLAN.md)。

