# F8 已发布版本的后续文字任务

日期：2026-09-25。范围：已发布记录中的文字反馈转为可继续安排执行的中任务与细任务。

## 可用流程

用户在生产发布记录中填写标题、改进要求和验收标准，创建后续任务。新任务保持 planned，由任务列表继续确认及入队；创建操作不自动执行。中任务固定到用户正在查看的发布版本，依赖原中任务；细任务记录 feedback 阶段。即使对象已有更新的接受版本，后续任务实际领取时仍使用原发布版本。

Core 在同一事务中核验已发布状态和版本，创建中任务、细任务及运行记录，增加计划 revision，并保存请求回执。相同 requestId 和相同内容重试返回原回执；更改内容后复用 requestId 被拒绝。发布记录、原任务和队列不被此创建操作改写。

UI 在响应丢失时保留输入和冻结请求，支持重试原请求；重查发现已保存回执后恢复成功状态。关闭面板隔离迟到响应，重新打开读取已保存记录。已提交回执可在项目重开后读取。未提交草稿及未确认请求目前只存在前端会话内，尚未持久化到磁盘。

入口：`objectTask.createPublicationFollowup` 和只读 `objectTask.publicationFollowups`；生产组件为 `ObjectPublicationFollowupPanel`，嵌入 `ObjectPublicationPanel` 的已发布记录。

## 定向验证

- Core：`cargo test --locked -p beaver-core object_publication_followup`，2 项通过。覆盖原子写入、幂等与冲突、发布状态和版本拒绝、队列不变、较新接受版本下的固定基线领取、释放 host lock 后重开恢复。
- Desktop：`object_publication_runtime_tests`，1 项通过；实际调度发布后经 Desktop 路由创建、重试和列出后续任务。
- 前端：`tests/object-publication-followup.test.ts` 和 `tests/object-publication.test.ts`，9 项通过；覆盖响应丢失重试、关闭后重查、错误来源回执、草稿和历史呈现。
- TypeScript 类型检查、Prettier 检查、修改 Rust 文件的 rustfmt 检查和 `git diff --check` 通过。
- `npm run check:effective-lines`：1110 个源文件，17 个未修改历史文件，0 项违规；未放宽基线。
- Chromium 加载生产发布组件，模拟 API 在提交成功后丢失首次响应；完成填写、失败后输入保留、完全相同请求重试、成功清空输入、重开只读恢复历史。截图见 [浏览器证据](../output/playwright/f8-followup/followup.png)，脚本位于 `output/playwright/f8-followup/smoke.js`。该夹具未加载完整应用样式，只作为交互证据。

日志位于 `output/f8-followup-*.log`。首次 Core 测试在未释放原 runtime 时打开第二个项目 host，触发 Windows host lock；测试改为先释放 runtime 后重开，重新运行 2 项通过。浏览器夹具首次启动的 Windows 路径分隔符比较已修正；随后交互脚本通过，控制台仅余夹具 favicon 404。测试浏览器及夹具服务器均已关闭。

## 尚未关闭的边界

本切片仅接收文字要求，明确不复用旧 PNG 坐标。发布前反馈加入当前轮次或后续中修的选择、图片反馈转后续任务、旧坐标重新定位、未提交草稿的持久恢复，以及真实 Godot/Blender 预览与选择均待完成。F8.1-F8.5 保持未勾选。

未运行原生 EXE/WebView 全流程、模型调用、Godot/Blender 或正式发布验收；Chromium 的 API 为模拟边界，Core 与 Desktop 使用各自定向测试证明真实存储和路由。此次结果不表示全部开发计划完成。
