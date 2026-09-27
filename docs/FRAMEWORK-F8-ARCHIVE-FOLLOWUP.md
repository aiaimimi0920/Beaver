# F8 版本交互存档后续任务

日期：2026-09-26。状态：已完成已发布版本的 Godot 编号帧附加后续任务子流程；整体 F8/F9 仍未完成。

## 可使用的流程

在对象版本的 Godot 交互预览中冻结画面，填写二维编号区域及意见并保存。在该版本对应的发布记录中打开后续任务表单，选择已保存帧，回看原图和编号意见，填写标题、反馈及验收要求后创建任务。创建结果为 planned 中任务及细任务，由用户继续确认入队，不自动执行。

选择器展示精确项目、对象、版本下最近 8 个带区域记录的存档，按保存时间倒序读取，不启动引擎。已绑定但超出最近 8 帧范围的引用仍保留并可继续提交；刷新列表不会替换它。草稿、未确认原请求及成功回执沿用既有恢复流程，重开后不会自动重复创建。

执行后续任务时，模型的 feedbackImage 回调获取保存时的原始 PNG、编号区域、逐区域和总体意见、相机、画面尺寸及版本/运行/快照来源。历史区域需要在目标内容中重新定位，不宣称真实三维命中，也不自动计入验收。

## 一致性与边界

- 创建请求仅保存 runId/frameId 引用。Core 在创建事务中验证发布记录、预览运行已完成、项目/对象/版本、快照及源路径/hash，并校验归档摘要、PNG 和区域记录；失败不插入计划任务。
- 模型读取沿用发布回执、细任务和运行身份校验，再解析不可变存档。PNG 数据与来源元数据分开交付，文本任务上下文不嵌入 base64。
- 同一创建请求的重试复用原回执；不同引用不能冒充同一请求。原回执存在后，即使存档缺失也可重查创建结果，但模型读取会明确失败，不改用新画面。
- 列表在 SQL 层筛选身份并限制最近 8 帧，避免把全部历史 PNG 读入应用。旧引用直接解析，不依赖列表是否仍展示。
- PNG 解码限制宽高不超过 16384、解码分配不超过 128 MiB。模型图片传输沿用 1 MiB 上限；超限返回明确文本状态和来源，不声称已交付图片。界面提示降低分辨率后重新保存。
- 未改变旧的发布前延后 PNG 反馈协议；旧草稿不含 previewFrame 时仍可恢复。草稿与未确认请求之间新增引用字段的一致性同样受校验。

## 本轮验证

以下命令通过 rtk proxy 在现有 PowerShell 工具链中执行，未清理 MSVC/LIB 环境。

| 检查                                                                                                                            | 实际结果                                         | 日志                                                                                                                   |
| ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| cargo test --locked -p beaver-core --lib publication_followup                                                                   | 5 通过                                           | output/archive-followup-core.log                                                                                       |
| cargo test --locked -p beaver-core --lib publication_deferred_image                                                             | 3 通过                                           | output/archive-followup-adjacent.log                                                                                   |
| cargo test --locked -p beaver-core --lib preview_selection_archive                                                              | 1 通过                                           | output/archive-followup-selection.log                                                                                  |
| cargo test --locked -p beaver-desktop publication_routes_accept_and_retain_history_without_redispatch                           | 1 通过                                           | output/archive-followup-desktop.log                                                                                    |
| npx tsx --test tests/object-publication-followup.test.ts tests/publication-frame-picker.test.ts tests/preview-selection.test.ts | 10 通过                                          | 本轮先前定向执行结果                                                                                                   |
| npx tsx --test tests/publication-frame-picker.test.ts                                                                           | 最终限制修改后 2 通过，不重复计入前述 10 项      | output/archive-followup-picker.log                                                                                     |
| npm run typecheck                                                                                                               | 退出码 0                                         | output/archive-followup-types.log                                                                                      |
| 定向 rustfmt 与 Prettier                                                                                                        | 退出码 0                                         | output/archive-followup-rustfmt.log、output/archive-followup-prettier.log、output/archive-followup-guidance-format.log |
| npm run check:effective-lines                                                                                                   | 1154 个源文件，17 个未改动历史超限文件，0 项违规 | output/archive-followup-structure.log                                                                                  |
| git diff --check                                                                                                                | 退出码 0                                         | output/archive-followup-diff-check.log                                                                                 |

Core 回归覆盖错误项目/对象/版本/运行/快照、篡改摘要/PNG/区域的原子拒绝，原请求幂等，归档移除，以及最近 8 帧排序和更早引用继续解析。重开回归使用实际 worker 和 RPC 子进程夹具，验证原 PNG 与相机/编号意见进入回调，完成后仍为 awaitingAcceptance。归档夹具直接构造持久记录；保存路径由相邻 preview_selection_archive 回归覆盖。

Desktop 验证实际读写路由和缺失帧拒绝。前端覆盖精确身份解析、生产编号组件静态呈现、引用随草稿恢复/重试保留，以及草稿与未确认请求引用不一致时阻止恢复。最终独立只读审查发现历史列表无界加载，已改为 SQL 限制 8 帧并补充回归。

编译日志仍有既有 unused_mut 与 validation State 未读取字段警告。本轮没有调用外部模型，没有执行浏览器交互或 Beaver.exe 原生验收，没有重复真实 Godot 渲染；上述证据不等同于这些验收。

## 后续工作

尝试来源的交互存档仍需接入当前返工和发布前延后反馈。Blender 预览、真实引擎命中、跨版本拓扑重新定位及 F9 全路径集成收口仍未完成。本次未勾选 F8.1-F8.5 或 F9，未变更版本，未提交或发布。
