# 创作、对象与制造 UI 预览与审阅说明

更新：2026-09-15。状态：可交互 UI mock，按用户反馈持续调整。

本轮按成品界面的视觉与信息密度制作创作、对象、制造三个面板，供用户直接体验并提出修改意见。新面板的真实业务功能在界面确认后开发；mock 操作不执行任务、不修改项目、不调用 Codex、Godot、Blender 或第三方插件。

## 三层创作入口

三个入口在开发过程中持续存在，用户根据目标自由选择介入深度，不要求依次手动操作三个页面。

- **创作**：任务与目标入口。用户提出一系列需求，由 AI 拆分并推进任务；用户无需了解对象的划分、组成或实现方式，也可以介入任务拆分与调整。
- **对象**：面向具体游戏内容的修改入口。了解游戏制作常识的用户可以指定模型、音乐等对象的改进目标，追加修改、管理迭代，并在需要时深入制造。
- **制造**：原“对象创作”页面，展示单个对象任务的具体制作流程、阶段、工具、交付和验收。用户可以参与实现细节；深入调整可能需要 Blender 等工具知识，具有更高操作自由度。

粗任务仍会拆分为对象任务和具体制作任务；用户只在关心的层次介入，后台流程仍完整推进。各层任务仍归入任务泳道，对象面板仍包含无任务绑定的对象，单对象修改仍串行排队。

设计依据：[对象创作框架开发设计](../OBJECT-CREATION-FRAMEWORK-DESIGN.md)。界面中的六个阶段仅用于展示流程布局，具体 NPR 小阶段与工具链尚未开发。

## 启动预览

桌面版本入口：[Beaver.exe，内部版本 0.1.19.30](../../release/Beaver-native-0.1.19.30-win32-x64/Beaver.exe)。保留该目录中的配套文件，双击 exe 后默认进入“界面预览”，其上方包含“创作 / 对象 / 制造”三个可切换的面板。

左侧“对象”下方增加“制造”入口，点击后在右侧完整显示原“对象创作”子页，省去预览面板的二级导航。对象与制造入口共享当前对象上下文，跨页链接同步左侧高亮；“界面预览”仍可独立浏览三个面板。这些入口复用现有 mock 数据和交互。

预览嵌入现有 `DesktopShell`，保留原生标题栏、窗口控制、项目选择和完整导航：“创作”“对象”“制造”“资料”“测试与画面”“游戏”“创作环境”“设置”。“素材”入口已由“对象”替代；创作、资料等原有页面继续使用现有业务功能。顶部选择的真实项目与 mock 数据互相独立，底部保留提示空间。应用外壳会读取本地状态和探测工具，mock 按钮不会触发制作。“界面预览”用于本轮审阅，不表示最终要额外保留一个产品面板。

可选的独立浏览器开发预览，在 Beaver 仓库根目录运行：

```powershell
npm run preview:objects
```

打开 <http://127.0.0.1:4175/?preview=objects>。服务仅监听本机，终端保持运行即可浏览，按 `Ctrl+C` 停止。这是三个新面板的独立开发视图，不包含原有桌面导航；审阅完整应用请使用 exe。在原生环境中，此查询参数不会绕过桌面外壳。

“重置演示”恢复初始界面；刷新页面也会清除临时交互状态。预览不会保存任务、提示词、选区或项目修改。

## 三个面板

| 面板 | 可以审阅的内容                                                             | 演示交互                                                             |
| ---- | -------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| 创作 | 粗修、中修、精修的父子关系，执行泳道，排队、依赖、失败与待验收状态         | 层级筛选、查看任务、跳转父子任务及所属对象和制作阶段                 |
| 对象 | 按对象组织的封面、组成内容、独立引用、版本和修改队列，包含未绑定任务的对象 | 搜索筛选、选中对象、查看版本、跳转任务或制造、输入修改要求           |
| 制造 | 当前对象、线性阶段、版本与快照、画面预览、交付、验收与记录                 | 切换对象、阶段和版本，框选画面、输入反馈，查看相关精修任务及审批演示 |

演示项目为“放课后”，包含 6 个对象和 10 个任务。对象覆盖 NPR 角色、教室、演出场景、代码控制器、独立材质和 UI；任务面板同时包含已完成任务和 8 个尚未完成的任务。

角色对象包含模型、颜色贴图、法线贴图和着色器。教室引用独立材质的固定版本，用来展示“对象内部组成”和“引用其他独立对象”的区别。材质与 UI 对象没有绑定任务，仍可在对象面板中浏览。

## 建议审阅路径

1. 在默认的对象面板查看“澪 · NPR 角色”，比较封面、对象组成、引用、版本与修改入口的位置。
2. 在“界面预览”中打开创作面板，查看“让澪在午后的教室里跳舞”的粗修任务，以及角色、教室等中修任务和各自精修任务。打开任务详情，沿父子关系与对象链接跳转。
3. 进入角色的制造面板，查看当前阶段的交付和验收。切换到历史 `r02`，再返回当前版本，观察历史、当前、尚未开始的阶段如何区分。
4. 在画面上选择“框选”，拖动框出一个区域，输入修改要求并提交。底部反馈包含对象、版本和选区范围，同时明确未创建真实任务。
5. 查看角色的排队修改任务，以及教室的失败精修任务 `T-204`。教室检查项显示“待修正”，未就绪的审批按钮不可用。
6. 使用分类图标或标签筛选对象，或搜索不存在的名字，检查空状态、清除筛选及开始修改入口。

优先反馈面板职责是否清楚、任务层级是否易读、对象与阶段之间能否快速定位、画面占比是否合适，以及哪些信息或操作应该移动、删减或补充。

## 测试数据边界

封面和创作画面采用手绘 SVG 演示插画，界面内有相应标识；这些图像没有经过真实模型渲染。等待依赖的演出场景显示“概念示意 · 尚未生成”。历史版本用于展示版本浏览与状态区分，不代表已有真实阶段备份或版本对应的渲染产物。

面板切换、搜索筛选、任务详情、跨面板跳转、版本选择和区域框选具有本地交互。“界面预览”内的新建任务、调整拆分、审批等入口显示演示弹窗或本地反馈；确认后不会推进固定测试任务的状态，也不会写入项目。切换版本会清除原有选区，历史和未来阶段不会误显示为可审批的当前交付。桌面侧栏中的“设置”等原有页面仍保留真实功能。

对象串行队列、阶段线性顺序和失败恢复在本轮展示为测试数据与界面状态。实际锁、排队调度、版本发布、任务执行、插件安装和工具调用仍需后续实现。

## 截图

原生 exe 截图，包含完整桌面外壳：

- [对象面板](../../output/playwright/native-ui-preview/objects.png)：原有侧栏、演示项目标识、对象封面与内容。
- [任务面板](../../output/playwright/native-ui-preview/tasks.png)：任务泳道与层级。
- [对象创作](../../output/playwright/native-ui-preview/creation.png)：阶段、画面与交付检查。
- [框选反馈](../../output/playwright/native-ui-preview/region-feedback.png)：原生窗口内的指针选区和演示反馈。
- 原有页面：[资料](../../output/playwright/native-ui-preview/legacy-2.png)、[测试与画面](../../output/playwright/native-ui-preview/legacy-4.png)、[游戏](../../output/playwright/native-ui-preview/legacy-5.png)、[创作环境](../../output/playwright/native-ui-preview/legacy-6.png)、[设置](../../output/playwright/native-ui-preview/legacy-7.png)。

此前独立浏览器预览的布局截图：

- [对象面板，1600 × 1000](../../output/playwright/object-ui-preview/objects.png)：对象封面与右侧信息区。
- [任务面板，1600 × 1000](../../output/playwright/object-ui-preview/tasks.png)：泳道、层级与执行状态。
- [对象创作，1600 × 1000](../../output/playwright/object-ui-preview/creation.png)：阶段、完整角色预览与交付检查。
- [框选反馈](../../output/playwright/object-ui-preview/region-feedback.png)：真实指针拖动选区及提示词输入。
- [历史版本](../../output/playwright/object-ui-preview/history.png)：历史快照的浏览状态。
- [对象创作，1280 × 800](../../output/playwright/object-ui-preview/creation-1280.png)：中等窗口布局。
- [对象面板，1024 × 768](../../output/playwright/object-ui-preview/objects-1024.png)：较窄窗口布局。
- [对象创作，1024 × 768](../../output/playwright/object-ui-preview/creation-1024.png)：教室预览与阶段检查。

截图和检查日志属于本地 `output/` 产物，重新检出仓库时可能不存在；可以启动预览重新查看。

## 工具栏精简与对象入口（0.1.19.5）

移除滑动条两侧图标和“对象 / 数量 / 添加对象”整行。标签按钮紧随分类图标，仅显示标签图标和下拉箭头；下拉内容采用换行排列的复选标签，移除标题、说明和数量。搜索框复用“素材”页的原生文本输入样式及即时过滤交互。

搜索框右侧提供“导入对象”和“生成对象”两个纯图标按钮。导入弹窗可选择外部文件或文件夹，也可使用示例资源，并填写对象名称、类型、标签和说明；文件选择只提取名称，不读取内容或写入项目。生成弹窗包含内容类型、对象名称、任务描述和验收要求，提交入口显示“添加任务”，本轮仍只演示反馈。

2026-09-15 补充：[后续导入与项目存储规范](../OBJECT-IMPORT-AND-PROJECT-STORAGE.md) 要求支持文件/文件夹、已登记项目、第三方 Beaver 项目三种来源，并将项目信息统一放入根目录 `.beaver`。该要求仅已入文档，本版弹窗及存储代码尚未接入。

定向 Prettier、TypeScript 检查、有效行检查、原生编译与打包通过。仅检查本轮控件：环境标签筛选得到 2 个对象，搜索“橡木”得到 1 个对象，导入和生成弹窗均可打开；生成提交显示 mock 反馈。已查看四张原生截图，弹窗内容与按钮完整可见。未运行全流程测试，`runtimeVerified` 保持 `false`。

已关闭旧 Beaver 并启动 `0.1.19.5`，确认仅有该版本的一个实例；检查后停留在左侧“对象”页，恢复全部 6 个对象，无筛选、搜索或弹窗。自动化连接已断开，应用保留运行供审阅。

截图：[对象工具栏](../../output/playwright/object-toolbar-v5/objects.png)、[复选标签](../../output/playwright/object-toolbar-v5/tags.png)、[导入对象](../../output/playwright/object-toolbar-v5/import.png)、[生成对象](../../output/playwright/object-toolbar-v5/generate.png)。证据：[编译](../../output/native-ui-object-toolbar-v5/build.log)、[打包](../../output/native-ui-object-toolbar-v5/package.log)、[控件交互](../../output/native-ui-object-toolbar-v5/controls.log)、[启动记录](../../output/native-ui-object-toolbar-v5/start-proof.json)。

## 对象工具栏（0.1.19.4）

对象页首行替换为工具栏，移除该页的演示项目标题、预览徽标和重置按钮。分类依次为全部、图像、音频、模型、场景、翻译、脚本、其他，使用八种独立于主题变量的图标颜色；悬停显示分类名。原列表内的文本筛选与搜索行已合并到首行。

滑动条调整缩略图宽度（160–360 像素）。标签下拉框支持多选：不选时不限制标签，多选时匹配任一所选标签；分类、标签和搜索条件同时生效。“导入对象”打开文件夹弹窗，路径、文件夹和包含内容均为测试数据，确认只显示本地反馈。

定向 Prettier、TypeScript 检查及原生编译通过。有效行检查为 534 个文件、18 个未变更历史超限文件、0 项违规；工具栏组件为 181 个有效行，导入弹窗为 111 行，对象面板为 294 行，工具栏样式为 305 行。原生截图发现标签按钮继承旧 `details` 的间距与边框，已在工具栏局部覆盖并重新编译，修正版保存到 `Beaver-native-0.1.19.4-win32-x64-ui2`，保留首次编译目录。

已关闭旧 Beaver 并启动修正版，确认仅有该交付路径的一个实例。局部交互确认：模型分类显示 1 个对象、环境标签显示 2 个对象、滑动条将卡片宽度从 290 调到 160 像素，导入弹窗正常展示。完成后恢复全部 6 个对象及 290 像素宽度，停留在左侧“对象”页。未执行全流程测试或游戏创作验收，`runtimeVerified` 保持 `false`。

截图：[首行工具栏](../../output/playwright/object-toolbar/native-final.png)、[标签下拉框](../../output/playwright/object-toolbar/tags.png)、[导入文件夹弹窗](../../output/playwright/object-toolbar/import.png)。证据：[最终构建](../../output/native-ui-object-toolbar/build-alignment.log)、[最终打包](../../output/native-ui-object-toolbar/package-alignment.log)、[启动记录](../../output/native-ui-object-toolbar/start-proof-alignment.json)、[控件交互](../../output/native-ui-object-toolbar/controls-final.log)、[名称搜索](../../output/native-ui-object-toolbar/search-final.log)。

## 对象侧栏入口（0.1.19.3）

左侧“对象”直接打开完整对象页，隐藏预览二级导航，保留原有页面。新增入口仍使用测试数据。

定向 Prettier、TypeScript 和有效行检查通过；`DesktopShell.tsx` 为 315 个有效行，`App.tsx` 为 423 行，`ObjectFrameworkPreview.tsx` 为 234 行，职责分别为外壳导航、页面路由和预览状态。

新版 exe 已编译、写入 `0.1.19.3` 版本并通过打包验证。构建日志采集时，PowerShell 将 Cargo 的 stderr 进度消息报告为 `NativeCommandError`；编译及后续版本写入已完成，打包脚本也已验证 exe 和前端版本一致。此次没有重复编译或扩大测试范围。

关闭旧版后已启动新版本，确认仅有该交付路径的 Beaver 进程。点击“对象”并查看[原生界面截图](../../output/playwright/objects-tab/native.png)，确认右侧显示对象列表和详情。未运行全流程测试或页面遍历验收，`runtimeVerified` 保持 `false`。

证据：[构建日志](../../output/native-ui-objects-tab/build.log)、[打包日志](../../output/native-ui-objects-tab/package.log)、[对象入口点击](../../output/native-ui-objects-tab/open-objects.log)。

## 首版验证（0.1.19.2）

原生集成与本次 exe 交付已执行以下定向检查：

- `npm run typecheck`：通过。
- `npm run build:native`：通过，包含结构检查、前端构建、Cargo release 编译与 Windows 版本写入。结构检查扫描 531 个文件，18 个未变更历史超限文件，0 项违规；未修改历史基线。
- 打包到新目录 `release/Beaver-native-0.1.19.2-win32-x64`，保留旧版本；exe 文件版本与产品版本均为 `0.1.19.2`。
- 使用 `scripts/start-fresh-native-test.ps1` 关闭旧 Beaver 并启动交付目录中的 exe，创建独立数据目录、WebView 目录及空白项目。交互检查前后均为 1 个项目、0 个真实任务。
- 在真实原生 WebView 中逐一打开“创作”“资料”“素材”“测试与画面”“游戏”“创作环境”“设置”，确认对应原有页面挂载、导航高亮正确；设置的返回工作区操作通过。
- 原生预览交互通过：三个面板切换、指针框选及提示词反馈、对象搜索和空结果、重置、演示弹窗。侧栏收起/展开、窗口最大化/还原通过；最终预览无文档级溢出。
- 原生交互期间没有页面异常；控制台为 0 条错误、0 条警告，另有 12 条 Chromium 密码字段不在 form 内的 verbose 提示，来自原有设置页。
- 修改文件的定向 Prettier 检查通过，源码和文档保持 UTF-8 无 BOM。

原生证据：[构建日志](../../output/native-ui-preview/build.log)、[打包日志](../../output/native-ui-preview/package.log)、[启动证据](../../output/validation/fresh-round-20260914-142219-c8e403dba4294b349a1171592fd4a058/start-proof.json)、[定向交互脚本](../../output/native-ui-preview/inspect-native.ts)、[交互运行日志](../../output/native-ui-preview/interaction.log)、[最终界面状态](../../output/native-ui-preview/final-state.log)和[控制台检查](../../output/native-ui-preview/console.log)。

交互脚本曾将空白项目资料页的“选择资料”误判为空页面，并使用了未包含导航副标题的精确匹配。已改为等待实际页面根节点，并匹配含副标题的导航名称，修正脚本后检查通过；未因此修改原有资料功能。

此前独立浏览器预览还检查了层级筛选、任务和对象跳转、版本链接、历史及未来阶段、失败状态、键盘关闭和框选反馈，并查看 1600 × 1000、1280 × 800、1024 × 768 的相关布局；该轮静态预览的网络记录中未出现非静态请求。这一网络结论不适用于保留真实业务页面的原生应用。此前日志保存在 [typecheck.log](../../output/object-ui-preview/typecheck.log)、[effective-lines.log](../../output/object-ui-preview/effective-lines.log) 和 [format.log](../../output/object-ui-preview/format.log)。

本次 exe 的 SHA-256：`CEFC2922BD83706AAF1696CA1E3303644D119274834C2B97B37667A5A4D9DC54`。

本轮未运行全流程测试或游戏创作验收；打包清单中的 `runtimeVerified` 保持 `false`，不将此次 UI 定向检查标记为完整运行验收。内部版本递增为 `0.1.19.2`，公开三位版本仍为 `0.1.19`。上述验证不代表新对象框架的业务功能已完成。

## 文件职责

- [预览入口与本地状态](../../src/ui/object-preview/ObjectFrameworkPreview.tsx)：三面板导航、演示弹窗和重置。
- [测试对象](../../src/ui/object-preview/mock-objects.ts)与[测试任务](../../src/ui/object-preview/mock-tasks.ts)：集中管理固定数据。
- [对象面板](../../src/ui/object-preview/ObjectLibrary.tsx)、[任务泳道](../../src/ui/object-preview/TaskLanes.tsx)、[对象创作](../../src/ui/object-preview/CreationView.tsx)：各自面板的展示职责。
- [阶段信息](../../src/ui/object-preview/StageInspector.tsx)与[区域预览](../../src/ui/object-preview/RegionPreview.tsx)：阶段证据和本地反馈交互。
- [启动脚本](../../scripts/preview-object-ui.ts)：使用 esbuild 构建并监听预览文件。
- [应用入口](../../src/ui/main.tsx)：仅在无原生桥接时，由 `?preview=objects` 选择独立 mock；原生环境始终挂载完整应用。
- [桌面路由](../../src/ui/App.tsx)与[桌面外壳](../../src/ui/DesktopShell.tsx)：默认预览入口与原有页面导航。
- [嵌入样式](../../src/ui/object-preview/preview-embedded.css)：适配原生标题栏、侧栏及剩余窗口空间。

预览样式使用 `op-` 命名空间，并按壳层、对象、任务、创作和控件划分文件。新增文件均低于 500 个有效行；当前最大为 `preview-creation.css` 的 461 个有效行。
