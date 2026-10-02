# Beaver 当前开发状态与手工测试入口

## 是否开发完毕

**尚未完成全部 Beaver 开发计划，但可以开始手工测试已接通的工作流。**
当前已交付固定的原生开发包 `0.1.19.43`，适合开发阶段的人工试用和问题
反馈。最新源码已编入此包，并在真实 WebView2 中读取了保留轮次的候选审阅。
完整计划、最终人工接受/发布和正式交付条件仍需逐项收口，不能用定向检查
或一次启动恢复来替代。

计划中未勾选的条目包含两类：部分能力已有实现但缺少集成或验收证据；部分
能力仍有明确实现缺口。因此不把所有未勾选项都当作完全没有实现，也不估算
缺少依据的完成百分比。当前明确待推进的边界包括：

- 最终候选逐区重新确认、独立冻结 PNG 来源和显式发布门禁已有源码及定向
  证明，并已编入 `.43`。当前真实候选尚无最终 output 编号帧；需要用户在
  应用内保存该帧、核对区域并明确批准，才可继续发布。完整三维交互等其余
  F8.5 条件仍未完成，不再把已实现的最终确认门禁列为“待实现”。
- 更完整的可配置阶段/报告策略、受管会话取消及资源恢复闭环。
- F9 中尚未逐项完成的生产入口、导航、迁移历史语义和跨入口一致性核对。
- 更完整的原生 WebView/真实引擎最终发布、受管 Blender、正式 release 和
  最终交付验收。本轮原生只读证明不覆盖这些条件。

完整状态以 [开发计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) 及各增量证据为准。

## 如何开始手工测试

本轮使用固定包，不再推荐会被下一次构建覆盖的 `target/release/Beaver.exe`。
两个入口均从 `delivery-proof.json` 读取包路径，并在启动前核对产品版本与
SHA-256；任意 Beaver 进程存在时拒绝启动，不自动停止进程。

- **继续现有候选**：本轮 `.43` 已恢复原数据并保留运行，可直接在现有窗口
  手测。以后通过托盘“退出”结束宿主，再双击
  [Resume-Preserved-Round.cmd](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/Resume-Preserved-Round.cmd)，
  恢复同一轮 `data` / `webview`。入口仅读取原 token 文件到子进程环境，不
  复制或打印凭据，不自动创建任务、确认或发布。
- **独立小对象试用**：先通过托盘“退出”结束现有宿主，再双击
  [Start-Isolated-Test.cmd](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/Start-Isolated-Test.cmd)。
  数据分别放在本轮证据目录的 `manual-test/data` 和 `manual-test/webview`，
  不使用或复制现有数据/凭据。重复运行保留此独立测试数据。

**关闭主窗口只是隐藏到托盘，不会退出。** 若启动器提示已经运行，应使用
托盘“退出”，不要反复启动第二份宿主。共同脚本见
[Start-Development-Test.ps1](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/Start-Development-Test.ps1)。
历史 `output/manual-test/Start-Manual-Test.*` 仍保留作旧证据和备用，不是固定
`.43` 入口。

使用独立入口首次启动后，在界面中创建一个全新的 blank 项目，放到专门的测试目录。
独立数据目录可能需要重新配置 Codex、Godot、Blender 和提供方；启动器不会
从现有数据目录复制凭据。模型生成需要实际可用的模型配置，预览需要对应
工具可用。不要把缺少这些外部依赖当作已完成的生成或预览测试。

启动器只准备隔离的手工测试环境，不执行生成、不创建项目、不代表原生
验收通过。正式自动化原生验收继续使用项目规定的
`scripts/start-fresh-native-test.ps1`。

## 继续当前蓝色斜撑候选

此路径使用已保留的轮次，不重新生成木箱，也不另开一次整体验收。打开原
`Fresh game` 项目，在“制造”中选择“按编号原图将木箱正面斜撑改成蓝色 ·
待验收”，进入“查看执行记录与恢复”，展开已有整体候选审阅记录
`numbered-feedback-review-20261001`。

1. 在“最终候选逐区重新确认”中查看原发布版本编号帧、1 号区域和原意见。
   原图已在 `.43` 的真实 WebView2 中成功读取，像素尺寸为 `960 × 540`。
2. 打开**此候选尝试的 output 场景预览**，不要使用旧接受版本或实时工作区
   来代替最终 output。冻结画面、框选蓝色斜撑对应区域并保存编号帧。
3. 返回候选发布面板，点“刷新编号存档”，选择准确的最终输出编号帧，逐区
   指定原反馈对应位置；若确实无对应位置，填写原因，不复用旧图坐标。
4. 自行核对两张原 PNG、区域及文件归属，再填写反馈处置、处置说明和最终
   接受说明；只有确实接受成果时，重新勾选逐区确认并显式“接受并发布对象”。
   不想接受可保持待审，无需为了测试强行发布。

本轮停点是步骤 1：最终输出选择器只有“不附加存档帧”，逐区确认未勾选且
禁用，“接受并发布对象”也禁用。这是缺少最终输出证据时的正确阻断，不是
已发布或已验收。此次没有代用户冻结新帧、填写决定、批准、调用模型或发布。

已观察到执行记录上方仍有“最终人工接受与版本发布尚未开放”的过期文案，
但下方实际发布面板已开放并受新门禁保护；保留为已知问题，本轮未为文案
追加源码修改和重构建。历史冻结报告也不应被后来成功改写。

## 建议第一轮测什么

1. **启动、项目和对象**：创建 blank 项目，创建一个简单对象；切换页面后
   确认同一对象和任务仍在，通过托盘退出后重开，检查保存内容。
2. **需求与规划**：输入一个小需求，例如“做一个可旋转的低面数木箱”。
   检查规划、人工修改、提交和制造跳转，确认没有自动重复提交。
3. **一次制作与候选审阅**：执行一个小任务，查看真实执行状态、冻结输出
   和报告。遇到错误先记录，再使用应用里的重试、刷新或中断入口。
4. **预览与编号反馈**：检查画面确实属于当前对象/尝试，保存一条编号意见，
   重开后核对原图、编号和文字仍一致；不要把旧图坐标视为新成果的已确认位置。
5. **接受与发布**：核对文件、接受说明和反馈处置，显式发布；检查接受版本、
   任务状态和队列更新。退出重开后应能读取原发布记录。
6. **恢复**：保留一个未提交表单草稿，正常退出并重开，核对恢复结果。出现
   未完成发布时先刷新状态，再明确选择重试或中止；重开本身不应自动发布。

先用一个小对象走通整条流程，再增加多项目、导入、历史版本和复杂模型。
无需等全部计划收口才开始反馈真实使用问题。第一轮重点是流程可用性、状态
一致性和恢复行为，不要求一次生成就达到最终美术质量。

反馈问题时提供：页面与操作步骤、预期和实际结果、项目路径、任务或请求 ID、
截图及发生时间。保留测试目录，便于核对日志和历史；不要贴出 token 或密码。

## 当前 `.43` 构建与恢复证据

本轮证据目录为
[beaver-native-delivery-20261001](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/)。
当地记录日为 2026-10-01，部分 UTC 日志为 2026-10-02。

- 固定包：[Beaver-native-0.1.19.43-win32-x64](../release/Beaver-native-0.1.19.43-win32-x64/)。
- 可执行文件：[Beaver.exe](../release/Beaver-native-0.1.19.43-win32-x64/Beaver.exe)。
- 产品版本：`0.1.19.43`，development；公共 SemVer 仍为 `0.1.19`，只把
  `beaverBuild` 从 42 增至 43，没有提交或推送。
- EXE 大小：`23,699,456` 字节。
- EXE SHA-256：`3FFA672849EDBECC2AEF01EB29309897A41B861361373C4A33290878C1D437E1`。
- 包校验：611 个文件，`27,340,104` 字节；没有向不可变包追加验证产物。

实际通过 `npm run build:native`，Cargo release 编译耗时 **6 分 49 秒**。
有效行门禁为 **1229 sources、17 unchanged legacy files、0 violations**。
随后单独打包，没有重复构建；产品版本/原生包定向测试 **6 passed、0 failed**，
`scripts/verify-native-release.ts` 通过。既有编译 warnings 保留，不扩展清理。
相关日志为本轮目录中的 `build-0.1.19.43.log`、`package-0.1.19.43.log`、
`delivery-focused.log` 和 `package-verification.log`。

升级前只读确认全部登记项目无 active 普通任务或 running/queued 对象 run。
因关闭主窗口仅隐藏到托盘，按准确 PID/path 停止旧 `.42` 宿主；不声称这是
正常退出，没有按名称终止 Godot、Blender 或其他程序。旧 EXE 回滚副本与
SHA 留在本轮目录 `rollback-0.1.19.42`，原数据与历史证据未删除。

`.43` 从原 data/WebView 目录恢复后，升级前后选定业务快照严格相等：**2 个
项目、7 个 completed 普通任务及既有 accepted 状态、木箱 2 个对象 run、原
对象版本/发布和 27 个受保护文件 SHA 均保留**。候选仍为
`awaitingAcceptance` / `awaitingGate`，候选 publications 为 0。只读 UI 导航
后再次比对通过，见 `delivery-proof.json`、`before-state.json`、
`after-state.json`、`after-state-verified.log` 和 `after-ui-state.log`。

真实生产 WebView 证明见
[native-ui-proof.json](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/native-ui-proof.json)、
[原编号 PNG](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/native-original-png.png)、
[缺少最终编号帧](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/native-missing-final-frame.png)及
[发布阻断截图](../../AI/GameEditor/linshi/beaver-native-delivery-20261001/native-publication-gate.png)。
附着期间 console 为 0 errors / 0 warnings；完成后已 detach，保留宿主和待审
界面运行。此结果只证明恢复、原图读取及缺少最终证据时的阻断，不证明真正
最终帧授权、实际发布或完整原生验收。

两个新入口通过 PowerShell AST、文本编码检查，并在现有宿主运行时实际
拒绝启动，未产生第二实例；独立入口的新数据首次启动分支未另行执行。前轮
Core/Desktop/前端及 Chromium 定向证明继续复用，升级前核对其 19 条文件
hash 一致，不把历史检查写成本轮重跑。

初次恢复比较脚本曾把内存中的 `undefined` 与 JSON 省略字段比较，导致
7 处 absent/undefined 差异；已在 JSON 序列化后比较，仍保留 null/absent
区别。原失败日志保留，修正后验证通过，未修改业务数据。只读 CLI 脚本
首次有表达式末尾分号解析错误，移除后通过，也保留原失败日志。

包内 `RELEASE.json` 的 `runtimeVerified: false`、`migrationComplete: false`
保持原样。当前交付不是 signed 正式 release，也不是整体验收通过；F8.5、
F8、F9 及 F9.5 总项均不因此勾选。

## 历史 `.36` 构建记录

2026-09-27 从当时工作区运行 `npm run build:native` 成功，release 编译耗时
8 分 37 秒，有效行门禁为 1199 sources、17 unchanged legacy files、0 violations。
日志见 [历史原生构建日志](../output/manual-test-build.log)。当时 EXE 为
`0.1.19.36` development，修改时间 `2026-09-27T05:59:43.4124355Z`，大小
`23,596,032` 字节，SHA-256 为
`2D364F3D1627E4A05ACA6B643763497879EBD5227743F539C5979181010056DF`。
当时没有发布包或启动 EXE，旧启动器仅经过语法检查，复用了 21 项前端定向
回归和 Chromium 双标签页证明。这些记录保持历史含义，动态 target 路径
已被后续构建替换，不代表当前可执行文件或本轮实际结果。
