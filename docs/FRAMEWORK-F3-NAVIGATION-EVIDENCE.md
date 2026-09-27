# F3.4 对象与制造导航证据

日期：2026-09-25。范围：从持久化对象迭代进入制造阶段和执行记录，再返回关联对象。

## 可用结果

选择目录对象后展示其真实中修迭代；选择迭代进入制造页，保留同一 object、task 和 run 身份。制造页展示责任粗修、精修阶段、提示词、验收要求、依赖和持久化状态。执行入口复用现有生产执行/恢复对话框；返回对象时清空目录筛选，显示关联对象。

App 使用 project ID 作为工作区 key，切换项目会清空选择和弹窗。共享查询取消旧请求并订阅更新；读取中、失败、空对象和过期选择有明确提示，不回退到其他对象任务。执行操作后刷新任务和对象目录。共享状态 schema、标签及预览映射补齐 accepted，使发布后的快照继续可读。

实现位于 ObjectFrameworkWorkspace、ObjectProductionTasks、App 及共享查询/状态模块。复用 objectTask.snapshot 和现有执行 API，本轮未增加 Core 存储或 Desktop 端点。

## 验证

- 导航、执行查询/界面、预览映射、项目查询定向测试 20 通过、0 失败，日志 output/f3-navigation-tests.log。
- 随后的导航/framework 组合检查中，导航 4 项通过，framework 因 Node 加载 CSS 失败，保留日志 output/f3-navigation-final-tests.log。将样式 import 移至 App 后，framework 4 项复验通过，日志 output/f3-navigation-framework-tests.log。
- npm run typecheck 通过，日志 output/f3-navigation-typecheck.log。
- npm run check:effective-lines 通过：1038 sources、17 unchanged legacy files、0 violations，日志 output/f3-navigation-structure.log。保留 validation/service.rs 已有 unused-field 警告。
- 10 个本轮源码、样式及测试文件执行 Prettier --write，全部 unchanged，日志 output/f3-navigation-format.log。

测试覆盖生产任务组件的渲染、回调和执行 session；App/工作区的父层导航、project key 接线经过静态核查，未运行真实浏览器或原生窗口交互验收。本轮未修改 Rust 行为，未构建或发布新原生程序。

## 剩余范围

本轮关闭生产对象选择和制造跳转缺口。F3.4 保持未勾选：完整全层级泳道、父子展开及依据必要工作项/阶段事实派生的进度仍待完成。受管引擎会话、规划、恢复和发布的其他边界继续按对应计划项推进。
