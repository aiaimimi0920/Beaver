# Beaver 创作布局草案 01

状态：待用户讨论与批准。仅独立 HTML 原型，不代表正式应用已采用此信息架构。

直接打开 `index.html`。不需要启动 Beaver、安装依赖或连接 AI 服务。素材、文档、任务状态和工具就绪状态均为示例；交互只改变网页内存，刷新还原。

## 本轮要讨论什么

- 一级入口改为「创作、资料、素材、游戏」，是否符合实际制作习惯。
- 创作页采用「任务列表 + 当前任务的结果 / 对话 / 变更 + 底部反馈」，而不是展示内部编排。
- 资料按内容归档；人可阅读和编辑，Codex 自行查找需要的上下文。
- 游戏下收纳总览、创作重点、功能与试玩导出。基础设定默认锁定。
- 创作环境与设置固定在左下角；零环境用户先走独立引导。

页面顶部的「首次使用」「创作工作区」「布局说明」属于审稿工具，不属于预期产品导航。右上窗口控制是位置示意，不操作浏览器窗口。

## 范围边界

- 素材缩略图、音频试听、模型观察、游戏画面均为占位，不制作实际游戏内容。
- 安装、连接验证、账户、额度、生成、导出、回退、功能包合并都不执行业务。
- 项目创建和接管弹窗只确认字段位置与入口，部分字段为示意，不保存完整配置。
- 密钥不写入文件或浏览器存储；请勿在原型中输入真实凭据。
- 新模板只在创建时选择。市场、外部渠道发行、云端执行不进入首版导航。

## 维护

只修改本目录。`prototype.ts` 是交互源码，`prototype.js` 是浏览器可直接加载的派生文件。

在 Beaver 项目根目录重新生成：

```powershell
rtk npx --no-install prettier --write docs/ui/prototypes/round-01/index.html docs/ui/prototypes/round-01/styles.css docs/ui/prototypes/round-01/prototype.ts
rtk npx --no-install esbuild docs/ui/prototypes/round-01/prototype.ts --bundle --format=iife --target=chrome110 --charset=utf8 --outfile=docs/ui/prototypes/round-01/prototype.js
```

沿用现有「一句成游」图标、186 / 52 px 侧栏、50 px 顶栏和 Neuro 色彩语义；没有修改正式运行时、原有设计文档或发布目录。侧栏折叠图标锚点不移动。

验证记录位于 `output/playwright/beaver-layout-r01/VALIDATION.md`。
