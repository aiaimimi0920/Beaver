# F5.3 冻结文件正文查看

2026-09-25，本切片已完成 Core、Desktop API 和生产执行记录页面接入。用户展开尝试的冻结文件清单，点击“查看输入”或“查看输出”，可查看当时保存的 UTF-8 正文。当前工作区文件被修改或删除后，读取仍使用持久化检查点。

## 行为与边界

- `objectTask.attemptFile` 使用 projectId、runId、attemptId、checkpoint、path 和 sha256 定位文件。Core 校验项目、尝试归属、清单成员和预期摘要，只访问内容寻址 blob，不接受工作区或任意文件路径。
- 输入可在运行期间读取；输出必须已经冻结。历史查询不依赖当前执行 claim、thread 或 turn 身份。
- 最多读取 1 MiB UTF-8 文本，读取后验证 SHA-256。二进制、非 UTF-8 或控制字符内容明确显示不支持预览；缺失、损坏及不安全 blob 返回错误。
- 超过 1 MiB 时仅返回超限状态与元数据，不读取正文、不宣称完成完整性验证。该预览不替代技术检查、验收或发布。
- 正文由 React 文本节点转义。切换文件、关闭正文和关闭执行会话会失效旧请求；迟到成功或失败不能覆盖当前选择。
- 沿用已有持久化检查点，无 schema 迁移，无自动恢复或模型调用。

## 验证

- `cargo test --locked -p beaver-core object_attempt_file`：2 项通过。覆盖冻结输出在工作区变更/文件删除后仍可读、持久化输入清单读取、错误项目/run/attempt/path/hash 拒绝、损坏/缺失 blob、未冻结输出、二进制、空文件及超限。
- 输入正文测试使用保存的检查点 fixture；本批没有重新执行真实模型生成与恢复流程。
- `cargo test --locked -p beaver-desktop --bin Beaver object_attempt_runtime::tests`：3 项通过，覆盖 catalog 类型/枚举、项目路由及冻结清单成员拒绝；同时完成 Desktop 编译。
- 前端正文及既有文件清单测试：8 项通过；正文、执行生命周期及生产 UI 相邻测试：13 项通过。去重共 18 项，正文 3 项因测试类型收窄修正重新执行。
- `npm run typecheck`、定向 Prettier/Rustfmt、`git diff --check` 通过。
- `npm run check:effective-lines`：1059 sources、17 unchanged legacy files、0 violations；15 个改动源文件检查为 UTF-8 无 BOM。
- 日志：`output/f5-content-{core,desktop,ui,ui-adjacent,typecheck,format-check,rustfmt-check,rustfmt-final,structure,diff-check}.log`。

Desktop 验证曾暴露 checkpoint schema 缺少字符串类型，已补齐。既有 catalog 校验器不执行 pattern；测试现按该校验器实际职责检查字段类型，摘要身份由 Core 清单匹配及 blob SHA-256 验证负责。

## 剩余项

本批未生成、启动或发布新的原生程序，未执行正式原生交互验收。F5.3 的完整工具回调、能力发现和操作轨迹仍待补齐，计划保持未勾选。
