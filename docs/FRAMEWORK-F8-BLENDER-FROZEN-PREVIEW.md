# F8 冻结 Blender 预览交付证据

日期：2026-09-26。范围：对象版本及制作尝试检查点的 .blend 冻结实体预览。

## 可用结果

对象版本文件列表和制作尝试文件预览现在提供 Blender 三维预览入口。用户选择 540p、720p 或 1080p 后，通过既有受管预览队列渲染冻结 .blend；后台运行、取消、失败重试和重新查询沿用已有会话与任务机制。完成后显示图像、来源版本或尝试、场景摘要、快照、实际引擎版本、渲染器版本及图像摘要。关闭界面后结果继续保存在项目存储中。

Core 将 Blender 作业分派给独立适配器；Desktop 根据作业选择 Blender 工具配置，不再把所有预览交给 Godot。适配器恢复快照到临时副本，使用 factory-startup 和 disable-autoexec，核对依赖路径、输出尺寸及证据摘要。图像经既有 validation 媒体路由读取，损坏后明确报错。保留冻结场景相机；无相机时依据网格边界及输出宽高比计算观察距离。

## 定向验证

- 真实 Blender：C:/Program Files/Steam/steamapps/common/Blender/blender.exe，版本 Blender 5.2.1 LTS。
- 新增真实回归 object_blender_preview_real_frozen_render_and_dependency_rejection：1 项通过，最终运行 17.23 秒。分别验证保存相机、自动相机及缺失外部依赖；先捕获版本并删除原始 .blend，再走生产 enqueue、Service、latest 和媒体读取。两张 1280 × 720 图像含真实绿色几何；自动取景保留边缘；缺失依赖失败且无证据。重复请求不新建运行，关闭服务后重查一致，篡改图像触发完整性错误。
- 相邻 Core scene_preview 回归：4 项通过，覆盖冻结版本、请求恢复、分辨率及尝试入口；没有重复运行历史 Godot 真实引擎用例。
- 前端 object-scene-preview 和 live-scene-preview：11 项通过，含 .blend 版本请求重试和尝试检查点身份拒绝；TypeScript 类型检查通过。
- Desktop cargo check 通过。新增真实测试同时完成 Core 编译。
- 相关 Rust rustfmt、TS/TSX Prettier 和 git diff --check 通过。Python 脚本由真实 Blender 实际执行验证。
- 有效行检查：1170 个源文件，17 个未修改历史超限文件，0 项违规；未修改基线。

生产 ObjectScenePreview 组件使用本轮真实 Blender 收据和 PNG 在 Chromium 中回放：点击渲染、显示完成图像、刷新页面后恢复；PNG naturalWidth 为 1280，无 Godot 交互按钮，控制台无错误或警告。这个浏览器夹具使用 localStorage 模拟 API，未宣称完成原生 Desktop 端到端验收。此次真实引擎验证针对对象版本；尝试入口沿用共享适配器并通过相邻回归，未单独执行真实 Blender 尝试渲染。

## 证据文件

- [保存相机图像](../output/blender-proof/blender-camera.png)
- [自动相机图像](../output/blender-proof/blender-automatic.png)
- [保存相机收据](../output/blender-proof/blender-camera.json)
- [自动相机收据](../output/blender-proof/blender-automatic.json)
- [生产组件重开截图](../output/playwright/blender/reopened.png)
- [真实 Blender 最终日志](../output/blender-real-final.log)
- [Core 相邻日志](../output/blender-adjacent.log)
- [前端回归日志](../output/blender-ui-tests.log)
- [类型检查](../output/blender-typecheck.log)
- [Desktop 检查](../output/blender-check.log)
- [结构检查](../output/blender-structure.log)

## 边界与剩余任务

Workbench 显示实体几何与材质基础色，不还原最终节点材质、灯光、合成或序列编辑效果。Blender 交互相机、点选框选、帧存档反馈及拓扑重定位仍未接入；UI 隐藏相应入口，Core 拒绝 Blender 交互预览请求。自动相机当前基于可渲染网格对象边界，复杂实例、动画与修改器场景尚需后续验证。

依赖检查发生在打开 .blend 后、渲染前；外部或缺失依赖不能产生已接受预览，但此机制不构成操作系统文件访问沙箱。绝对路径引用若不属于恢复快照会明确失败；可使用快照内相对引用或打包资源。只处理可信文件。

本轮关闭 F8.1 中冻结 Blender 实体显示子流程，F8.1-F8.5 和 F9 均保持未勾选。没有运行完整原生或发布验收，没有构建交付 EXE，没有修改版本号、提交或推送。全计划仍未完成。
