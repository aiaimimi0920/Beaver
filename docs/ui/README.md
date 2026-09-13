# Beaver UI 设计文档

移植日期：2026-09-06。

状态：Beaver 0.1.6 新增紧凑的[创作方向与功能块筛选](../UPDATES-0.1.6.md)，保留 0.1.5 的[侧栏图标固定位置](../UPDATES-0.1.5.md)、Loom 壳层和统一右上角通知。正式图标沿用用户于 2026-09-06 选定的第四轮 A「一句成游」，见[正式图标规范](BRANDING.md)。候选稿按历史原样归档：[第四轮](icon-rounds/round-04/index.html)、[第三轮](icon-rounds/round-03/index.html)、[第二轮](icon-rounds/round-02/index.html)、[第一轮](icon-rounds/round-01/index.html)。原始参考副本仍保持不变。

当前 0.1.10 将[游戏总览](./PROJECT_OVERVIEW.md)改为右上独立设定卡与二级卡牌选择器，左上只有游戏名称，其余左侧留白；保留 0.1.9 编辑锁与保存后的强提示。0.1.8 的[功能包规划](./ENGINE_PACKAGES.md)和 0.1.7 的[项目规划原型](./PROJECT_BLUEPRINT.md)继续保留。上方 0.1.6 说明为视觉沿革，本轮仍先确认控件语义，不执行业务。

## 1. 采用的设计方向

Beaver 采用 Neuro 当前统一设计方案，并参考 Loom 的紧凑桌面工作台交互经验：

- 近黑色、不透明的应用壳层，配合有限的白色重点阅读面。
- 信号黄用于激活、焦点和当前上下文的唯一主操作。
- 信号绿用于品牌、在线、成功和完成。
- 信息蓝与危险红保持各自的业务语义。
- 紧凑排版、清晰分隔、有限切角和低密度工业结构图形。

这是视觉与交互规则的移植，不是把 Beaver 改成 Loom 的工作流编辑器，也不复制 Neuro / Loom 的产品标识和业务模块。

## 2. 阅读顺序

1. [Beaver UI 实现提示词](./AI_PROMPT.md)。
2. [Beaver UI 设计规范与适用边界](./DESIGN_SYSTEM.md)。
   同时遵守 [紧凑界面补充约束](./COMPACT_UI.md)，不要重新加入口号和重复说明。
3. [Neuro 原始提示词](./references/neuro/AI_PROMPT.md)与[完整设计系统](./references/neuro/DESIGN_SYSTEM.md)。
4. [精确 JSON 令牌](./references/neuro/tokens.json)与[CSS 令牌及派生公式](./references/neuro/tokens.css)。
5. [离线视觉参考页](./references/neuro/index.html)与[参考截图](./references/neuro/screenshots/)。
6. 按需查阅下面的 Loom 历史 UI 文档。

离线参考页保留上游名称与演示数据，可以直接在浏览器中打开。它不是 Beaver 原型，也不是 Beaver 已实现功能的演示。

## 3. 已复制的资料

| 来源 | 本地副本 | 定位 |
| --- | --- | --- |
| Neuro `docs/UI设计与颜色方案/` | [完整参考包](./references/neuro/README.md) | 当前权威视觉基线；包含 8 个文档/网页文件与 5 张截图 |
| Loom `docs/analysis/phase-27-workflow-graph-ui-audit.md` | [工作流图 UI 审计](./references/loom/analysis/phase-27-workflow-graph-ui-audit.md) | 历史交互分析，不是 Beaver 功能要求 |
| Loom `docs/progress/phase-27-workflow-graph-ui.md` | [工作流图 UI 阶段记录](./references/loom/progress/phase-27-workflow-graph-ui.md) | 历史选择/属性编辑经验与验证记录 |
| Loom `docs/progress/phase-35-cn-ui-hook-sync.md` | [中文界面与截图入口记录](./references/loom/progress/phase-35-cn-ui-hook-sync.md) | 中文文案、紧凑操作和截图上下文入口参考 |

来源根目录：

```text
C:\Users\Public\nas_home\AI\GameEditor\Neuro
C:\Users\Public\nas_home\AI\GameEditor\Neuro\Loom
```

## 4. 权威性与历史差异

- 精确色值、派生公式及通用视觉规则以本地 Neuro 副本为基线；Beaver 的产品语义与首版范围以本目录的适用说明为准。
- Loom 的 Phase 27 / 35 是历史记录，不是新的全局设计系统。文中 `glass UI`、`modern-gradient`、pill 等描述不能覆盖当前 Neuro 的近不透明表面、克制装饰和单一主色规则。
- Loom 的图节点、YAML 编辑器、Art、Hook Bridge、MCP 商店和协议名称不自动成为 Beaver 功能。Beaver 不要求人类编排 Codex 内部工作流。
- 上游文档中的旧绝对路径、skill 路径、代码位置和已通过的测试记录按原文保留，只表示上游历史；它们不是 Beaver 的文件依赖、执行指令或验证结果。
- `--neuro-*`、`--p-*` 等是参考页的变量，不代表 Beaver 已经存在相同的主题 API。后续实现应按语义映射，不能假装已有组件或变量。
- 原始参考页的 `styles.css` 和 `app.js` 仅服务于文档预览，不引入为 Beaver 生产代码或决定其技术栈。

## 5. 副本管理

[sources.json](./sources.json) 记录 16 份源文件的绝对来源、相对目标、字节数和 SHA-256。复制的是当前来源工作树内容，包括尚未提交的资料，不假定它们来自干净提交。

`references/` 中的文件按字节复制，不改写产品名、历史记录和原始措辞。后续 Beaver 特有规则写在本目录，避免把改写后的内容冒充上游原文。

本副本不自动追随源目录变化，也不依赖用户机器上的 Neuro、Loom 或 Codex skill 路径。更新参考资料时应先审阅差异，再显式更新副本与来源清单。

## 6. 文档移植范围与当前实现

最初移植仅新增 Beaver UI 文档与离线参考资料；没有移动或修改 Neuro/Loom 源文件。后续获准开发后，应用实现位于 `src/ui`，构建与真实桌面测试记录见 [验证说明](../VALIDATION.md)。这不改变上游参考文件的历史含义。
