# F3.4 规划完成声明

日期：2026-09-25。范围：owner 对当前粗修或中修责任树确认需求已展开，保存到项目并在生产看板查看当前或失效状态。

## 可用流程

任务泳道的粗修和中修卡片提供“确认当前责任范围的规划”。owner 填写确认依据后保存，界面显示确认者、时间、依据和覆盖的任务 ID。粗修声明包含其全部责任后代，中修声明只包含自己的责任后代；依赖和其他独立任务不会被当作子任务。中修无需逐个先声明，粗修可以一次确认整个已展开范围。

Core 拒绝精修目标、已撤销目标、仍有待规划事项的活动范围，以及没有有效子任务的活动父任务。必要与可选分支均属于待规划范围。撤销记录保留在范围指纹中，声明不将撤销计为验收，也不替代修订目标或接受结果。

声明保存于项目 JSON 实体，含不可变 request ID 回执；重复同一请求返回原回执，不同内容复用 ID 被拒绝。最新声明供快照和看板读取，项目关闭重开后保留。写入同一事务内核对预期 plan revision 和最新责任范围内容指纹；声明本身不增加任务或计划 revision，不修改任务/run 状态，不唤醒执行调度器，仅发出查询刷新通知。

指纹覆盖排序后的任务定义、身份关联、位置、基准要求及撤销事实，忽略普通执行状态和执行 revision。定义修订、增减子任务、顺序或撤销变化使旧声明不适用于当前范围；内容完全恢复一致时，原声明再次适用。独立范围变化不影响已有声明，但新声明仍要求当前项目 plan revision。普通入队不使声明失效。

UI 在响应丢失后保留原请求，重试沿用原 ID、范围和依据，即使后台快照已改变。owner 可放弃原重试、刷新并重新检查范围。项目切换沿用面板 project key 的卸载边界。所有声明均明确提示：规划确认不表示任务验收、对象发布或整体目标完成，不输出伪精确完成率。

## 实现与验证

Core：object_task_planning_declaration.rs；Desktop：objectTask.declarePlanningComplete、业务通知和事务快照；共享类型：object-task-planning-declaration.ts；生产 UI：ObjectTaskPlanningDeclaration.tsx、ObjectTaskBoard.tsx 和 ObjectTasksPanel.tsx。

验证：生产看板、声明重试与工作区 UI 回归 15 项通过；Desktop 对象任务分发回归 24 项通过（含声明持久化、项目重开、幂等冲突、范围失效和规划缺口 3 项）；业务通知回归 1 项通过。相邻回归同步更新空快照的 planningStates 契约，以及由执行运行时承接的 attemptFile/attemptTrace 路由归属。日志分别为 output/f3-declaration-ui.log、output/f3-declaration-desktop-final.log 和 output/f3-declaration-effects.log。

TypeScript typecheck、Rustfmt、Prettier 和有效行数检查通过：1080 个源文件，17 个未改动历史文件，0 项违规。没有运行正式原生交互或发布验收；F3.4 的完整验收仍未关闭。
