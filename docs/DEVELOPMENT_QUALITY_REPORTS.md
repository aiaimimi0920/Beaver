# 开发期静态质量报告

普通 PR 的静态质量发现作为非阻断报告保留，不表示问题已修复。
Development Quality Reports 在 PR、main、定期和手动运行，保留原始输出、
退出码、结构化分类及 check summary；artifact 缺失或上传失败仍报错。
没有新增 Issue 写权限，报告按 workflow/run/check 区分，不生成重复修复任务。

Beaver 先严格编译 code-structure scanner，再分类 binary 的 0/1/2 退出。共享 npm、本地 build:native 与游戏任务的代码结构检查不改。
分类器验证实际输出与退出码一致；未知诊断、语法/配置错误、空扫描、
缺失或过时报告和解析失败仍失败，超时也保留已有输出。

功能测试、类型检查、编译、来源及安全契约不放宽。
正式发布 workflow、签名、部署及本地严格入口不变；开发报告不授予发布资格。
代码设计标准及历史问题仍需遵守和后续修复。
