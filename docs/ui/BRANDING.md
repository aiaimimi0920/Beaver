# Beaver 正式图标：一句成游

确认日期：2026-09-06。用户确认原文：「一句成游这个图标很好，就这个了，我很喜欢」。正式采用[第四轮 A 原稿](icon-rounds/round-04/01-talk-to-play.svg)，不再继续候选设计。

![一句成游](icon-rounds/round-04/01-talk-to-play.svg)

## 图形与使用

- 对话气泡与游戏手柄合为一个轮廓，表达「通过对话制作游戏」。Beaver 名称保持不变，「一句成游」是图标概念名。
- 保留原稿的轮廓、比例、透明十字方向键与双按键，不重新描画或添加字母、齿轮、星光等装饰。
- 主体信号绿 `#22c55e`，按键信号黄 `#d9ff38`；透明背景，不添加渐变、阴影、发光或底板。
- 壳层图标容器为 28 × 28 CSS px，沿用 Loom 对齐后的标题栏与侧栏尺寸，不增加说明文案。
- 本图标是品牌标识，不替代任务成功、错误、播放等功能图标。

## 唯一母版与生成

唯一生产母版为 `resources/branding/beaver.svg`，与获选原稿逐字节一致。前四轮候选只作为历史档案，不是运行时依赖。

执行 `npm run build:icons`，从母版生成：

| 文件 | 用途 |
| --- | --- |
| `beaver-16/20/24/32/48/64/128/256.png` | 同一造型的透明 PNG，各尺寸独立从 SVG 栅格化 |
| `beaver.ico` | 包含全部 8 个尺寸的 Windows 托盘与 EXE 图标 |
| `dist/beaver.svg` | 构建时复制，供界面与 favicon 使用 |

不要手工修改派生 PNG/ICO；`npm run build` 会先重建图标。栅格化使用锁定版本的 [resvg-js](https://github.com/thx/resvg-js)，不加载系统字体，不依赖本机绘图软件。

## 桌面与打包入口

- 界面品牌位和 favicon 使用同一份 SVG。
- 窗口使用 256 px PNG；Windows 托盘直接使用多尺寸 ICO，其他平台代码路径使用 32 px PNG。
- Windows 打包使用 [ResEdit](https://github.com/jet2jet/resedit-js) 与 `pe-library`，只修改新发布目录里的 Electron 副本，将所有图标组替换为 Beaver，并更新产品名、文件名和版本。
- 不修改依赖目录中的 Electron，不覆盖旧发布目录，不更改用户数据目录。资源编辑后的预览 EXE 未签名，不能沿用 Electron 原始签名的信任状态。
- 当前实际发布目标仅为 Windows x64；macOS / Linux 图标与打包尚未完成真实系统验证。

## 回归约束

`tests/branding.test.ts` 检查原稿一致性、透明镂空、精确配色、多尺寸生成和 ICO 帧往返一致性。打包时再次读取生成的 PE 图标资源，逐帧比较 PNG 字节。桌面冒烟测试检查真实界面 28 px 图标与 favicon，并通过 Windows shell 从实际启动的 EXE 提取图标，保存截图作为证据。

本次只确立应用品牌，不重新设计游戏项目的美术，不修改 Neuro / Loom 原始参考资料。
