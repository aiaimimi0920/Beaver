# Beaver 图标提案：第四轮

2026-09-06。用户否定前三轮后，明确要求更有趣、容易辨认、与软件概念相关的图标。本轮不再以河狸外形为必要条件，围绕“通过对话，把想法变成可玩的游戏”探索六个不同的概念。

打开 [离线 HTML 对比页](index.html)，可查看深浅底色、16/24/32/48/64px、28px 品牌组合、单色轮廓与应用底板。所有方案都是待选候选，不代表已经定稿。

| 编号 | 候选                            | 概念与软件的联系                                            |
| ---- | ------------------------------- | ----------------------------------------------------------- |
| A    | [一句成游](01-talk-to-play.svg) | 对话气泡与游戏按键融合，直接表达“对话做游戏”。              |
| B    | [游戏出炉](02-game-toaster.svg) | 烤箱弹出游戏卡带，把从想法到成品的过程变成有趣的日常比喻。  |
| C    | [像素孵化](03-pixel-hatch.svg)  | 像素生命从蛋壳中出现，表达从点子孵化角色与游戏。            |
| D    | [造梦之门](04-dream-door.svg)   | 带播放切口的门扇向外打开，表达进入自己创造的可玩世界。      |
| E    | [愿望引擎](05-wish-engine.svg)  | 神灯回应愿望，呼应用户提出目标、AI 自行完成过程的使用方式。 |
| F    | [世界种子](06-world-seed.svg)   | 嫩芽从立方世界中生长，表达从一个想法不断生长出内容。        |

依旧采用 Neuro / Loom 的信号绿 `#22c55e`、信号黄 `#d9ff38`、简洁几何与透明切口；不复制 Loom 的铁砧图形。六组不是同一图形的换色或朝向变化。本轮审阅中把 B 的单独十字改成成组游戏按键，把 D 的台阶底座改成真正打开的门扇，减少医疗和墓碑歧义；最终辨识度仍由用户判断。

页面选择只在当前页面有效，不持久保存、不自动发送。复制意见后发回对话；剪贴板不可用时可手动复制。前几轮、应用源码、正式图标和发行包均不修改。

## 复现检查

`review.ts` 为交互源码；`review.js` 为可通过 `file://` 加载的 IIFE。页面和六组 SVG 没有外部字体、脚本或网络资源依赖。

```powershell
rtk npm exec -- esbuild docs/ui/icon-rounds/round-04/review.ts --outfile=docs/ui/icon-rounds/round-04/review.js --platform=browser --format=iife
rtk npm exec -- tsc --noEmit --skipLibCheck --target ES2022 --module ESNext --moduleResolution Bundler docs/ui/icon-rounds/round-04/review.ts
rtk npm exec -- prettier --write docs/ui/icon-rounds/round-04/index.html docs/ui/icon-rounds/round-04/review.css docs/ui/icon-rounds/round-04/review.ts docs/ui/icon-rounds/round-04/review.js docs/ui/icon-rounds/round-04/README.md
# BEAVER_PLAYWRIGHT 指向本机已有的 Playwright 包。
$env:BEAVER_BROWSER_CHANNEL = 'msedge'
rtk node scripts/icon-review-smoke.mjs docs/ui/icon-rounds/round-04 6
```

检查覆盖六组独立资源、本地 SVG 加载、深浅色、键盘选择、全部重想、离线复制回退、无外部网络请求和 360/560/800/1200/1440/1920px 布局。截图与 `proof.json` 位于 `output/playwright/icons-round-04-<timestamp>/`。本轮不重发应用安装包。
