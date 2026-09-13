# R10 自动决策实现与交互验收

日期：2026-09-09。可运行包：`release/Beaver-native-0.1.19-preview-r10d-win32-x64/Beaver.exe`。

本次完成全局和单任务自动决策、五档重要性筛选、推荐答案填充、决策来源记录，以及共享 API / MCP 接口。单任务设置为“跟随全局”时实时继承；显式设置覆盖全局。已有安装默认询问 100%。

## 实际交互结果

全部创作输入、项目创建、回答提交与任务推进均通过 Beaver API 完成。主测试代理没有直接调用 Codex、Godot 或 Blender，没有代写游戏代码、素材或游戏设计文档。下面的方向文档由 Beaver 调用其 AI 后生成。

1. 全局完全自动，任务继承：太空救援弹幕肉鸽的三个选择全部采用 AI 推荐，任务没有停在等待用户状态，自动回复提问工具并继续完成方向确认。任务 ID：`c68a522a-a0e2-489c-9524-2ea13d355882`。
2. 全局完全自动，任务覆盖询问 100%：赛博朋克游戏产生三个带选项、推荐、理由和重要性的真实问题。改成完全自动后，原问题仍保持等待回答。UI 中保留一个自定义答案，明确点击填充后补齐另外两个推荐；刷新再进入仍保留草稿和来源。通过 API 提交后，任务完成，记录为一个用户答案和两个自动答案。任务 ID：`c69c5719-0a77-45a1-9181-debaaa81d860`。
3. 任务询问 10%：奇幻弹幕肉鸽产生重要性为 95、88、78 的三个问题。Beaver 仅询问 95 分的核心操作，自动选择另外两个推荐。手动选择操作方案后继续完成方向确认。任务 ID：`d4a94d4c-c8a3-4fbe-ae04-1f4beedc0822`。

三个任务最终均为 `completed`，没有任务错误。自动决策记录和混合回答来源在重启后仍存在。后两个“待用户问题”和“自动决策”数组可能包含同一项显式填充的问题；来源以 `answerSources` 和 `autoAnswers` 为准，不能把两个数组长度相加当作问题总数。

## 验证范围与证据

- TypeScript：`npm run typecheck` 通过，`npm test` 共 85 项通过。
- Rust：`cargo test --locked -p beaver-core` 共 83 项通过；桌面 `cargo check` 和 release 编译通过。
- 格式：本次变更涉及的 TypeScript / CSS 文件通过 Prettier 检查，Rust 通过 `cargo fmt --all -- --check`。
- UI：两个最终验收脚本共 12 项断言通过，覆盖五档全局控件、任务覆盖、推荐说明、自定义答案保护、显式填充、刷新持久化、决策记录和实际生效档位。
- API：五档任务设置全部生效；五档全局设置影响继承任务且不改变覆盖任务；null 恢复继承；任务非法档位 20 返回 400，全局非法档位返回 409；把自定义答案冒充自动推荐返回 409，随后正常提交成功。
- MCP：真实 stdio 初始化、45 项能力发现、`task.autonomy` 调用共享正在运行的 API 状态，3 项断言通过。
- 中间档位的精确边界由确定性测试覆盖；本轮真实 AI 会话验证了完全自动、全部询问和询问 10% 的混合分流，没有把 30% / 70% 的配置验证宣称为真实 AI 会话验证。
- 包完整性验证通过：606 个文件，15,720,533 字节。已从最终包启动，PID 在验收时为 36756，API 端口为 4321。

最终包与已测试构建的 EXE SHA-256 相同：

`1CD99DCB3FDBBB4F3ECDF26F7D9F28E9031590BB25C07E5CDFCB443642C00A0B`

隔离验收数据：`output/validation/autonomy-live-1788917846936`。

可复核文件：

- `output/validation/autonomy-proof.json`
- `output/validation/autonomy-ts-tests.log`
- `output/validation/autonomy-rust-tests.log`
- `output/validation/autonomy-mcp.mts`
- `output/playwright/autonomy-check.mts`
- `output/playwright/autonomy-history.mts`
- `output/playwright/autonomy-pending.png`
- `output/playwright/autonomy-history.png`
- `output/playwright/autonomy-settings.png`

## 测试中处理的问题

打包两次遇到 GitHub 原始文件服务 429。增加可选的历史许可证缓存来源：先验证旧发布包完整性，再严格匹配包名、版本、许可证声明和固定上游提交，复用匹配的许可证原文。R10d 使用已验证的 R9 许可证完成打包，没有修改旧包。新增参数用法：

```powershell
rtk proxy npx tsx scripts/package-native.ts <new-release-name> <verified-old-release-directory>
```

R10、R10b 是下载失败候选，R10c 是缓存清单解析失败候选，均保留以便追踪；交付入口只使用 R10d。缓存清单的 `packages` 包装层读取已修正并通过真实打包和完整性验证。

UI 验收脚本最初假定刷新后仍停留在原任务，且使用了错误的项目控件名称，发生等待超时。脚本改为显式选择项目并重新进入任务后通过；未为脚本定位错误修改产品行为。

## 尚未覆盖

本次证明自动决策功能和方向确认交互闭环成立。完整游戏制作、可玩性、长任务中重要性评分的稳定性、全部发行与迁移门禁仍需后续验收。比例采用重要性阈值，单次三问不会强制凑成固定百分比。

发布仍是未签名预览版，清单保留 `runtimeVerified:false` 和 `migrationComplete:false`；本次定向运行证据不替代完整产品认证。通用 `native-business-smoke` 的能力数量断言已更新为 45，但本轮没有重跑其文档写入场景，MCP 与新 API 行为使用上述定向验收验证。
