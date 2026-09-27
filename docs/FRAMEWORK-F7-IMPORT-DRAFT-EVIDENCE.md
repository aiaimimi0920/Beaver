# F7 导入编辑草稿恢复证据

日期：2026-09-25。范围：三种导入来源的未保存表单恢复。未修改 Core、Desktop API、产品版本或历史准备记录。

## 可用结果

已登记项目、外部 Beaver 项目和普通文件/文件夹共用按目标项目隔离的本地草稿。来源类型、路径、来源项目、对象/版本选择、文件路径、分组及尚未添加的分组名称随编辑保存。关闭并重新打开弹窗后可继续编辑；使用相同 WebView 存储目录重新启动应用时也可读取草稿。

草稿只含编辑字段，不保存检查快照、摘要、回执、执行请求或可提交状态。恢复后处于 editing，用户必须点击读取来源并完成新的检查，才能准备导入。对象和版本选择仅在新检查结果仍包含它们时恢复；文件分组重新检查时剔除已不存在的文件。源离线或检查失败时，输入和分组保留。重开历史准备及正式执行仍由原历史入口承担。

存储键为 `beaver.object-import-draft.v1.{targetProjectId}`；数据具有版本、目标身份和 Zod 结构校验。损坏、跨目标、不可读或超出存储配额均显示错误；读取失败不删除原草稿，继续编辑会尝试写入当前输入。禁用本地存储时，内存编辑仍可用，界面明确提示关闭可能丢失输入。草稿不跨设备同步，清理 WebView 数据会清除草稿。

弹窗沿用打开时绑定目标的行为：即使外部当前项目变化，目标标签、session 和草稿仍指向打开时的项目。需要另一目标时关闭后重新打开，不在进行中的输入或请求上静默切换目标。独立审查提出的“projectId 改变后 session 保持旧目标”符合这一已有约束；界面目标标签直接显示 `session.targetProjectId`。

关闭时先移除保存订阅，再取消 session；取消清除检查与请求结果并使迟到响应失效，不把清理过程覆盖回持久草稿。StrictMode 的清理保留编辑选择，不读取来源、不启动准备或正式提交。

## 实现与验证

草稿序列化与存储由 `object-import-draft.ts` 负责，React 生命周期由 `use-object-import-draft.ts` 负责，生产 `ObjectImportDialog.tsx` 已接入。为保持 session 大小，分组编辑提取至 `object-import-groups.ts`；新增分组会避开恢复数据中的 ID。session 为 493 有效行，保持单一交互状态机职责，未放宽结构基线。

- 定向测试：`npx tsx --test tests/object-import-draft.test.ts tests/object-import-session.test.ts tests/object-import-file-session.test.ts tests/object-import-picker.test.ts tests/object-import-contract.test.ts tests/object-import-history.test.ts`，40 通过、0 失败。日志：`output/import-draft-tests.log`。
- 新测试覆盖关闭重建 session、文件分组及未完成名称、三种来源、来源离线保留输入、显式重新检查、对象版本选择恢复、目标隔离、损坏/跨目标/版本错误数据以及存储拒绝读写。
- 首次 typecheck 发现测试对 getter 的 assert 收窄跨 async 调用后保持为 undefined，调整该测试断言后 `npm run typecheck` 通过；3 项草稿测试复验通过。日志：`output/import-draft-typecheck-recheck.log`、`output/import-draft-tests-recheck.log`。
- 修改文件 Prettier 通过，修正后的测试 Prettier check 通过。日志：`output/import-draft-format.log`、`output/import-draft-format-recheck.log`。
- `npm run check:effective-lines`：1031 个源文件、0 违规。日志：`output/import-draft-lines.log`。
- `git diff --check` 通过。新文件 UTF-8 无 BOM。

本轮没有启动原生窗口或实际浏览器做关闭重开验收，自动测试证明存储/session 行为，生产接线与 effect 顺序经过静态核验。没有重跑未修改的 Rust 测试或全量验收。

## 剩余目标

本轮关闭 F7.5 的未保存导入表单恢复缺口。F7.4 的已登记/外部 Beaver 项目依赖闭包正式导入、F7.5 的这些来源正式结果导航与生成对象完整流程仍未完成，两个条目保持未勾选。导入待验证版本的验证和提升也仍为后续工作。
