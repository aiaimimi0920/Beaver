# F9.2 最终发布表单草稿恢复

日期：2026-09-26。范围：最终接受与发布表单中尚未提交的接受说明、反馈处置及处置说明。

## 可使用的结果

生产发布表单现在按项目、完整审阅身份及发布预览摘要保存本机草稿。用户填写到一半后刷新差异或重开页面，可以继续编辑；恢复不会自动发布。不同项目、尝试、审阅或预览摘要使用独立记录，旧记录保留，不将旧候选的处置带入新候选。

文件归属确认和替换接受版本确认只保存在当前表单状态中，整页重开后必须重新勾选。存储写入失败时保留当前输入、显示错误并阻止表单提交，可重试保存；同一发布会话刷新差异时继续保留尚未写入的输入。损坏记录、来源不一致或反馈分流不一致时阻止覆盖与提交，保留原记录并提供重新读取入口。

提交前再次保存草稿，成功后调用既有 `ObjectPublication.publish`。发布请求契约、Core 接受规则、Desktop 路由及不可变发布记录未变化。

## 实现范围

- [草稿存储](../src/ui/object-tasks/object-publication-approval-draft.ts)：持久化、身份隔离、恢复校验与错误状态。
- [发布会话](../src/ui/object-tasks/object-publication.ts)：按摘要持有草稿实例，刷新期间保留未保存输入；复用现有发布和重试流程。
- [生产发布面板](../src/ui/object-tasks/ObjectPublicationPanel.tsx)：绑定草稿、提示恢复状态、读写重试及提交前保存。
- [新增回归](../tests/object-publication-approval-draft.test.ts)：恢复、身份隔离、损坏保护、失败恢复及生产组件呈现。

沿用已有浏览器存储接口，无数据库迁移，不修改公共 API，不增版本，不提交或推送。

## 验证

日志目录：[output/f9-publication-approval-draft](../output/f9-publication-approval-draft/)。

1. `rtk proxy npx tsx --test tests/object-publication-approval-draft.test.ts tests/object-publication.test.ts`：10 项通过，其中新增 4 项。包含原发布响应丢失重试、取消迟到响应及已发布记录读取的相邻回归。
2. `rtk proxy npm run typecheck`：通过。
3. 对本批 4 个源文件/测试文件运行官方 Prettier：通过。
4. `rtk npm run check:effective-lines`：1195 个源文件，17 个未改变的历史超限文件，0 项违规；未修改 baseline。
5. Chromium 加载生产 `ObjectPublicationPanel` 与 `ObjectPublication`，使用独立模拟 API 和浏览器存储：整页重载恢复、确认不恢复、写入失败阻断、刷新保留输入、保存重试、摘要隔离、损坏记录阻断、读取恢复及最终请求字段校验通过。第一次定位选择框、第二次重载后定位文本框因 `getByLabel` 精确匹配超时，改用实际可访问角色定位后整条流程通过，未因此修改生产代码。

浏览器夹具和截图在 [output/playwright/f9-publication-draft](../output/playwright/f9-publication-draft/)，已查看 [恢复截图](../output/playwright/f9-publication-draft/restored.png)。命名浏览器会话及本轮本地服务已关闭。

## 边界与后续

本批完成 F9.2 的未提交发布表单草稿恢复子流程，F9.2/F9 整体保持未勾选。浏览器验证使用模拟发布响应，没有执行真实对象发布、原生 EXE、真实引擎或外部模型验收。Core 未变化，本批没有重复其既有功能测试。

草稿保存在当前浏览器或 WebView 的 localStorage；不随项目导出、设备迁移或新的 WebView 数据目录迁移。不恢复已经勾选的人工确认，也不把草稿视作服务端发布回执。原发布请求的跨整页重载本地保存、跨窗口并发编辑冲突及草稿清理策略未在本批实现；已有服务端 applying/aborting 发布记录仍由既有查询与原请求重试路径处理。

下一步仍需完成 F8.5 的已发布版本/独立 PNG 对应及最终候选重新定位门禁，并继续核对 F9 其余集成与原生交付条件。不能据本批草稿恢复声明框架计划完成。
