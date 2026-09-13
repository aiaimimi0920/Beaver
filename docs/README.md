# Beaver 文档

设计文档：[游戏代码验收与画面流程开发设计](GAME-VALIDATION-DESIGN.md)，2026-09-13。保留 GUT 代码验收、独立 Tab、四类可重播画面流程、对话反馈及正式发布诊断开关的需求和讨论记录。

实现记录：[游戏验收功能实现与验证](GAME-VALIDATION-IMPLEMENTATION.md)，2026-09-13。功能已在独立分支实施，记录使用方式、真实引擎证据、集成状态及尚未执行的原生交互验收。

设计文档：[资产制作任务窗口开发设计](ASSET-TASK-WINDOW-DESIGN.md)，2026-09-13。记录实时 Blender 预览、三种修改定位方式、流程干预及与人物 skill 的交接；当前仅完成设计，尚未实施。

当前正在进行 Electron 到 Tauri/Rust 的迁移。最新 R8 原生预览完整目录为 15.66 MB，新增共用桌面逻辑的 44 项业务 API/MCP 操作，并通过真实程序验证；导入 UI、完整回退与产品验收仍未完成。下列 0.1.x 文档保留各轮历史契约，不作为最新完成状态清单。

- [最新选项交互修复与两轮真实交互验收](NATIVE-RELEASE-R9.md)。
- [R8 原生发布与 API/MCP 验证证据](NATIVE-RELEASE-R8.md)。
- [R7 会话迁移与显式启用证据](NATIVE-RELEASE-R7.md)。
- [完整数据备份、恢复格式和迁移边界](DATA-MIGRATION.md)。
- [业务 API、MCP 接入、共享执行逻辑与当前边界](BUSINESS-API.md)。
- [原生迁移的完整进度与剩余门槛](NATIVE-MIGRATION.md)。

- [使用与开发入口](../README.md)。
- [0.1.8 引擎能力功能包规划](./UPDATES-0.1.8.md)。
- [全部引擎能力包、来源键和交互含义](./ui/ENGINE_PACKAGES.md)。
- [0.1.7 项目规划交互原型](./UPDATES-0.1.7.md)。
- [项目规划控件含义与未来交付契约](./ui/PROJECT_BLUEPRINT.md)。
- [0.1.6 本机 API 与游戏创作目录](./UPDATES-0.1.6.md)。
- [游戏类型、题材与功能块目录](./GAME_CATALOG.md)。
- [0.1.5 侧栏图标固定位置](./UPDATES-0.1.5.md)。
- [0.1.4 正式图标「一句成游」](./UPDATES-0.1.4.md)。
- [0.1.3 统一通知与图标第一轮](./UPDATES-0.1.3.md)。
- [0.1.2 Loom 壳层对齐](./UPDATES-0.1.2.md)。
- [0.1.1 界面与工具发现修复](./UPDATES-0.1.1.md)。
- [产品需求与首版范围](./PRODUCT.md)。
- [本地架构、文件恢复和安全边界](./ARCHITECTURE.md)。
- [AI 服务接入与协议](./AI_SERVICES.md)。
- [验证记录](./VALIDATION.md)。
- [后续目标](./ROADMAP.md)。

- [UI 设计文档入口](./ui/README.md)：Neuro / Loom 设计资料副本、Beaver 适用边界和阅读顺序。
- [Beaver UI 设计规范](./ui/DESIGN_SYSTEM.md)：继承的视觉规则与本项目语义映射。
- [UI 实现提示词](./ui/AI_PROMPT.md)：供后续界面开发与审阅使用。

`ui/references` 保留上游原始副本，不随应用实现改写或污染 Neuro/Loom 来源。
