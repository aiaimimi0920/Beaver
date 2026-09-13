# Beaver 图标提案：第二轮

2026-09-06。第一轮全部未获认可，本轮按用户最新要求重新提供 **6 组**，不是对第一轮做换色。仅设计评审，不替换正式应用、托盘或发行包图标。

打开 [离线 HTML 对比页](index.html)，可以查看深浅底色、16/24/32/48/64px、28px 品牌组合、单色轮廓和应用底板。选 A-F 或“全部重想”，复制意见发回对话；页面不自动发送，也不持久保存意见。

| 方向 | 标识                            | 出发点                                         |
| ---- | ------------------------------- | ---------------------------------------------- |
| A    | [负木者](01-timber-carrier.svg) | 完整侧影、扁尾与搬木动作，保留河狸的生物特征。 |
| B    | [尾印](02-tail-sign.svg)        | 抽取最有辨识度的扁尾，形成独立的工具符号。     |
| C    | [啮刻](03-gnaw-cut.svg)         | 不直接描绘动物，以材料上的咬口和木屑表达创造。 |
| D    | [榫合](04-dovetail.svg)         | 木构件与燕尾榫，把创造表达为结构咬合。         |
| E    | [筑坝](05-dam-builder.svg)      | 堆叠构件与新加入的一块，表达逐步搭建世界。     |
| F    | [凿匠](06-chisel-maker.svg)     | 棱角侧面与平口门齿，探索更硬朗的工业标识。     |

六组均为手工构造的 SVG 矢量候选，使用 Neuro / Loom 的信号绿 `#22c55e` 与信号黄 `#d9ff38`。没有复制 Loom 的铁砧标识，没有渐变、外部字体或网络资源。孔洞透明，单色预览使用图像滤镜而非依赖本地文件跨域行为的 CSS mask。

## 生成与检查

`review.ts` 为交互源码，`review.js` 为可直接通过 `file://` 加载的 IIFE 产物。

```powershell
rtk npm exec -- esbuild docs/ui/icon-rounds/round-02/review.ts --outfile=docs/ui/icon-rounds/round-02/review.js --platform=browser --format=iife
rtk npm exec -- tsc --noEmit --skipLibCheck --target ES2022 --module ESNext --moduleResolution Bundler docs/ui/icon-rounds/round-02/review.ts
rtk npm exec -- prettier --check docs/ui/icon-rounds/round-02/index.html docs/ui/icon-rounds/round-02/review.css docs/ui/icon-rounds/round-02/review.ts docs/ui/icon-rounds/round-02/review.js scripts/icon-review-smoke.mjs
```

浏览器检查复用 `scripts/icon-review-smoke.mjs`，第二个参数为预期候选数：

```powershell
# BEAVER_PLAYWRIGHT 指向本机已安装的 Playwright 包；浏览器使用已安装的 Edge。
$env:BEAVER_BROWSER_CHANNEL = 'msedge'
rtk node scripts/icon-review-smoke.mjs docs/ui/icon-rounds/round-02 6
```

结果写入 `output/playwright/icons-round-02-<timestamp>/`，包含深色、浅色、窄屏截图及 `proof.json`。本轮不修改游戏编辑器功能，不增加依赖，不重发应用安装包。
