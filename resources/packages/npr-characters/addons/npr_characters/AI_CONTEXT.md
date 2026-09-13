# NPR 人物制作上下文

适用于本目录的 NPR Characters 1.0，以及定制 Godot Forward+ 引擎 `db1af1e`。先遵循本文件和 `asset_contract.json`，再阅读实现。历史 shader 注释和变量名可能沿用捕获代码，不能单独作为语义依据。

## 1. 制作产物

每个新人物交付一个 `.glb` 或等价的 Godot PackedScene、13 个材质输入纹理、一个 `NPRCharacterMaterials` `.tres` 和一个 `NPRCharacterDefinition` `.tres`。纹理可以按需复用同一资源，例如暂时让暖 / 冷 Ramp 相同；所有必填字段仍要有值。

定义的字段：

| 字段 | 类型与默认值 | 填写规则 |
| --- | --- | --- |
| `model_scene` | PackedScene，空 | 导入后的整个纯模型场景，必填 |
| `body_path` | NodePath，`Body` | 相对模型根节点，指向身体 / 衣服 / 装备网格 |
| `face_path` | NodePath，`Face` | 指向面部网格 |
| `hair_path` | NodePath，`Hair` | 指向头发网格 |
| `material_set` | NPRCharacterMaterials，空 | 下表定义的纹理与局部轴；标准流程必填 |
| `use_source_materials` | bool，false | 新 glTF 保持 false；仅完整 authored Godot 材质链使用 true |
| `material_profile` | NPRMaterialProfile，新建默认值 | 8 个高光指数，默认全为 24；保留来源说明 |
| `display_height` | float，3.0 | 正数：统一缩放、水平居中并落地；0：保留原始尺寸和 pivot |

三个路径必须非空、互不相同、为子孙节点路径，不能用绝对路径或 `..`。节点可以嵌在 Skeleton3D 或其他 Node3D 下，名称本身不重要。

## 2. 模型与 Blender 工作步骤

1. 使用 Blender 创建人体、衣服和装备。为渲染角色准备 Body、Face、Hair 三个网格对象。Body 内可以保留互不连接的装备几何岛。合并对象后，把材质槽整理为每个对象恰好一个。
2. 给每个渲染角色制作 UV atlas，保留 UV1。Face 可额外制作 SDF UV2；默认先使用 UV1。明确设置平滑 / 硬边和法线，不依赖材质节点自动生成 NPR 法线。当前高光使用网格法线，不提供切线空间法线贴图入口。
3. 在绑定骨骼前整理坐标。导入 Godot 后以 Y 为高度、+Z 为面朝方向、+X 为正侧向，使用米或一致的等比例单位。根节点尽量为单位变换、脚底位于 Y=0。对已经绑定的模型，不要随意应用会改变 bind pose 的变换。
4. 标准新模型的脸与头发网格局部轴应同样满足 +Z 朝前。若导出工具产生了局部旋转，查看 Godot 中网格局部轴并填写下面的 axis 字段，不能根据 Blender 屏幕朝向猜测。
5. 三角化；导出 normals、UV、需要的 vertex colors、skin 和 blend shapes。使用 glTF 2.0 / GLB，导入 Godot 后检查三个绑定节点都是单 surface 的 ArrayMesh。导出后发生多材质拆 surface 时应修复原资产。
6. 骨骼使用 Godot 支持的标准 skin 数据。每个 skinned MeshInstance3D 必须能通过 `skeleton` NodePath 找到 Skeleton3D，且具备有效 Skin。支持标准 4 / 8 权重数据。BlendShape 应保留在 ArrayMesh；不能用自定义 vertex shader 位移代替可同步的骨骼 / BlendShape 变形。
7. 先用建议环境、曝光 1.0、MSAA 4x 验收正面、侧面、背面、左右主光、头部特写和动画姿态，然后再融入宿主后处理。

版本 1 要求三个角色都存在。额外网格会被拒绝；不要静默遗留未绑定的眼球、睫毛、牙齿或装备。按渲染需求将它们整理到对应角色的 atlas 和网格中。没有头发的角色需要后续扩展角色类型，不能伪装成已经支持任意拓扑。

## 3. 纹理字段

| 字段 | 类型 / 尺寸 | 实际用途与色彩空间 |
| --- | --- | --- |
| `body_base` | Texture2D，非空 | RGB 为 sRGB 底色，A 为现有发光控制输入；普通不发光像素可用 A=1，模板关闭额外发光 |
| `body_ilm` | Texture2D，非空 | 原始数据；G 为遮蔽 / 明暗控制，B 为高光阈值输入，A 为 8 槽编号；当前身体主流程不使用 R 调节 rim 宽度 |
| `body_lut` | Texture2D，**8 x 8** | 混合颜色 / 数据 LUT，禁止整体 sRGB 转换；具体行见下一节 |
| `body_ramp` | Texture2D，宽至少 2，高 16 | sRGB 暖色 Ramp atlas |
| `body_cool_ramp` | 同上 | sRGB 冷色 Ramp atlas |
| `face_base` | Texture2D，非空 | sRGB 脸部底色，和 face ILM / makeup 共用 UV1 atlas |
| `face_ilm` | Texture2D，非空 | 原始 RGBA 数据；R 为区域分类，G 为眼部 stencil 覆盖，B 为鼻线遮罩，A 为面部角度阈值 SDF |
| `face_map` | Texture2D，非空 | 原始 RGB 数据，依次控制腮红 / 化妆、唇色、脸部渐变；中性输入可全黑 |
| `face_ramp` | Texture2D，非空 | sRGB 距离 / 平面 LUT 颜色输入；其采样含义不能从字段名推成脸部明暗 Ramp |
| `hair_base` | Texture2D，非空 | sRGB RGB；A 控制眼区头发的透明混合模式，普通头发颜色 pass 仍为不透明 alpha=1 |
| `hair_ilm` | Texture2D，非空 | 原始数据；R 为高光带遮罩，G 控制明暗，B 控制高光形状，A 为 8 槽编号 |
| `hair_ramp` | 宽至少 2，高 16 | sRGB 暖色 Ramp atlas |
| `hair_cool_ramp` | 同上 | sRGB 冷色 Ramp atlas |

底色和数据贴图不要求彼此相同分辨率，但必须使用对齐的 UV atlas。建议用无损 RGBA PNG；不要对 ILM 和 LUT 使用 JPEG 或有损 GPU 压缩。关闭 normal-map 自动识别和 roughness 自动处理，保留 A 通道。shader 的 `source_color` 负责底色 / Ramp 的 sRGB 解码；ILM、face_map、body_lut 不能勾选成颜色输入再手动转换一遍。

**采样方向必须实际校验。** Body、Hair、Face 底色及常规遮罩在 shader 中使用 `(UV.x, 1-UV.y)`。Face SDF 的 A 通道通过局部光方向函数提供的 UV 采样，那里没有这个 Y 翻转，同时根据光在左右哪边镜像 X。制作 `face_ilm` 时，RGB 区域遮罩与 A 的图像行方向因此有不同采样约定。不要仅凭图像编辑器的上下方向判断；在导入后用带有不对称标记的测试 atlas 确认。UV2 仅替换 SDF 所用坐标；eye stencil 继续使用 UV1 的 G 通道。

身体 / 头发 Ramp 使用线性过滤、禁止 repeat，并禁止 mipmap 采样。对槽 `s`，采样 Y 为 `(2*s+1)/16`；在线性过滤下会落在对应两条相邻 texel 行之间。为每个槽填相同的两行，避免纵向混色。横向从暗到亮，0.15 附近也会被原生阴影读取。冷 / 暖两套 atlas 的槽布局必须一致。

ILM 槽编码使用 `s = fract(floor(A*8)/8)*8`。新资产推荐槽 s 写 `(s+0.5)/8`，s 为 0..7。A=1 会回绕槽 0；不要把 1 当作槽 7，也不要把该 alpha 当作透明度。A 的离散区域边界应留出纹理过滤的 padding。

## 4. Body LUT 的完整布局

每列对应一个槽 s=0..7，shader 用 `texelFetch` 读取，不做插值或 mipmap。必须提供完整 8 行，即使当前显示预设禁用了某些效果。颜色行的 RGB 按 sRGB 数值保存，shader 在读取后显式转线性；标量行原样读取。

| 行 y | 内容 |
| --- | --- |
| 0 | RGB 高光颜色，手动 sRGB 解码；A 当前不作为主要控制 |
| 1 | R 历史高光指数输入，G 高光羽化 / 粗糙度输入，B 高光强度；profile 启用时指数使用独立的 8 个 float |
| 2、3、4 | 当前身体入口未读取，保留为零 |
| 5 | RGB 内侧 rim shadow 颜色，手动 sRGB 解码 |
| 6 | R 内侧 rim shadow 宽度，G 羽化，B 额外 bloom 强度 |
| 7 | RGB 额外 bloom 颜色，手动 sRGB 解码 |

中性起点：第 0 行 RGB 约 0.6，第 1 行 `(1, 0.04, 0.5)`，第 5/6/7 行 RGB 为零；profile 指数全为 24。之后按身体、衣服和装备区分槽，并制作不同高光。LUT 用最近邻、repeat off，不要把整张纹理标为 source_color。当前 profile 的参数是美术选择，未声称等同原游戏。

## 5. 脸部 SDF 和眼部

`face_forward` 默认 `(0,0,1)`，`face_right` 默认 `(1,0,0)`，两者均为导入后的**网格局部空间**单位向量，互相正交。它们的顺序确定正负侧向；不要交换轴来修正一张左右画反的遮罩。头骨动画和整个角色旋转会经模型变换进入计算。

新模型使用 `atan(dot(L_local,right), dot(L_local,forward))` 得到水平角度，归一化阈值为 `abs(angle)/PI`，限制在 0.0001..0.9999。正面光阈值接近 0，侧光为 0.5，背光接近 1。负侧向光镜像 SDF 采样 X。A 大于当前角度阈值的区域较亮；`set_sdf_feather()` 控制阈值过渡。制作 SDF 时应设计鼻子、脸颊的过渡形状，不能用普通 AO 或法线图替代。

`sdf_on_uv2` 默认 false。只有确实提供 UV2，且 A 通道按该 UV2 排布时才能设 true。模型检查器会拒绝缺失 UV2 的配置。

ILM R 严格位于 `(0.1, 0.8)` 时判定为眼球区域，也会从刘海接触阴影中排除；推荐眼球 R=0.5，普通脸部 R=0，其他分类需按实际调色需求验证。G 严格大于 0.5 写眼部 stencil；推荐眼区 G=1，其余 G=0。G 控制眼区覆盖，与 R 的区域分类有关联但互不替代。B 是鼻线输入，中性为 0。不同通道必须独立绘制，禁止把全白 ILM 当作无效果默认图。

## 6. 头发和顶点数据

高光带由法线、视线、灯光和 ILM R/B 共同决定。R=0 关闭对应高光带，R=1 允许高光；B 较低时保留 ribbon，B 还参与另一条捕获高光形状。先以 G=0.5、B=0、合适的槽 A 建立中性 hair ILM，再绘制局部变化。

`hair_sheen_axis` 默认 `(0,0,1)`，为头发网格局部空间单位向量。它控制眼部头发的 reveal alpha，和高光带方向是两个控制过程。eye-hair pass 与非眼区 pass 必须成对，模块会创建并维护 stencil 顺序。不要手动用一个普通透明材质代替其中一个 pass。

没有特殊顶点遮罩时，顶点色使用线性 `(1,1,1,1)`，保持部件隐藏关闭。顶点色 R 会影响身体遮蔽强度；脸部 R/G/A 也参与眼部、唇线和发光控制。若启用分件隐藏，应按实现的 `int(color * 256) & mask` 编码并检查导出后的实际数值；全白 1.0 会变成 256，和 8-bit mask 相与为 0，不能用它表示启用隐藏时的“所有部件”。本版本的基础流程不要求分件隐藏。

标准流程不会给 shader 添加额外 vertex 位移。需要摆动、表情或轮廓同步变形时，使用骨骼和 BlendShape，使颜色、阴影、深度和 CPU 拾取可以读取同一份几何状态。

## 7. AI 自检与交付说明

在提交角色前，运行 `definition.validate()` 和实际实例化，确认 `actor.initialized` 为 true 且 `validation_errors` 为空。检查标准主光、左右光、背光、面部特写、眼区头发、描边、接触阴影及动画。角色姿态和 SDF 绘制质量需要看实际图像；仅通过字段检查不代表美术已经完成。

资源必须是自己制作或有权使用的内容。模块的程序人偶用于展示接线方式、GLB round trip 和 GPU 验证，其中的简单 SDF atlas 只是诊断阈值图。不要把程序人偶描述为完成了精细面部建模或作为游戏角色美术验收标准。

模型交付说明应记录：模型来源、尺寸、三个节点路径、导入后的局部轴、UV1/UV2 使用方式、13 个贴图字段到文件的映射、每个 material slot 的用途、骨骼 / BlendShape 范围、测试使用的引擎版本和截图。对未制作或未验收的部分明确标记。
