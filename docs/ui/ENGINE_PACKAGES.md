# 引擎能力功能包目录

版本：0.1.8。定位：功能包的选择、含义与未来交付规划，不是 68 个引擎模块已经开发完成。

## 参考范围

核对 [City Game Studio 官方翻译的固定版本](https://github.com/binogure-studio/city-game-studio-i18n/blob/9a23d7a097df1f9560afbb536afb6f051ed30d38/languages/zh.yml#L1297-L1364)：`LABEL_GAME_ENGINE_*` 中的 68 项实际能力，覆盖原文件第 1297–1364 行。全部保留对应 `sourceKey`；Beaver 的名称与说明按实际游戏制作语义整理。

排除 `LABEL_GAME_ENGINE`、`LABEL_GAME_ENGINE_LIST`、`LABEL_GAME_ENGINE_TITLE`、`LABEL_GAME_ENGINE_DESCRIPTION` 等标题与提示，以及更新、删除、捐款、授权出售等经营游戏动作。它们不是可以装进用户游戏的能力。没有复制源游戏代码、美术、整份翻译或经营数值；授权核实边界延续[项目规划契约](PROJECT_BLUEPRINT.md#8-参考与可追溯性)。

## 交互与真实性

- “功能块”工作区新增两个页签：**功能包规划**与**源码包**。
- 功能包规划可浏览全部 78 项：本次 68 个能力规划包，加原有 10 个附带源码的包。每行标明“规划”或“源码”，不是“已安装”。
- 创建的“功能规划”、项目设置的“功能规划”和功能包工作区共用同一选择器；已有项目同样可以后加规划。
- 可按名称、用途、ID 或来源键搜索，按分组与来源筛选，也可只看已选。空结果明确显示没有匹配项。
- “全选当前”只追加当前筛选结果，保留其他选择；“清空”清空整个规划选择；“加入推荐”仍推荐已有源码包，不擅自替用户选新引擎能力。
- 选中功能包或点“说明”，在列表下方展示它的含义与边界。列表有内部滚动，避免把 78 项展开成整页卡片墙。正文使用通用颜色与字体，不新增主题。
- 创建时选择会随创建保存；已有项目编辑后需点“保存规划”。两个页面共用按项目隔离的草稿；导航和切换到源码页不会丢失已选项。
- 源码包页保留原有 10 项真实“接入 / 更新”动作。68 个新能力没有安装、接入或运行按钮。
- 即使直接调用 `feature.add`，`engine-` 标识也会在任务入口被明确拒绝，不回退为执行提示词，不创建任务。

## 全部 68 项

以下“键”省略公共前缀 `LABEL_GAME_ENGINE_`。实际完整键和逐项原创建议说明在 `src/shared/engine-packages.ts`，UI 显示该完整键，便于与来源逐项核对。

| 分组 | 数量 | 键与 Beaver 名称 |
| --- | --- | --- |
| 世界与玩法 | 4 | `OPEN_WORLD` 开放世界；`SANDBOX` 沙盒玩法；`MAP_EDITOR` 游戏内地图编辑；`MOD_SUPPORT` 玩家模组支持 |
| 输入与识别 | 7 | `GESTURE_CONTROL` 高级动作识别；`MOTION_CONTROL` 基础体感输入；`FACIAL_RECOGNITION` 面部识别；`AUDIO_RECOGNITION` 语音识别输入；`JOYSTICK` 摇杆输入；`GAMEPAD` 手柄输入；`MOUSE` 鼠标输入 |
| 图形与显示 | 15 | `HIGH_DEF_DISPLAY` 高清显示适配；`ULTRA_HIGH_DEF_DISPLAY` 超高清显示适配；`AR_32BITS` 增强现实；`2D_1BITCOLOR`、`2D_8BITCOLOR`、`2D_16BITCOLOR`、`2D_24BITCOLOR`、`2D_32BITCOLOR` 五种 2D 色彩表现；`3D_1BITCOLOR`、`3D_8BITCOLOR`、`3D_16BITCOLOR`、`3D_24BITCOLOR`、`3D_32BITCOLOR` 五种 3D 色彩表现；`VR_32BITCOLOR` VR 图形；`VOXEL_32BITCOLOR` 体素图形 |
| 网络与服务 | 6 | `SIMPLE_ANTICHEAT` 基础反作弊；`AI_POWERED_ANTICHEAT` AI 辅助反作弊；`CROSS_PLAY` 跨平台联机；`LOW_LATENCY_SERVER` 低延迟服务；`DEDICATED_SERVER` 专用游戏服务器；`SOCIAL_NETWORK_INTEGRATION` 社交平台连接 |
| 角色与难度 | 6 | `AI_POWERED_DIFFICULTY` AI 辅助难度；`REALISTIC_CHARACTER` 写实角色；`PREBUILT_CHARACTER` 预设角色；`CUSTOMIZABLE_CHARACTER` 角色自定义；`DIFFICULTY` 难度选项；`DYNAMIC_DIFFICULTY` 动态难度 |
| 对话与叙事 | 3 | `BASIC_DIALOG` 基础对话；`INTERACTIVE_DIALOG` 分支交互对话；`AI_GENERATED_DIALOG` AI 对话生成 |
| 声音与音乐 | 7 | `MONO_SOUND` 单声道输出；`MIDI_SOUND` MIDI 音源；`STEREO_SOUND` 立体声输出；`51_SOUND` 5.1 环绕声；`BASIC_MUSIC` 基础配乐；`ORCHESTRAL_MUSIC` 管弦乐配乐；`INTERACTIVE_MUSIC` 交互式配乐 |
| 存档与排行 | 5 | `PASSWORD_SAVE` 进度口令；`BASIC_SAVE` 本地存档；`CLOUD_SAVE` 云端存档；`LEADERBOARD` 本地排行榜；`CLOUD_LEADERBOARD` 在线排行榜 |
| 物理模拟 | 3 | `BASIC_PHYSICS` 基础物理；`ADVANCED_PHYSICS` 复杂物理交互；`REALISTIC_PHYSICS` 拟真物理 |
| 动画与演出 | 6 | `SIMPLE_ANIMATION` 基础动画；`DYNAMIC_ANIMATION` 动态动画；`AI_BASED_ANIMATION` AI 辅助动画；`SIMPLE_CINEMATIC` 基础过场；`ADVANCED_CINEMATIC` 复杂过场；`REALISTIC_CINEMATIC` 写实过场 |
| 引导与加载 | 4 | `BASIC_TUTORIEL` 基础教程；`INTERACTIVE_TUTORIEL` 交互式教程；`LOADING_SCREEN` 加载界面；`BACKGROUND_LOADING` 后台加载 |
| 商业化 | 2 | `ADS` 游戏内广告；`IAP` 游戏内购买 |

`TUTORIEL` 是上游键的实际拼写，保留而不是擅自“修正”导致失去映射。列表的 68 项逐一作为独立功能包出现；表格中合并列举色彩变体只为压缩文档长度。

## 容易混淆的含义

1. **跨平台联机不是多系统导出。** 它要求不同平台共享会话、协议和身份；当前没有承诺 Steam、Epic 或其他渠道之间已打通。
2. **进度口令不是账号密码。** 它是通过编码恢复游戏进度的机制。
3. **MIDI 不是声道等级。** 它与立体声、环绕声可以在方案中共存；管弦乐也不是自动“升级”基础配乐的品质档。
4. **色深是表现方向。** 单色、8/16/24/32 位色的条目可用于讨论不同场景或艺术目标，不直接切换 Godot 底层渲染格式。AR、VR、体素需要另外的设备或实现验证。
5. **AI 功能不等于 Beaver 的 AI 服务。** AI 对话、动画、难度或反作弊分别讨论游戏内容与运行机制；在制作期还是运行时使用、费用、安全与回退仍需确定。
6. **游戏内广告不等于发行宣传。** 前者是在游戏里展示广告的商业化机制；发行准备的广告宣传则是推广游戏。IAP 不代表已接入支付。
7. **基础对话 / 本地存档不自动绑定旧源码。** 规划项表达需要的能力，原有 `dialogue` / `save-slot` 是可供适配的源码包，两者不能伪装成同一个已实现版本。
8. **复杂程度不自动决定互斥关系。** 翻译只提供名称，没有足够依据证明基础 / 高级 / 拟真是互斥或必需依赖；本轮可同时记录。网络能力也不会擅自改动已有在线开关或服务器规划。

## 数据和执行边界

新包使用 `engine-<source-suffix>` 标识，例如 `engine-cloud-save`，保留来源键为 `LABEL_GAME_ENGINE_CLOUD_SAVE`。元数据类型是 `EnginePackage.kind = "planning"`，没有虚构版本号、源码目录、采用基线或安装状态。

选择仍保存为版本 1 blueprint 中的 `plannedFeatures`，总上限沿用 100；全部 78 项可一次保存。不改写旧选择，不强制迁移 SQLite 或 Godot 工程。若已有选择不在当前目录中，显示可移除的 ID，而不是静默删除。

68 项不放进 `resources/features`，也不出现在真实 `state.features` 中。运行时包读取、功能块快照、采用版本与更新流程仍仅面向已有源码。没有新增 AI 调用、服务器部署、广告 SDK、支付、账号登录、玩家信息收集或生物信息处理。

## 验收与下一步

当前验收要求：68 个键不遗漏且不重复，10 个源码包不被污染；三个选择入口可用；搜索、分组、来源、已选、全选和说明可交互；保存/重启保留；跨页面草稿一致；误调用接入被拒绝；游戏文件和任务数不变；默认及最低窗口仍可用。

下一步在用户确认目录与交互后，逐包确定交付文件、实现方式、依赖、兼容性、支持的 Godot 版本和验证标准。不能只给元数据补一个版本号就把它升级为可执行源码包。
