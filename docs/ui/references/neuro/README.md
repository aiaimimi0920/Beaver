# Neuro 设计方案

这里是 Neuro 系列应用唯一的跨项目 UI 设计基线，正式名称为 **Neuro 设计方案**。实验阶段的其他候选不属于产品规范。

## 读取顺序

1. AI 或开发者先读 [`AI_PROMPT.md`](./AI_PROMPT.md)。
2. 再读 [`DESIGN_SYSTEM.md`](./DESIGN_SYSTEM.md)，理解颜色角色、构图、组件和交互原则。
3. 实现时以 [`tokens.json`](./tokens.json) 和 [`tokens.css`](./tokens.css) 为精确令牌源。
4. 打开 [`index.html`](./index.html) 查看控件、状态、应用框架和对比度的可视参考。
5. 需要快速比对时查看 [`screenshots/`](./screenshots/)；截图是验收参考，不是源代码替代品。

## 文件职责

| 文件 | 职责 |
| --- | --- |
| `AI_PROMPT.md` | 可直接交给其他 AI 的实现提示词和执行规则 |
| `DESIGN_SYSTEM.md` | 设计语义、页面结构、交互和验收规范 |
| `tokens.json` | 机器可读的颜色、角色、派生状态与构图元数据 |
| `tokens.css` | 可复制或映射到项目主题系统的 CSS 变量 |
| `index.html` | 只有一套方案的离线设计参考网页 |
| `styles.css` / `app.js` | 参考网页实现；不作为产品运行时依赖 |
| `screenshots/` | 参考网页的已验证截图 |

## 权威性

- 本目录是跨项目的设计方向和精确颜色来源。
- 各产品运行时代码仍是“当前已实现能力”的事实来源。应用本规范前必须检查目标项目真实存在的主题变量、组件、状态和宿主限制。
- 如果文档与运行时代码不一致，不要假装已经迁移完成；应在任务范围内完成映射或明确记录差异。
- 不要新增另一套平行的“Neuro 主色”。产品可有局部业务色，但必须保留这里定义的角色边界。

## AI skill 集成

当前 Codex 用户环境中的设计 skill 是：

```text
Z:\_windows_home\vmjcv\.codex\skills\neuro-ui-style\SKILL.md
```

该 skill 只负责识别适用任务、规定工作流程，并把 AI 引导到本目录。精确颜色、组件语义和构图规则只在本目录维护，避免 skill 与设计文件分别保存两套易漂移的颜色代码。

使用支持 skill 的 Codex 环境时，可显式要求：

```text
Use $neuro-ui-style to implement this Neuro application surface.
```

在没有该 skill 的其他 AI 环境中，直接提供本目录并要求 AI 先读取 `AI_PROMPT.md`，可以获得等价的设计约束。

## 本地预览

该网页没有 CDN、外部字体或网络脚本依赖。可直接打开 `index.html`，也可以从本目录启动任意静态服务器。例如：

```powershell
npx --yes http-server . -a 127.0.0.1 -p 1427 -c-1
```
