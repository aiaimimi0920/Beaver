# F9.2 原发布与中止请求恢复

日期：2026-09-26。完成范围：人工确认后的发布请求及中止意图跨页面重开恢复，接续[未提交表单草稿](FRAMEWORK-F9-PUBLICATION-APPROVAL-DRAFT.md)。

## 可使用的结果

用户确认发布后，生产会话先保存完整原请求，再调用 Desktop 发布 API。请求 ID、审阅身份、尝试、预览摘要、人工确认和反馈处置保持不变；响应丢失或页面重开后不会生成新请求替代它。中止操作同样先保存原发布请求和中止方向，重开后即使服务端仍显示 applying，也只会重试中止。

重开只查询服务端记录，不自动提交。恢复后的重试按钮在查询协调完成前禁用；用户明确点击后才发送原请求。确认完成的 published/aborted 回执会清除本地待处理意图。清除失败时保留恢复记录，阻止继续发送，并提示刷新以重新核对回执和保存完成状态。

存储损坏、来源身份错误或读取失败时保留原始记录并阻止发布与中止，生产面板提供重新读取入口。写入失败保留内存中的原请求，阻止 API 调用，可通过原请求重试再次保存。发现多个未完成发布、其他活动发布或同 ID 不同内容的回执时停止协调，不覆盖本机请求。

本地键按项目、任务、轮次、对象隔离，允许从同轮次另一审阅或尝试重新打开原请求。完整原审阅和尝试仍保存在不可变请求内，不将目标改到当前预览。未提交表单的文件归属和替换确认仍不跨重开恢复；只有已经明确提交的原请求保留当时确认，以支持幂等重试。

## 实现和验证

- [恢复存储](../src/ui/object-tasks/object-publication-intent.ts)：版本化记录、来源核验及保存。
- [生产会话](../src/ui/object-tasks/object-publication.ts)：提交前保存、只读协调、原请求重试、中止方向及完成清除。
- [生产面板](../src/ui/object-tasks/ObjectPublicationPanel.tsx)：恢复提示、存储错误、读取重试和提交门禁。
- [新增回归](../tests/object-publication-intent.test.ts)：重建会话恢复、提交前无 journal、丢失中止响应、存储故障、完成清除失败、损坏/异源记录、范围隔离、冲突回执及服务端 aborting 响应。

日志位于 [output/f9-publication-intent](../output/f9-publication-intent/)。定向命令：

```powershell
rtk proxy npx tsx --test tests/object-publication-intent.test.ts tests/object-publication.test.ts tests/object-publication-approval-draft.test.ts
rtk proxy npm run typecheck
rtk npm run check:effective-lines
```

新增 6 项及相邻 10 项测试共 16 项通过；TypeScript 类型检查通过。相关文件官方 Prettier 和定向差异检查通过；有效行检查为 1197 个源文件、17 个未改变历史文件、0 项违规，未更改 baseline。6 个源文件/测试/文档均为 UTF-8 无 BOM，两个文档的本地链接均存在。测试使用模拟 API 和注入存储，重建真实生产会话并验证生产组件静态呈现；本批未执行浏览器、原生 EXE、真实引擎或外部模型验收。Core 和 Desktop API 契约未改动。

## 剩余边界

此批关闭原发布请求跨页面重开的本地恢复缺口，F9.2/F9 整体保持未勾选。localStorage 仅属于当前浏览器/WebView 数据目录，不提供跨设备迁移、跨窗口写冲突协调或记录清理策略。存储损坏不自动删除或重置；需修复原记录后重读。服务端冲突保留现场供核对，不自动撤销其他发布。

最终发布重新定位门禁、已发布版本与独立 PNG 对应，以及 F9 的其余集成和原生交付条件仍待完成。未更改版本，未提交或推送，未声明全部开发计划完成。
