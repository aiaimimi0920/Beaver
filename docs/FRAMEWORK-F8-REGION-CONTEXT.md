# F8 编号区域上下文缩略图

日期：2026-09-26。完成 F8.4 的区域上下文展示增量；F8 和 F9 整体仍未完成。

## 可用结果

用户编辑冻结帧或 PNG 的编号意见时，每个区域旁显示原图位置和带留白的局部上下文。保存后的存档选择器及发布反馈审阅复用同一展示。删除区域后，缩略图编号与剩余意见同步；窄屏自动换行。

`RegionContextThumbnail.tsx` 使用已解码的原图 URL，不启动引擎、不重新读取文件、不改变冻结来源、归一化坐标或存档身份。局部视图保留纵横比，裁剪范围限制在原图内；显式 SVG clipPath 避免细长区域的图像内容溢入留白。图片未解码或尺寸不匹配时不展示缩略图。

生产接入位置：

- `src/ui/object-preview/PreviewFrameRegions.tsx`：实时冻结帧、存档及版本/尝试选择器共用。
- `src/ui/object-tasks/ObjectReworkImageEditor.tsx`：PNG 返工意见编辑。
- `src/ui/object-tasks/ObjectPublicationFeedback.tsx`：发布审阅回看。

## 验证

- `npx tsx --test tests/preview-selection.test.ts tests/publication-frame-picker.test.ts tests/object-rework-image.test.ts`：7 项通过，0 项失败。覆盖直接依赖的坐标、身份及图片反馈边界。
- 最终 `npm run typecheck`、修改源码的 Prettier 检查通过。
- 最终 `npm run check:effective-lines`：1161 个源文件，17 个未修改历史超限文件，0 项违规；未调整基线。
- Chromium 使用生产 `PreviewFrameRegions` 和独立生成的四色 PNG 夹具，完成编辑第二个区域、删除首个区域、保存后只读回看及整页重载。剩余区域重新编号为 1，意见仍为 `keep right strip`。
- 360 像素视口无水平溢出。伪造图片尺寸时显示 `PREVIEW_FRAME_DIMENSIONS`，局部缩略图数量为 0；恢复正确尺寸后为 1。
- 亲自检查宽屏只读和窄屏截图，确认细长右边缘区域仅显示对应绿色/黄色上下文，留白不再出现越界图像。首次截图暴露裁剪问题，修复后重新构建夹具并检查。

日志：`output/region-context-{tests,types,structure,format-check,browser-results,replay,diff-check}.log`。
浏览器夹具：`output/playwright/region-context/main.tsx`。
截图：`output/playwright/region-context/replay.png`、`output/playwright/region-context/narrow.png`。

浏览器保存/重载使用夹具 localStorage，验证生产组件展示和交互；本轮未重复 Core 持久化、Desktop、真实引擎、外部模型或原生发布验收。返工 PNG 和发布反馈接入经类型检查，未单独运行其完整浏览器流程。既有持久化和模型交付证据见前轮尝试存档返工记录。

## 剩余目标

真实三维命中集合、拓扑定位、Blender 预览及 F9 集成仍待实现。二维上下文缩略图不提供三维命中证据，F8.1-F8.5 和 F9 保持未勾选。未提交、推送或修改产品版本。
