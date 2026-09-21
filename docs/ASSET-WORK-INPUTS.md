# 源文件与依赖输入清单：第五批框架交付

日期：2026-09-14。本批已实现每次执行的显式输入文件清单、Beaver 保存的字节快照和 SHA-256、依赖版本复核、完成后重开子任务及只读输入展示。Beaver 现在能回答一次 attempt 声明使用了哪些文件、开始时保存了哪个版本，并在相关提交边界拒绝已经变化的依赖。

本文保留第五批输入证据及其恢复路径的历史记录，包括当时的剩余工作；后续完成状态与边界见 [通用框架契约](WORKFLOW-FRAMEWORK.md)和[开发计划](CODEX-WORKFLOW-DEVELOPMENT-PLAN.md)。第五批回调协议升级为版本 **4**，当前为版本 **5**，应用内部版本仍为 `0.1.19.1`。第五批没有重编译、启动或替换 Beaver.exe，也没有改动发布目录。

## 使用约定

先通过 `state` 读取真实 revision、当前阶段和 subtask ID。下面仅展示 `work` 请求的 `change` 字段，外层仍需要 `operation: "work"`、新 `requestId`、`expectedRevision`、`assetRevision` 和 `stageId`：

```json
{
  "action": "begin",
  "subtaskId": "从当前状态读取的实际 ID",
  "inputs": { "design": "用户确认的设计输入" },
  "inputFiles": [
    { "path": "models/base.blend", "role": "source" },
    { "path": "refs/front.png", "role": "dependency" }
  ]
}
```

`inputs` 继续保存 Codex 报告的 JSON。`inputFiles` 是必须显式提供的数组，Beaver 读取实际文件并将捕获结果保存到 attempt；省略或传入 `null` 会被拒绝。`inputFiles: []` 表示本次明确声明没有文件输入。历史 attempt 缺少该字段时显示“未采集”，不会自动转成空清单。

文件声明中的 `role` 决定后续复核方式：

- `source`：保存开始时的源文件快照，允许在建模中编辑工作副本。后续复核冻结的输入字节，不要求当前源文件保持原状。
- `dependency`：保存开始时的依赖快照，后续同时复核冻结字节和当前工作副本 SHA-256。依赖变化后需要重新执行受影响步骤。

可选 `expectedSha256` 必须是已知的真实小写 SHA-256，格式在文件 IO 前检查，随后与 Beaver 实际捕获的哈希比较。没有已知版本时省略该字段，不填写示例哈希或猜测值。

输入只接受工作副本内的相对文件路径，使用 `/` 分隔；外部参考资料先复制到工作副本。最多 32 个文件，单个文件非空且不超过 256 MiB，总计不超过 512 MiB。拒绝越界路径、内部路径、重复路径和仅 ASCII 大小写不同的重复项。内容存入现有内容寻址 blob 库，attempt 保存路径、角色和哈希，避免把文件内容塞进回调日志。

路径、角色和完整性由调用方声明。当前实现没有扫描 Blender 场景寻找依赖，也不能仅凭该清单证明 Codex 实际使用了全部声明文件。

## 复核与恢复

成功 `finish`、`submitDelivery` 和 owner 批准候选时，Beaver 复核对应 attempt 的输入。多个 attempt 声明同一路径时，各版本均保留并验证；后来的哈希不能覆盖并隐藏前面 attempt 的冲突。成功结束仍是完成报告，阶段推进继续需要候选文件及 owner 批准。

依赖变化后的恢复方式取决于当前状态：

1. attempt 仍在运行：用 `finish` 报告 `failed`，检查文件与实际场景后，用新请求 ID 和 `recoveryNote` 重新 `begin`，捕获新输入。失败结束不会被依赖漂移阻塞。
2. 子任务已完成、尚未提交候选：调用 `work` 的 `change: {action: "reopen", subtaskId, reason}`。目标必须是当前阶段已完成子任务，且没有运行中的 attempt 或待 owner 审批。目标及同阶段后续未取消子任务变为 `stale`，保留旧 attempt；按原顺序带 `recoveryNote` 重新执行。
3. 候选已经待审批：owner 退回候选，再检查并重新执行。依赖漂移不会阻止 owner 退回，Codex 不能自行跳过审批。
4. 阶段已经批准：使用现有 owner 阶段返工流程，撤销受影响的批准并保留历史。`work.reopen` 不能直接修改已批准阶段。

同一执行身份、请求 ID 和相同 JSON 的回执重试返回已保存结果，不重新捕获文件；即使当前文件已经变化，也只确认原请求当时已提交的事实。新执行先查询旧回执并检查工作副本，再使用新请求 ID。协议升级后，新 `begin` 必须符合版本 4；版本 3 中省略 `inputFiles` 的请求不能直接作为版本 4 的重试输入，历史回执仍可查询，执行器按工具版本建立新 thread。

候选批准只检查该候选绑定的 attempt 输入；已批准上游输出仍按既有文件规则复核。最终合入沿用全部阶段批准和输出一致性检查，没有新增对全部历史 attempt 输入依赖的递归扫描。

## 宿主实现与观察

输入捕获和依赖文件 IO 在 SQLite 锁外执行。私有 prepared 对象保存精确请求 JSON；提交时重新核验 callback revision、asset revision、thread / turn 和当前状态。准备好的文件证据不能授权不同的请求。新 `begin`、成功 `finish` 和 `submitDelivery` 经过异步适配器；直接同步调用缺少宿主证据时会被拒绝，已保存回执仍可按原约定重试。

本批还修复了尚未建立 Blender 上下文时的动态回调分发。该分支原先直接进入同步回调，文件请求会失败；现在统一走异步适配器，无资产上下文时从宿主 SQLite 路径取得数据目录。模拟执行器已通过这一真实分发路径捕获并复核 `project.godot`，没有依赖 Blender 会话来使文件清单生效。

资产窗口的只读“子任务与执行尝试”面板分别显示“Beaver 保存的输入文件版本”和“Codex 报告的输入、输出与工具”。输入面板显示路径、源文件 / 依赖角色和 SHA-256，并区分历史未采集、显式空清单及非空清单。工具与插件报告仍标为 Codex 报告，没有升级成宿主自动观测到的真实调用轨迹。

主要实现入口：

- [输入捕获与复核](../native/core/src/asset_work_inputs.rs)、[锁外准备与请求绑定](../native/core/src/task_callback_prepare.rs)、[异步适配器](../native/core/src/task_callback_runtime.rs)。
- [work 协议](../native/core/src/asset_work_contract.rs)、[子任务操作与恢复](../native/core/src/asset_work_actions.rs)、[候选审批](../native/core/src/asset_delivery_review.rs)。
- [动态工具分发](../native/core/src/executor_tools.rs)、[输入观察面板](../src/ui/asset-task/WorkPanel.tsx)、[受管理 Codex 指令](../resources/instructions/task-callback.md)。

## 定向验证

已通过 **37 项 Rust 测试、2 项 React SSR 组件测试、3 个模拟 Codex 执行器场景及 TypeScript 类型检查**。Rust 测试同时完成了相关代码编译验证。日志保存在 [本批证据目录](../output/asset-input-validation/)，不会将模拟子进程或组件渲染结果计作真实模型制作和原生窗口验收。

| 范围                                       | 结果                                                                 | 证据                                                                                                                                       |
| ------------------------------------------ | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| 输入、依赖、work、恢复、交付、回调集成测试 | 31 项通过                                                            | [core-tests.log](../output/asset-input-validation/core-tests.log)                                                                          |
| IO 后身份 / revision 变化、请求与证据绑定  | 2 项通过；分别覆盖 begin、成功 finish 和 submit                      | [prepared-tests.log](../output/asset-input-validation/prepared-tests.log)                                                                  |
| 执行器动态工具分发                         | 4 项通过                                                             | [dispatch-tests.log](../output/asset-input-validation/dispatch-tests.log)                                                                  |
| 新 thread、恢复 thread、旧协议升级         | 3 个场景通过；含无 Blender 上下文的文件捕获、中断与新 attempt        | [executor/proof.json](../output/asset-input-validation/executor/proof.json)、[executor.log](../output/asset-input-validation/executor.log) |
| 输入面板                                   | 2 项通过；历史 / null / 空清单、宿主与模型信息区分、角色及 HTML 转义 | [ui-tests.log](../output/asset-input-validation/ui-tests.log)                                                                              |
| TypeScript                                 | `tsc --noEmit` 通过                                                  | [typecheck.log](../output/asset-input-validation/typecheck.log)                                                                            |

实际命令在仓库根目录通过普通 PowerShell / MSVC 环境执行，保留原有 `LIB`：

```powershell
rtk proxy cargo test --locked -p beaver-core --test asset_work_inputs --test asset_work_dependencies --test asset_work --test asset_work_recovery --test asset_delivery --test asset_delivery_integrity --test task_callback
rtk proxy cargo test --locked -p beaver-core --lib task_callback_prepare::tests
rtk proxy cargo test --locked -p beaver-core --lib executor_tools::tests
rtk proxy cargo run --locked -p beaver-core --example executor_contract -- output/asset-input-validation/executor --callbacks
rtk proxy npx tsx --test tests/asset-work-panel.test.ts
rtk proxy npm run typecheck
```

PowerShell 日志中的 `NativeCommandError` 包装来自原生命令向 stderr 写入编译进度，成功记录以命令退出码和测试结果为准。执行器最初失败时发现的无 Blender 上下文分发问题已经修复，并重新运行通过；旧失败轮次仍保留在证据目录。

交付静态检查已通过：`cargo fmt --all -- --check`、9 份本批 TS / TSX / Markdown 文件的 Prettier 检查、`npm run check:effective-lines` 和 `git diff --check`。有效行数扫描 490 个源文件，0 项违规，18 个未改动的历史文件沿用原有基线；没有调整 baseline。26 份本批源文件和文档均为 UTF-8 无 BOM，文档本地链接目标存在。Git 日志中仅有行尾转换提示，没有差异格式错误。

静态检查日志：[Rust 格式](../output/asset-input-validation/rust-format.log)、[Prettier](../output/asset-input-validation/format-check.log)、[有效行数](../output/asset-input-validation/effective-lines.log)、[差异检查](../output/asset-input-validation/diff-check.log)；汇总见 [final-checks.json](../output/asset-input-validation/final-checks.json)。测试通过后仅运行交付所需静态检查，没有扩大到全流程测试。

## 当前边界与剩余工作

文件哈希证明字节身份，不证明 `.blend` 可正确打开、拓扑合格、NPR 画面达标或声明角色恰当。复核发生在有界业务检查点，没有为外部编辑器提供跨文件原子锁定；文件系统仍可能在检查之后变化。捕获失败可能留下未引用 blob，但失败请求不会提交部分任务状态或回执。

框架后续仍需逐批完成：

1. **Godot / Codex / Blender 插件生命周期**：查询、选择、安装、兼容性、启用、重载和实际可调用状态；先复用现有准备机制，以一个真实需要的插件验证。
2. **真实工具与 Skill 调用关联**：把可观察的调用请求、结果和版本关联 attempt，明确无法观测的缺口，不能用模型报告代替工具轨迹。
3. **有版本的技术与语义检查**：将检查器、规则、候选版本、实际报告和视觉评价来源绑定，输入或规则变化后重新判断结果。
4. **观察和恢复联动**：为新增 attempt / 输入面板补充聚焦原生验证，进一步关联实时帧、执行身份与候选预览；未保存场景恢复仍需相应证据。
5. **耗时操作查询与取消**：当前文件请求有数量及大小上限，仍直接等待结果，尚无持久 operation 查询 / 取消协议。
6. **按制作需求补充依赖发现及查看能力**：当前只支持显式清单，没有 Blender 依赖完整性扫描、跨历史输入依赖闭包复核或专用输入快照导出界面。

后续按上述缺口逐项交付和批准，再进入已讨论的 NPR 小步骤与工具调用链迭代。本批没有写死人物阶段模板或提前实现整套 NPR Skills，也没有运行全流程游戏 / NPR / 发布验收。
