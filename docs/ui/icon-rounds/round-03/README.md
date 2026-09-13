# Beaver 图标提案：第三轮

2026-09-06。前两轮全部未获认可，本轮继续提供六组候选。放弃构件、工具、字母式拼接，重新围绕河狸的形态做轮廓探索。不替换正式应用、托盘或发行包图标。

打开 [离线 HTML 对比页](index.html)。本轮减少说明文案，保留深浅底色、16/24/32/48/64px、28px 品牌组合、单色与应用底板，直接比较图形。

| 编号 | 候选                         | 方向                           |
| ---- | ---------------------------- | ------------------------------ |
| A    | [宽颊](01-broad-cheeks.svg)  | 宽颊、平鼻、双门齿的正面形象。 |
| B    | [游弋](02-river-swimmer.svg) | 向前游动的完整侧影。           |
| C    | [回环](03-orbit-beaver.svg)  | 回环外轮廓与紧凑侧面。         |
| D    | [抱尾](04-curled-beaver.svg) | 蜷身朝左，宽尾环抱在身前。     |
| E    | [线刻](05-line-beaver.svg)   | 连续粗线勾勒全身。             |
| F    | [折面](06-folded-beaver.svg) | 单体折面与几何切口。           |

保持 Neuro / Loom 的信号绿 `#22c55e` 与信号黄 `#d9ff38`，不复制 Loom 的铁砧图形。矢量文件无背景、无渐变、无外部资源。色彩图标在浅底上的对比度是本页展示的实际效果；单色版本另列，不能用它代替彩色预览结论。

页面选择仅在当前页面有效，不会自动发送或持久保存。复制意见后发回对话；剪贴板不可用时提供手动复制。修改意见或候选会清除旧的复制反馈，避免把过期内容误认为当前选择。

## 复现检查

```powershell
rtk npm exec -- esbuild docs/ui/icon-rounds/round-03/review.ts --outfile=docs/ui/icon-rounds/round-03/review.js --platform=browser --format=iife
rtk npm exec -- tsc --noEmit --skipLibCheck --target ES2022 --module ESNext --moduleResolution Bundler docs/ui/icon-rounds/round-03/review.ts
rtk npm exec -- prettier --check docs/ui/icon-rounds/round-03/index.html docs/ui/icon-rounds/round-03/review.css docs/ui/icon-rounds/round-03/review.ts docs/ui/icon-rounds/round-03/review.js docs/ui/icon-rounds/round-03/README.md
# BEAVER_PLAYWRIGHT 指向本机已有的 Playwright 包。
$env:BEAVER_BROWSER_CHANNEL = 'msedge'
rtk node scripts/icon-review-smoke.mjs docs/ui/icon-rounds/round-03 6
```

浏览器检查覆盖六组独立资源、本地图形加载、深浅色切换、键盘选择、全部重想、离线复制回退、无外部网络请求以及 360/560/800/1200/1440/1920px 布局。截图与结果写入 `output/playwright/icons-round-03-<timestamp>/`。仅图标评审页验证，不代表图形已经获得用户认可或已经应用到软件。
