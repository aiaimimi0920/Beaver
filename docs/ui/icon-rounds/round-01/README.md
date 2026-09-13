# Beaver 图标候选 / 第一轮

状态：三组待选，尚未替换应用侧栏、托盘或 EXE 图标。

打开 [index.html](index.html)；无需服务、网络字体或 CDN。页面包含深浅背景、16 / 24 / 32 / 48 / 64px 尺寸、应用底板、单色和 28px 侧栏预览，以及选择与复制修改意见。

| 候选 | 含义 | 侧重点 |
| --- | --- | --- |
| A 河狸工匠 | 河狸面孔、门齿、扁尾、创造火花 | 角色辨识，亲近感 |
| B 字创造印记 | B 字与扁尾组合 | 几何工具品牌，小尺寸识别 |
| C 河狸工坊 | 河狸嵌入六角构建单元 | 游戏世界构建，结构感 |

图形是原创 SVG 矢量提案，不是定稿，也不声称已完成商标检索。沿用 Neuro / Loom 的信号绿 `#22c55e`、信号黄 `#d9ff38` 和简洁几何轮廓；不复制 Loom 的铁砧标记。单色预览是轮廓检查，不额外算作候选方向。

用户反馈后新增 `round-02`，仍提供三组并保留本轮。只有用户明确确认最终方向后，才生成正式应用、托盘和发行图标。页面点击选择只改变当前页状态，不会自动回传对话；复制文字后发给 Codex。

交互源文件 `review.ts` 通过项目现有 esbuild 构建为 `review.js`：

```powershell
npm exec -- esbuild docs/ui/icon-rounds/round-01/review.ts --outfile=docs/ui/icon-rounds/round-01/review.js --platform=browser --format=iife
```

验证脚本：`scripts/icon-review-smoke.mjs`。设置 `BEAVER_PLAYWRIGHT` 指向已有 Playwright；若使用系统 Edge，另设 `BEAVER_BROWSER_CHANNEL=msedge`。
