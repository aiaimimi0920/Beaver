# F5.4 历史提示词载入

日期：2026-09-25。状态：定向开发交付；F5.4 整项仍未完成。

## 可用流程

生产对象任务树中，打开尚未开始的精修任务的定义修订窗口，展开“载入历史提示词”。选择同一对象的其他历史迭代，再选择一次冻结尝试，预览提示词与验收要求，点击“载入并替换当前提示词与验收要求”。内容进入本地编辑，保留目标任务的标题、依赖、身份及用户填写的修订原因。

继续编辑后，经“核对修订”和“确认修订”保存。来源 run、fine、attempt 和冻结定义 revision 以明确来源说明附加到持久化修订原因中；用户原因仍必填，合并后的文本继续遵守现有字节上限。来源说明表示内容的载入来源，载入后允许继续修改，不声明最终内容与来源逐字相同。

只读取已有 objectTask.attempts，再使用已有 objectTask.revisePlanned 保存。载入和查询不会启动任务、改写旧尝试、切换基准、接受阶段或发布对象。目标必须满足现有 planned 修订资格；服务端继续使用 task/plan revision CAS 及 run 状态校验。运行中或已取消的源迭代可提供冻结定义，目标始终必须是尚未开始的精修。

## 实现范围

- src/ui/object-tasks/object-task-prompt-history.ts：限定同项目、同对象的其他历史迭代；校验响应项目、对象、run、medium/fine 归属和重复 attempt；切换查询或关闭时丢弃迟到响应。
- src/ui/object-tasks/ObjectTaskPromptHistory.tsx：来源选择、冻结内容预览、空结果、错误重读及明确替换按钮。
- src/ui/object-tasks/object-task-revision.ts：仅编辑阶段允许载入；保留目标标题和依赖；来源随现有修订请求冻结，复用确认、精确重试、冲突重读及保存流程。
- src/ui/object-tasks/ObjectTaskRevisionDialog.tsx：接入真实精修编辑窗口，核对时显示来源；只读任务不提供载入操作。
- tests/object-task-prompt-history.test.ts：新增 4 项行为回归。

不新增 Core、Desktop API 或数据库结构；沿用已接通的冻结定义查询与原子修订协议。本批没有迁移。

## 定向验证

1. npx tsx --test tests/object-task-prompt-history.test.ts tests/object-task-revision*.test.ts：既有修订 28 项通过。新测试初次出现正则字面量转义错误，随后运行发现测试夹具未同步 medium/run 状态；两处均已修正，未改动生产逻辑来适应测试。
2. npx tsx --test tests/object-task-prompt-history.test.ts：最终新增 4 项全部通过，覆盖本地载入到明确提交、空验收覆盖、来源保存、标题依赖保留、历史不变、错误身份和重复回包、迟到查询、关闭、已启动目标及必填原因。日志：output/f5-prompt-tests-final.log。既有 28 项证据：output/f5-prompt-tests.log。
3. npm run typecheck：通过；最终日志 output/f5-prompt-types-final.log。
4. npx prettier --check：上述 5 个 TS/TSX 文件通过；日志 output/f5-prompt-prettier.log。
5. npm run check:effective-lines：通过，971 sources，17 unchanged legacy，0 violations；报告 output/effective-code-lines.json，日志 output/f5-prompt-structure.log。未修改 baseline。
6. git diff --check：通过；日志 output/f5-prompt-diff-check.log。
7. 独立只读审查：未发现明确缺陷；范围为本次控制器、界面、测试及直接依赖。

修订控制器 323 effective lines，保持单一修订会话责任；历史查询控制器 107 行，修订窗口 216 行，新测试 194 行。新模块与源文件均保持 UTF-8 无 BOM。

## 剩余边界

本批支持将历史内容载入同对象新迭代的未启动精修，不能修改已运行 attempt 的冻结输入。当前轮次内的阶段重做、下游失效与恢复、门槛批准及后继 fine 推进尚待实现，F5.4 保持未勾选。历史来源目前记入修订原因，没有新增结构化来源字段或执行授权。

未运行浏览器交互或完整原生验收，未生成 EXE、调用真实模型、发布、提交或推送。下一步仍需实现持久阶段门槛与安全的后继精修推进，并补齐受管引擎会话恢复；完整开发计划尚未完成。
