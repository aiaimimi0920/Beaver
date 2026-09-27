# F8 发布前后续中修反馈

本轮完成候选审阅中的“排入后续中修”流程：填写反馈、后续标题及验收标准，保存后在发布预览确认处置；发布事务同时创建固定到新版本的 planned 中任务、细任务、运行记录和回执。任务不会自动入队。原有本轮返工路径保留。

## 持久化与恢复

反馈绑定原审阅和尝试，进入发布摘要；保存新反馈后，旧发布确认失效。发布只接受与反馈去向匹配的处置。反馈保存与发布使用同一锁顺序，发布未完成时不能追加反馈。最终事务失败不会留下后续任务；相同请求重试复用回执，重新打开存储仍可查询。

可选 PNG 从冻结 blob 校验 hash、解码及实际尺寸。发布记录保留准确区域；后续任务包含原始尝试、图片来源及编号区域意见，并明确要求执行前重新核对位置，不能把历史坐标视为新版本的可靠定位。

生产界面在发送前保存完整请求及 requestId。响应丢失后，页面重载恢复原请求，由用户点击重试；保存失败或损坏记录阻止新提交，保留原始数据。未提交的编辑字段仍只保存在 React 状态中，本轮未实现此入口的完整草稿恢复。修复损坏存储后需要重载页面。

## 验证

- Core：`cargo test --locked -p beaver-core object_publication -- --nocapture`，15 项通过，包含事务失败、精确重试、过期摘要、去向校验、重开及真实 PNG 校验。
- Desktop：`cargo test --locked -p beaver-desktop publication_routes_accept_and_retain_history_without_redispatch -- --nocapture`，1 项通过，覆盖保存、重试、发布及后续回执查询。
- 前端：deferred、publication、followup、candidate-rework 四个测试文件，17 项通过；`npm run typecheck` 通过。
- 修改的 Rust 文件 rustfmt 检查与 TS/TSX 文件 Prettier 检查通过；`git diff --check` 通过。
- `npm run check:effective-lines`：1116 个源文件，17 个未改变的历史文件，0 项违规；未修改基线。
- Chromium 使用生产 `ObjectCandidateReworkPanel`、`ObjectPublicationPanel` 和真实前端状态类，API 为模拟边界。保存后模拟丢失响应，重载恢复同一请求，再确认处置并发布，显示固定版本的后续任务回执；断言保存仅一次且本地待确认请求清除。浏览器与临时服务已关闭。

日志位于 `output/f8-deferred-*.log`；浏览器脚本为 [smoke.js](../output/playwright/f8-deferred/smoke.js)，结果为 [published.png](../output/playwright/f8-deferred/published.png)。首次浏览器尝试修正了测试夹具的必需审阅规则及控件定位，最终 `browser-final.log` 通过；未因测试夹具问题修改生产实现。

## 剩余边界

本轮完成发布前反馈分流子流程。真实 Godot/Blender 选择、跨版本拓扑映射、未提交草稿恢复及 F8 整体目标仍待完成。浏览器证明覆盖生产组件与模拟 API；本轮没有运行 Beaver.exe、真实引擎或外部发布验收，F8.1 至 F8.5 保持未勾选。
