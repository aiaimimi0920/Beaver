# F8 Blender 冻结网格选择与归档

2026-09-26。本增量完成 Blender 静态网格从冻结预览、真实点选和穿透框选，到编号意见保存及重开回看的流程。F8 和 F9 的整体清单保持未完成。

## 实现

- Blender worker 在渲染帧边界复制世界坐标三角形与相机矩阵；选择查询只使用该份冻结数据。请求必须匹配 session、revision、sequence 和实际 PNG SHA-256。
- 点选返回最近的三角形命中、位置、法线和对象身份。框选使用包含近远裁剪面的六面裁剪，返回与区域相交的对象，包括被遮挡网格；最多 32 个对象，超限明确截断。
- 对象身份采用 JSON pointer 转义的 /objects/ 路径。修改器、形态键、实例及不支持几何会跳过并报告；单帧最多 100000 个三角形。按渲染层、集合和对象的 render visibility 收集，viewport 隐藏不影响渲染可选对象。
- Core 复用现有可信选择回执、冻结图像和持久化边界；启动 worker 时安装 Blender picking 模块。Desktop 工具说明及生产 UI 显示 Blender 能力和限制，保留编号、局部上下文、逐项意见和总体意见。
- 修复自动观察相机在复制 matrix_world 前未更新 view layer 的问题；真实初始图已人工检查，立方体可见且无相机落在立方体内部的异常。

## 本轮验证

真实 Blender 5.2.1 LTS，经 Core Service API：

- object_blender_real_mesh_picking_and_archive：1 项通过，38.32 秒。检查真实中心命中、空命中、被遮挡对象框选、近远裁剪、范围外对象、32 项截断、重复请求、过期帧和请求冲突；伪造对象回执明确返回 PREVIEW_SELECTION_PICK_UNTRUSTED。归档及重开读取沿用真实持久化路径。
- object_blender_live_camera_archive_and_lease：1 项通过，35.52 秒。覆盖自动相机、移动、分辨率、复位、冻结字节稳定性、关闭和租约恢复。
- 单独 Blender 可见性探针通过：viewport-hidden 对象保留，render-hidden 对象与集合排除；两个 Python 模块 AST 解析通过。探针是模块级补充，未宣称完整引擎场景验收。
- 前端 preview-picking、live-scene-preview、preview-selection：12 项通过；npm run typecheck 和 cargo check --locked -p beaver-desktop 通过。
- 相关 Prettier 和 Rustfmt 检查通过。npm run check:effective-lines：1177 个源文件、17 个未修改历史超长文件、0 项违规。git diff --check 通过。

Chromium 使用生产 ObjectScenePreview 组件，模拟 API 传输重放上述真实 PNG 和命中回执。完成冻结、点选、框选、两条意见、总体意见、保存、关闭和重载回看，断言归档保留两个真实来源命中。该检查证明 UI 接线；Core 的真实引擎与持久化由上述测试独立验证。未执行原生 WebView 全流程验收。

本地证据：

- [真实选择归档](../output/blender-pick-proof/blender-live-archive.json)
- [真实冻结帧](../output/blender-pick-proof/blender-live-frozen.png)
- [自动相机初始图](../output/blender-pick-camera-proof/blender-live-initial.png)
- [生产组件重开回看](../output/playwright/blender-live/picking-restored.png)
- [Core 日志](../output/blender-pick-core-final.log)、[相机日志](../output/blender-pick-camera.log)、[UI 日志](../output/blender-pick-ui-tests.log)、[浏览器日志](../output/blender-pick-browser.log)

## 边界与后续

本能力只支持冻结静态网格，穿透框选不代表可见像素覆盖；不支持动画、修改器求值、形态键或实例选择。当前轨道相机中心取整场景边界，极远离群几何可能放大旋转位移；裁剪夹具采用小幅相机运动，普通自动相机回归保留正常运动幅度。

旧版本及拓扑变化后的重新定位、F8 其余完整条件核对、F9 生产接线与原生开发版交付仍需推进。本轮未更新产品版本、未发布，未将 F8.1-F8.5 或 F9 标记完成。下一步优先关闭旧坐标重新定位到当前目标并提交反馈的用户流程。
