# F8 后续任务草稿恢复

日期：2026-09-25。接续 [已发布版本后续任务](FRAMEWORK-F8-PUBLICATION-FOLLOWUP.md)，补齐该表单在前端实例重建后的文字草稿和未确认请求恢复。

## 已实现

生产后续任务表单在编辑时写入版本化 localStorage 记录，按项目、发布请求和发布版本隔离。初始化恢复原文字；创建前先持久保存完整请求及 requestId，再调用 Desktop API。请求响应丢失或页面关闭后，重新进入可读取同一个请求，保持内容锁定并重试；若服务端已完成，正常只读重查识别回执，清理草稿与 pending request 并刷新任务列表。

恢复时严格检查记录结构、项目和发布版本，以及草稿与未确认请求的一致性。损坏或来源错误的记录保留原值，禁止生成新的创建请求，界面提供恢复存储后的重新读取入口。存储写入失败显示错误，且创建前持久化失败不会调用修改 API。

实现位置：`src/ui/object-tasks/object-publication-followup-draft.ts` 负责版本化存储与归属校验，`object-publication-followup.ts` 负责先保存后提交及回执协调，`ObjectPublicationFollowupPanel.tsx` 显示恢复状态和错误。沿用现有 UI 本地草稿模式；服务端回执和幂等仍由现有 Core 负责。

## 验证

- 前端后续任务及发布相邻回归共 12 项通过。新增 3 项测试覆盖独立实例恢复、请求先持久化后调用、服务端成功后只读恢复、跨版本隔离、原请求重试、存储故障及损坏记录保留。严格类型检查发现测试中的数组首项可能为空，补充断言后，后续任务 6 项重新通过。
- TypeScript、Prettier、`git diff --check` 通过；有效行检查为 1111 个源文件、17 个未修改历史文件、0 项违规，未修改基线。
- Chromium 使用生产组件和真实 localStorage：填写后完整页面 reload 恢复三项文字；模拟 API 提交成功后丢失响应，再次完整 reload 通过只读查询恢复已创建记录，累计只有一次创建调用，持久 pending request 已清空。截图见 [恢复截图](../output/playwright/f8-followup-draft/recovered.png)。
- 运行记录：`output/f8-followup-draft-*.log`；浏览器夹具和断言脚本位于 `output/playwright/f8-followup-draft/`。本次创建的浏览器及服务器均已关闭。

## 边界

localStorage 随同一浏览器 origin / WebView 数据目录保存，不写入项目 SQLite，也不跨设备或多个客户端同步。清理或更换 WebView 数据目录会失去本地未提交草稿。测试覆盖页面重载和独立 session 实例，未执行原生 EXE 退出重启验收。浏览器 API 为模拟；本轮未修改 Core/Desktop，沿用上一切片已通过的真实存储、幂等和重开证据。

本增量关闭已发布版本文字后续任务的本地草稿恢复子流程。其他反馈入口的草稿恢复、发布前当前/后续分流、图片后续反馈、旧坐标重新定位及真实 Godot/Blender 预览仍待完成，F8.1-F8.5 保持未勾选。
