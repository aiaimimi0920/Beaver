# F5 冻结输入输出文件清单

日期：2026-09-25。范围：F5.3 输入输出查询的文件清单部分；F5 整项继续保持未完成。

## 可使用的流程

在生产任务树打开中修的“执行记录”，展开某次尝试的“查看冻结输入/输出文件清单”，查看相对文件路径、输入和输出 SHA-256，以及新增、修改、删除、未变状态。二进制文件也按内容摘要比较。每页最多 100 项，显示总数，可前后翻页。

运行中的输出为 `null`，显示尚未冻结；已经冻结的空输出为 `{}`，可以如实表示输入文件全部删除。中断回执只包含控制结果，不携带文件清单，因此确认中断后显示明确的待查询提示；点击“刷新文件清单”执行只读查询，不重复中断、不启动模型，也不丢弃已确认回执。

数据取自持久化 `Attempt.input` / `Attempt.output`，与冻结定义和经过身份校验的执行视图在同一 Store 锁内读取。查询不读取当前工作区。重试历史、取消保留、删除工作区并重开后的清单均保留原检查点；后来重新创建的同路径工作区不影响历史。

## 源码边界

- `native/core/src/object_attempt_view.rs`：查询用 `Checkpoints`；`list_with_details` 保持原有 lineage、重试和取消历史校验；控制调用仍使用 `list` 返回原 `View`。
- `native/core/src/scheduler_object_control.rs`：`Execution.checkpoints` 经现有 Desktop `objectTask.attempts` 路由序列化。持久化 Attempt、旧中断回执、重试回执均未改格式。
- `src/shared/object-attempts.ts`：校验文件摘要和输出保存状态的一致性。
- `src/ui/object-tasks/ObjectTaskCheckpointFiles.tsx`：独立文件对照展示、分页、React 文本转义；区分对象原型名称与真实文件键。
- `ObjectTaskExecutionDialog.tsx`、`object-task-execution.ts`：生产入口和中断回执后的显式查询；沿用项目、run、对象、父子任务与重复 ID 校验。

## 定向验证

全部 Cargo 命令在同一 PowerShell/MSVC 环境执行，保留 `LIB`；使用 `rtk proxy`，日志在 `output/f5-checkpoints-*.log`。

| 命令 | 结果 | 日志后缀 |
| --- | --- | --- |
| `cargo test --locked -p beaver-core object_attempt --lib` | 32 passed，1 ignored，0 failed | `core` |
| `cargo test --locked -p beaver-core retries_chain_checkpoints_and_keep_immutable_history_after_disposition --lib` | 1 passed | `retry` |
| `cargo test --locked -p beaver-core removal_preserves_home_blobs_other_workspaces_and_history --lib` | 1 passed | `removed` |
| `cargo check --locked -p beaver-desktop` | 通过，既有 warnings | `desktop` |
| `npx tsx --test tests/object-task-execution*.test.ts tests/object-task-resume-execution.test.ts` | 27 passed | `ui` |
| `npx tsx --test tests/object-task-execution-files.test.ts` | 测试索引类型修正后 5 passed | `files-final` |
| `npm run typecheck` | 通过 | `types` |
| `cargo fmt --all -- --check` | 通过 | `fmt` |
| Prettier 检查本次修改的 7 个 TS/TSX 文件 | 通过；索引修正后的测试单独重新格式化并检查 | `prettier-check`、`files-format` |
| `npm run check:effective-lines` | 968 sources，17 unchanged legacy，0 violations；未修改 baseline | `structure` |
| `git diff --check` | 通过 | `diff-check` |

结构报告为 `output/effective-code-lines.json`；最终格式化后的测试源文件 SHA-256 与报告一致。

类型检查首次指出新增测试的数组索引可能为 undefined，已修正并重跑类型及该测试。前端回归覆盖二进制摘要相等、增删改、空输出与未冻结输出、HTML 样式文件名转义、原型键文件名、分页数量、非法查询数据，以及中断后的显式查询和跨项目拒绝。Core 回归覆盖重试历史和删除工作区后重开，并创建新工作区文件证明不会污染历史清单。

## 未完成边界

本切片提供文件清单和内容摘要，不提供文件正文下载或逐行文本 diff。未进行浏览器视觉检查、原生 EXE 构建、完整原生验收、发布、提交或推送。独立只读审查未发现本切片具体缺陷。

F5.3 仍缺对象/run/attempt 感知的完整回调、能力发现和实际操作轨迹；F5.1 的持久化门槛与后继 fine 推进、F5.2 的自动验收提示词等仍未完成。文件已冻结不能替代检查报告、验收决定或对象发布，生产制造工作区的 Demo 数据尚未替换。
