# F3.3 稳定插入锚点

2026-09-25，本批完成生产草稿编辑器的首尾及相邻插入流程。

## 可用行为与边界

前后插入绑定任务 ID，在调用时从工作区最新计划中解析位置。即使按钮来自旧一次渲染，任务已重排或其他编辑已发生，插入仍跟随同一个任务，并保留当前对象、标题及其他计划内容。首尾插入及添加粗/中/精修也从最新计划计算位置，连续操作不会相互覆盖。

锚点已删除时，界面提示“插入位置对应的任务已不存在，请重新选择位置。”，保留当前计划和未保存状态。已提交锁定、提交中和解锁中继续由工作区拒绝修改。

插入是即时草稿编辑，没有待恢复的插入命令。插入结果以原有任务 ID、数组和连续 `position` 保存，继续使用现有 Desktop/Core 草稿 revision 协议；不新增锚点持久字段、API 或迁移。跨窗口保存冲突仍需显式处理，不自动合并远端计划。既有 Core 提交和快照排序实现不变。

## 实现

- `src/ui/object-tasks/object-task-draft-insertion.ts`：身份锚点解析、草稿任务创建和位置重编号。
- `src/ui/object-tasks/ObjectTaskDraftEditor.tsx`：所有插入入口消费最新计划。
- `src/ui/object-tasks/object-task-workspace.ts`：支持函数式编辑，并将缺失锚点错误显示在现有告警区。
- `tests/object-task-insertion.test.ts`：直接调用生产按钮回调，验证旧渲染后重排、删除锚点、连续插入及保存后新工作区恢复。持久化接口使用内存 API 测试替身，本批未执行真实原生交互。

## 验证

- `npx tsx --test tests/object-task-insertion.test.ts tests/object-task-workspace.test.ts tests/object-task-draft-lock.test.ts tests/object-task-rebase.test.ts tests/object-task-baseline.test.ts tests/object-task-cancellation-ui.test.ts`：26 项通过，日志 `output/f3-insertion-tests.log`。
- 首次类型检查发现测试数组索引可能为 undefined；补齐测试断言/类型后，`npm run typecheck` 通过，新增插入测试复验 4 项通过，日志 `output/f3-insertion-typecheck.log`、`output/f3-insertion-tests-final.log`。
- 本批 4 个 TS/TSX 文件的 Prettier 检查通过，日志 `output/f3-insertion-format.log`。
- `npm run check:effective-lines`：1047 sources、17 unchanged legacy、0 violations，日志 `output/f3-insertion-structure.log`。
- 独立只读复核未发现可执行问题；差异空白及 UTF-8 无 BOM 检查通过。

F3.3 的稳定插入锚点范围完成；真实 Codex 标题建议及正式交互验收仍未完成，计划项保持未勾选。本批未生成或发布新原生程序。
