# F8 候选尝试编号存档本轮返工

日期：2026-09-26。状态：完成尝试编号存档到本轮返工、恢复、模型图片交付及发布反馈保留的子流程；全部 Beaver 开发计划及 F8/F9 仍未完成。

## 可使用的流程

用户在候选输出反馈面板选择“本轮返工”，选择该尝试的原始编号存档，回看区域意见后进入费用与恢复确认。确认面板显示原 runId/frameId；确认后才创建新尝试。存档与冻结 PNG 文件标注互斥，切换反馈去向保留引用，未确认请求重试使用原始请求。

新尝试通过 beaver_object_attempt 的 feedbackImage 读取原始 PNG、编号区域意见、相机和来源。执行说明要求查看原图并重新定位历史二维区域。项目重开、原请求重试及失败或中断后的普通重试沿持久回执保留原引用；发布反馈继续保存此引用，供已有审阅回看入口精确读取。

此增量替代上一份尝试存档延后反馈记录中“本轮返工禁止提交”的限制；历史证据保留原意。

## 实现与边界

- Core 的返工授权接受 previewFrame，在事务内复用已有精确尝试存档校验；拒绝 image 与 previewFrame 同时提交，以及缺失或不匹配的存档。
- 模型回调从经过校验的恢复回执解析绑定，核对当前尝试与回执身份；普通重试沿前驱回执查找最近返工来源，防止循环或多重绑定。阶段推进或无帧的新返工不继承旧图。
- Desktop 的 objectTask.reworkCandidate 接受严格 runId/frameId 引用；生产反馈、草稿恢复及确认面板使用同一契约。
- 回调复用 PNG 完整性校验和 1 MiB 模型图片上限；存档元数据与图像分开处理。历史二维区域不作为真实三维命中，也不保证自动完成跨版本拓扑定位。

## 验证

Native 命令通过现有 PowerShell 工具链的 rtk proxy 执行，保留 MSVC/LIB 环境。

| 检查                                                                                                    | 结果                                             | 日志                                                   |
| ------------------------------------------------------------------------------------------------------- | ------------------------------------------------ | ------------------------------------------------------ |
| cargo test --locked -p beaver-core candidate_rework                                                     | 9 通过                                           | output/rework-frame-core.log                           |
| cargo test --locked -p beaver-core --lib publication_deferred                                           | 8 通过（含同一新增返工用例）                     | output/rework-frame-adjacent.log                       |
| cargo test --locked -p beaver-desktop candidate_review_routes_final_output_and_replays_without_dispatch | 1 通过                                           | output/rework-frame-desktop.log                        |
| tsx：object-candidate-rework、object-candidate-feedback-draft、publication-frame-picker                 | 9 通过                                           | output/rework-frame-ui.log                             |
| npm run typecheck                                                                                       | 退出码 0                                         | output/rework-frame-types.log                          |
| 定向 rustfmt --check、Prettier --check                                                                  | 通过                                             | output/rework-frame-format-check.log；rustfmt 退出码 0 |
| npm run check:effective-lines                                                                           | 1160 个源文件，17 个未改动历史超限文件，0 项违规 | output/rework-frame-structure.log                      |
| git diff --check                                                                                        | 退出码 0，仅现有换行转换提示                     | output/rework-frame-diff-check.log                     |

新增 Core 回归覆盖双图片来源和缺失存档的原子拒绝、主机停止后重开、修改请求冲突与原请求幂等、普通中断重试继承来源，以及实际 worker/RPC 子进程交付原 PNG 和完整存档元数据，最后完成检查、审阅和发布反馈保留。Desktop 回归覆盖缺失存档通过真实路由被拒绝。前端回归覆盖存档确认、提交前不调用返工、重试与未确认请求恢复，同时保留 PNG 路径覆盖。

初次测试发现测试夹具在 worker 绑定前保存的 target 缺少 thread/turn 身份，导致 OBJECT_ATTEMPT_STALE_TARGET；已改为从最终尝试重新取得 target，最终 9 项通过。初次 Desktop 过滤器匹配 0 项，已使用真实测试名称重跑并通过 1 项。只读独立检查未发现明确的身份越界或引用丢失。

本轮归档由夹具构造，未执行真实 Godot 采集、外部模型调用、浏览器交互、Beaver.exe 原生验收或正式发布；worker/RPC 回归不能替代这些证据。

## 剩余工作

Blender 真实预览、真实三维命中、跨版本拓扑重新定位及 F9 全路径集成收口仍未完成。F8.1-F8.5 和 F9 保持未勾选。未变更版本，未提交或推送。
