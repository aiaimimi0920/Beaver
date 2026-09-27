# F2 导入读取与准备实施记录

日期：2026-09-22。对应 [整体开发计划](FRAMEWORK-IMPLEMENTATION-PLAN.md) 的 F2.3 三来源只读读取，以及 F2.4 导入准备。

本批完成项目接受版本的冻结预览、依赖闭包、内容校验、身份映射与请求级准备回执，并补齐外部项目目录选择、自动身份识别、三来源统一读取和普通文件准备的原生分发。普通文件/文件夹支持路径累积、只读清单预览、人工归组及独立准备回执；分组编辑会清除旧回执并恢复可操作状态。F2.4 与 F2.1 的最小版本联调已经完成：`object.acceptVersion` 只把指定版本从 `captured` 转为 `accepted`，保留版本、文件、blob、组件、引用、metadata 和登记字段，不复制内容、不发布、不创建 task，也不执行 F7 正式导入。目标对象目录不变，回执始终返回 `readyToCommit: false`；正式复制、身份改写、目标对象提交和恢复由 F7 负责。

## 实现和可观察行为

接受版本使用 schema version 1 的类型化清单，必须显式标记为 accepted，且项目、对象和版本归属一致。清单固定对象名称、组件、文件路径/角色/大小/SHA-256 和引用版本。候选版本、缺失或不支持的清单、错误归属及非法路径返回阻塞原因。预览使用接受版本的冻结名称与文件清单，当前工作目录的同名文件不会改变预览或准备结果。

依赖遍历以 `(objectId, versionId)` 为键，支持循环、共享依赖和同一对象的多个固定版本。准备摘要来自确定的版本闭包；检查后版本或引用清单变化，准备必须失败。每个文件只从 `.beaver/content/blobs/<sha256>` 读取，核对大小与 SHA-256，并拒绝链接/reparse 路径、非普通文件及单文件超过 512 MiB 的内容；缺失 blob 不会退回读取工作目录。

项目来源统一经过 `ProjectStorageRouter` 校验清单身份、宿主登记路径和 Runtime 归属。已打开的源项目复用其 Store，在短事务内读取已提交快照；已登记但关闭的项目及未登记的外部项目获取源 lease，在临时 SQLite 副本上读取，准备期间保留 lease。读取不会新增宿主登记、迁移源项目、打开新的源 Runtime、运行源脚本或改写源数据库。外部目录只需选择路径，项目 ID 从 `.beaver` 清单读取；传入预期 ID 时必须匹配，对象目录为空时也会保留合法项目身份。

每次新准备使用新的 `requestId`，目标项目和请求共同限定准备身份及映射。短 SQLite Immediate 事务执行回执的比较和插入，不覆盖已有回执。相同请求可以在源项目离线时重放原回执和身份映射；相同请求 ID 携带不同输入返回 `IMPORT_REQUEST_CONFLICT`。改变来源或选择后开始新的准备会生成新请求 ID。回执保存规范化源路径、输入摘要、固定版本闭包、来源信息和对象/组件/版本身份映射，供 F7 使用。

`ObjectFrameworkWorkspace` 原有的对话框目标固定行为继续保留。新的导入 session 在构造时保存目标，提交明确的版本 ID 和检查摘要；修改来源、对象或版本及关闭对话框都会失效化旧操作，晚到的成功或失败不会覆盖当前选择。传输失败后的重试复用原请求，重复提交和准备成功后的重复提交被阻止。界面显示不可准备原因、冻结文件和准备回执，明确提示正式导入尚未提交。

对话框分别提供已登记项目、外部 Beaver 项目和普通文件/文件夹入口，项目身份只读。外部目录选择后自动检查；目录取消 `null`、文件取消 `[]` 均保留当前选择，非法响应显示错误。选择器序号与会话修订共同过滤重叠操作，即使最新一次选择被取消，较早的结果也不能覆盖当前来源。普通文件选择保持累积；增删分组或调整文件归属会失效化在途检查/准备，清除回执和请求身份，有快照时回到 `ready`，否则回到 `editing`，下一次准备使用新请求 ID。

## 业务契约

- `object.inspectExternal` 必须传入绝对 `path`，可选 `projectId` 作为预期身份、`query` 作为查询条件；返回发现的 `projectId`、对象目录及 `importVersions`。显式空值、非字符串或空项目 ID 会被拒绝。每个版本包含 `objectId`、`versionId`，并提供可准备的 `sourceDigest` 或不可准备的 `blocker`；此步骤不校验 blob，内容校验发生在准备时。
- `object.prepareImport` 必须传入 `requestId`、`targetProjectId`、`source: { path, projectId }`、`objectId`、`baseline` 和检查获得的 `sourceDigest`。`source` 仅允许这两个字段，路径必须为绝对路径。API 支持 `latestAccepted` 与 `pinnedVersion`，拒绝空基准；界面统一使用 `{ kind: "pinnedVersion", versionId }`。
- `object.getImportPreparation` 接收 `projectId` 和 `preparationId`，只返回对应目标项目的回执，不访问源项目。
- `object.inspectFiles` 接收非空 `paths`，返回普通文件/文件夹的只读清单快照。
- `object.prepareFileImport` 接收 `requestId`、`targetProjectId`、`snapshot` 和 `groups`，重新校验快照并保存请求级准备回执，不复制文件或登记对象。
- `object.getFileImportPreparation` 接收 `targetProjectId` 和 `preparationId`，只读取该目标项目的普通文件准备回执。

以上六个方法统一通过原生导入分发边界。两种检查的业务调用日志仅写宿主，不关联源项目或任务；准备及回执查询绑定目标项目，普通文件回执也持久化在目标 Store。项目导入拒绝目标与源项目相同，普通文件准备不套用项目来源身份规则。

`latestAccepted` 忽略候选版本，仍须匹配检查时的闭包摘要。界面校验检查响应的项目/对象/版本对应关系，拒绝重复或缺失的选项；回执必须匹配原请求、目标、来源、对象、版本和摘要，不能把不匹配响应显示为成功。

## 源码职责与规模

- [object_version_manifest.rs](../native/core/src/object_version_manifest.rs)（122 有效行）负责接受清单及逻辑路径校验；[object_import_snapshot.rs](../native/core/src/object_import_snapshot.rs)（128 行）负责闭包、归属检查和摘要；[object_import_content.rs](../native/core/src/object_import_content.rs)（46 行）负责内容地址文件校验。
- [object_import_preparation.rs](../native/core/src/object_import_preparation.rs)（173 行）编排准备；[object_import_receipt.rs](../native/core/src/object_import_receipt.rs)（91 行）负责请求冲突和不可覆盖的回执持久化；[object_external_snapshot.rs](../native/core/src/object_external_snapshot.rs)（103 行）保持外部源 lease 与只读快照寿命。
- [project_object_source.rs](../native/core/src/project_object_source.rs)（115 行）解析源身份和所有权；[object_source_snapshot.rs](../native/core/src/object_source_snapshot.rs)（78 行）承载共用项目快照；[object_import_file_source.rs](../native/core/src/object_import_file_source.rs)（275 行）负责普通文件的只读清单。
- [data_dispatch_object_import.rs](../native/desktop/src/data_dispatch_object_import.rs)（263 行）统一六个导入方法的原生分发；[data_dispatch.rs](../native/desktop/src/data_dispatch.rs)（383 行）接入该边界；[business_catalog.rs](../native/desktop/src/business_catalog.rs)（348 行）声明公开参数；[business_routing.rs](../native/desktop/src/business_routing.rs)（327 行）限定调用日志归属。
- [object-import.ts](../src/shared/object-import.ts)（210 行）解析未知 API 响应；[object-import-session.ts](../src/ui/object-preview/object-import-session.ts)（497 行）管理导入会话的异步选择、重试和失效；[ObjectImportDialog.tsx](../src/ui/object-preview/ObjectImportDialog.tsx)（357 行）负责表单与状态显示。

上述导入模块及本轮相关测试最大为 497 有效行，251-500 行文件分别保持读取、分发、会话或展示职责。新增身份发现、原生分发、日志路由、文件分组及选择器测试分别为 116、152、214、223、195 有效行。所有改动源码为 UTF-8 无 BOM，没有为本轮变更更新历史超限 baseline。完整计数见 [结构检查报告](../output/effective-code-lines.json)。

## 实际验证

Rust 检查在保留 MSVC `LIB` 的正常 PowerShell 环境执行。以下均为本批新运行的定向检查：

- `rtk proxy cargo test --locked -p beaver-core object_`：39 项通过，0 失败；[日志](../output/f2-external-selection-20260922-043415/core-object-tests.log)。覆盖冻结文件、依赖闭包、新身份、目标目录不变、过期摘要、离线重试、请求冲突、内容及路径拒绝，以及未登记/已登记关闭/已打开项目的身份发现、路径归属、源快照一致性和源文件不变。
- `rtk proxy cargo test --locked -p beaver-desktop object_import`：13 项通过，0 失败；[日志](../output/f2-external-selection-20260922-043415/desktop-import-tests.log)。覆盖公开参数校验、六方法分发、普通文件检查/准备/取回回执、目标隔离及幂等、源身份发现与错误输入、宿主/目标日志归属和自导入拒绝。
- `rtk proxy npx tsx --test tests/object-import-contract.test.ts tests/object-import-session.test.ts tests/object-import-file-contract.test.ts tests/object-import-file-session.test.ts tests/object-import-picker.test.ts`：34 项通过，0 失败；[日志](../output/f2-external-selection-20260922-043415/frontend-tests-final.log)。覆盖目标固定、冻结预览、空对象目录身份、取消与重叠选择、晚到成功/失败、文件累积、分组编辑失效及新请求重试。分组修复前新增回归为 5 通过、4 失败，复现 `preparing` / `inspecting` 状态未重置；[修复前日志](../output/f2-external-selection-20260922-043415/group-regression-before.log)。
- `rtk proxy cargo check --locked -p beaver-desktop`：通过；[日志](../output/f2-external-selection-20260922-043415/cargo-check-desktop.log)。
- `rtk proxy npm run typecheck`：通过；[日志](../output/f2-external-selection-20260922-043415/typecheck-final.log)。
- `rtk proxy cargo fmt --all -- --check`：通过；[日志](../output/f2-external-selection-20260922-043415/cargo-fmt-check.log)。八个相关 TS/TSX 文件及本次两份文档的 `rtk proxy npx prettier --check` 通过；[日志](../output/f2-external-selection-20260922-043415/prettier-check-final.log)。
- `rtk proxy npm run check:effective-lines`：通过，0 违规；[日志](../output/f2-external-selection-20260922-043415/effective-lines.log)。历史超限文件继续作为既有债务列出。
- `cargo test --locked -p beaver-core object_`：62 项通过，0 失败；覆盖登记、更新、捕获及 `captured -> accepted` 的 receipt 幂等、请求冲突、revision 和对象投影一致性。
- `cargo test --locked -p beaver-desktop object_catalog`：6 项通过，0 失败；覆盖 acceptance business schema、dispatch、状态转换、receipt replay 及不创建 task 的回归。
- `npx tsx --test tests/object-catalog.test.ts tests/object-command.test.ts`：7 项通过，0 失败；覆盖 acceptance command/receipt schema、版本状态、hash、组件、登记字段、revision 和版本数量校验。

核心编译仍报告 `validation/service.rs` 中既有的未读取字段，桌面测试报告既有的多余 `mut`；本批无编译错误。测试使用临时项目和受控 API 响应，未执行原生窗口或全流程验收，不把静态渲染及 session 测试记为真实 WebView 交互验证。

## 迁移、交付和后续

没有变更项目数据库 schema 或写入用户项目，也未修改版本、构建发布包或启动 Beaver。产品元数据保持公共基线 `0.1.19`、内部迭代 `36`；已有发布记录保留。旧的、从可变工作文件生成的准备回执缺少本次冻结协议字段，读取时明确拒绝，需用新请求重新检查并准备；不静默转换或据此提交。

本批三来源读取与准备代码及定向验证已完成，F2.4 与 F2.1 的最小版本联调也已完成：指定 `captured` 版本可以通过 `object.acceptVersion` 转为 `accepted`，但 `readyToCommit` 仍为 `false`，因此 F2.4 仍保持未勾选。尚无用户对本批原生界面的验收反馈，整体计划和 F2 继续进行；下一批转入 F3 任务拆分、统一任务身份和依赖队列，再推进 F4/F5。F7 正式导入提交仍未实现，内容复制、身份改写、目标对象提交、完整版本发布和恢复仍待完成；F3-F9 后续工作按整体计划推进。
