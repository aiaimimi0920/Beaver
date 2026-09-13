# 轻量桌面架构纠正

日期：2026-09-07。

## 结论

用户不接受 200 MB 以上的编排应用，这是合理的产品约束。此前以 Electron 的固定成本解释体积，只解释了原因，没有解决错误选型。Beaver 没有必须携带 Electron/Chromium 的业务理由。

后续桌面架构方向调整为与 Loom、Hook 一致的 Tauri 2 / Rust 原生宿主，保留适用的前端交互，不继续把完整 Electron 当作最终发布方案。

## 当前仓库核验

| 项目 | 实际源码证据 | 实测 EXE 样本 |
| --- | --- | ---: |
| Loom | `Loom/apps/desktop/package.json` 使用 React 与 Tauri 2；`apps/desktop/src-tauri/tauri.conf.json` 定义桌面宿主；主工作区为 Rust | `Neuro/release/Loom/loom-live-unit-native-parity-20260906-r2/Loom.exe`：9.84 MiB |
| Hook | `Hook/src-tauri/tauri.conf.json` 和 `package.json` 明确使用 Tauri | `Neuro/release/Hook/live-contained-preview-20260907-r1/V0.2.29/portable/hook.exe`：7.62 MiB |
| Beaver | `scripts/package.mjs` 整体复制 Electron 运行时，再更换品牌资源 | 当前 `Beaver.exe`：234.80 MiB；完整发布目录：379.29 MiB |

这些 EXE 样本不能当作 Loom/Hook 的完整部署体积：Loom 还有 daemon 与能力插件，必须另外统计。它们仍足以说明，当前 UI 并不需要一个 234 MB 的浏览器宿主。

## 为什么最初用了 Electron，为什么现在应该更换

**是的，针对 Beaver 的产品定位，更换为 Tauri 2 + Rust 更合适。最初选型过度偏向快速实现，而没有把轻量发布和与 Loom、Hook 的技术一致性作为前置约束，这是选型上的不足，不是业务必然需要 200 MB。**

从现有代码结构可以看出 Electron 路线的直接便利：React 界面和 TypeScript 后端共用工具链，Node 主进程可以处理文件、子进程和通信，Electron 提供窗口、托盘及桌面集成。它降低了早期把交互原型连接到真实功能的成本。不过，这说明的是该路线的工程便利，并不是有一份经过充分比较的最初决策记录；不能事后把这些便利包装成已完成过严谨评估。

问题在于，这条路线把完整浏览器及 Node 运行时变成每份应用的固定发布成本。Beaver 是否执行重计算，与这笔固定成本不是一回事。即使主要负责组织文件和驱动 Codex，Electron 的运行时也不会因此缩小。

对 Beaver 而言，任务、会话、文件归档、人工修订、冲突保护和环境安装才是产品能力；它们没有要求必须使用 Electron。现有 React UI 可以保留，桌面宿主和本地服务改为 Rust，Windows 页面渲染使用共享 WebView2。这样更符合轻量安装的目标，也更接近现有 Loom、Hook 的方向。不是去掉 Web UI，更不是让应用只能联网使用。

代价必须明确：现有 Node/Electron 后端不能原样直接在 Tauri 中运行。凭据、数据库、任务进程、恢复、回退、素材协议和平台安装逻辑都要迁移并验证；系统 WebView 的跨平台差异也需要测试。对零环境用户，应检测并安装缺失的 WebView2，而不是宣称没有运行时依赖。

因此，正确做法不是为了一个小 EXE 丢掉功能，也不是隐藏一个 Electron/Node 后台，而是保留前端成果、逐项迁移后端，并以完整发布载荷和功能等价一起验收。当前源码已经存在 `native/core` 与 `native/desktop`，但默认 `start`、`build`、`package` 仍是旧 Electron 路线；不能把已有原生骨架当成迁移完成。

## 必须解决根本原因（实施约束）

- 不把删除 source map、多语言包或压缩 ZIP 当作架构问题已经解决。
- 不把 Electron 改名为后台进程，也不藏到第一次启动时下载。
- 不为保留现有后端而默默捆绑另一个大型 Node 运行时，然后只汇报一个小启动 EXE。
- Windows 使用系统 WebView2；缺少运行时时，通过明确的检测和安装完成零环境启动，依赖下载量与磁盘占用必须单独告知。
- Codex、Godot、Blender 等创作工具继续按需安装，和 Beaver 自身体积分开统计，不隐瞒总体环境成本。

## 保留成果与迁移边界

现有 React 界面、图标、项目规划语义和任务交互可以作为迁移基础。不能只换窗口：当前 Electron 主进程及 Node 核心使用的 SQLite、凭据加密、文件操作、进程控制、Codex 通信、任务日志和回退能力必须逐项替换或迁移。

已有数据和旧发布版本保留。不得为减小体积丢掉并发隔离、崩溃恢复、人工编辑保护、托盘后台运行和导出能力。Loom、Hook 仅作为只读参考，不修改这两个项目来迁就 Beaver。

## 体积验收要求

以 **Beaver 完整未压缩发布载荷不超过 50 MB（50,000,000 字节）** 为新的工程验收目标；这是约束，不是已达到的结果。

计入主程序、Beaver 自有后台、必需 DLL、界面、模板及随包资源。不能只测 EXE，也不能拿压缩包大小代替安装后的大小。系统共享 WebView2 和独立安装的创作工具分别报告。

同时重新验证：启动与窗口、托盘隐藏/退出、凭据读写、已有数据、任务及子任务、人工补充、并发冲突、回退、资料和素材预览、Godot/Blender 检测、真实游戏导出。

## 当前状态

已经核实差异并纠正后续架构方向；尚未完成 Tauri 迁移，也没有声称现在的 Beaver 已低于 50 MB。0.1.19 是功能对照基线，不是符合新体积约束的最终交付。
