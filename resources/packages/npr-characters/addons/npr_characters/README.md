# NPR Characters 1.0

这个文件夹提供可移植的人物渲染运行时、材质模板和资产制作约定。目标是我们的定制 Godot Forward+ 引擎，已知匹配版本为 `db1af1e99a5025bfbf8b05ef27dd4a5e739ec371`。需要其中的 stencil、`post_light`、未抖动投影、几何读取和 TriangleMesh 扩展。

## 导入和试用

1. 将整个 `addons/npr_characters` 复制到目标项目的同名路径。保留 `.uid` 文件，忽略原项目的 `.godot` 缓存。路径是模块的资源命名空间，请保持不变。
2. 在定制引擎的编辑器中启用 **NPR Characters** 插件。它安装 `config/shader_globals.cfg` 中的着色器全局参数并保存 `project.godot`。出现同名异值或类型冲突时会停止安装，保留宿主设置；根据诊断处理冲突后重新启用。
3. 使用 Forward+。打开 `examples/mannequin.tscn` 并运行，可看到一个完全由程序生成、没有银狼资源依赖的测试人物。

自动化导入可在首次编辑器 import 前执行：

```powershell
& "<定制引擎目录>/godot.windows.editor.x86_64.exe" --headless --path "<项目目录>" --script res://addons/npr_characters/install.gd
& "<定制引擎目录>/godot.windows.editor.x86_64.exe" --headless --path "<项目目录>" --editor --import --quit
```

第一步成功打印 `NPR_INSTALL_OK`。CLI 安装为下次启动写入设置；编辑器插件会同时注册当前编辑器使用的参数。运行游戏不需要插件持续工作，也不需要 autoload。禁用插件会保留已安装的设置，避免损坏仍在使用的材质。

## 接入自己的角色

先阅读 [AI_CONTEXT.md](AI_CONTEXT.md)。使用 Godot Inspector 创建 `NPRCharacterMaterials` 资源并填入 13 个贴图字段；创建 `NPRCharacterDefinition` 资源，绑定导入的模型 PackedScene、三个网格路径及材质资源。然后给一个 Node3D 挂 `npr_character.gd` 并填入 definition。

也可以用脚本创建：

```gdscript
var actor := NPRCharacter.new()
actor.definition = preload("res://characters/my_character/definition.tres")
actor.set_ramp_mix(0.45)
add_child(actor)
if not actor.initialized:
    print(actor.validation_errors)
```

参数 setter 支持在 `add_child()` 前调用，初始化后会应用。角色节点拥有自己的材质副本、阴影代理和深度缓存；模型 Mesh 和 Texture 资源共享，不能把共享贴图的像素修改当作单角色材质调整。需要不同贴图时，为对应定义创建独立资源。

版本 1 使用三个明确的渲染角色：Body、Face、Hair，各绑定一个单 surface 的 ArrayMesh。衣服、装备、手套、鞋、饰品可以是 Body 中互不连接的几何岛，用 UV atlas 和 ILM 材质槽区分。头发两个 stencil pass 由模块建立；脸部也会自动建立眼部 stencil pass。模型本身不需要任何专属根节点脚本。

银狼继续位于主项目中，通过 `showcase/npr_character_preview.gd` 使用 `use_source_materials` 接口接入。该适配器保留其经过校准的导入坐标、材质贴图和参数。通用模块不包含银狼贴图、网格或实验室 UI。

## 灯光、相机与环境

宿主项目提供当前 Camera3D 和 WorldEnvironment。可以使用 `materials/studio_environment.tres` 作为起点：线性色调映射、曝光 1.0；示例启用 MSAA 4x。模块不强行覆盖宿主环境。改变曝光、色调映射、后处理或相机采样会改变最终外观。

每个角色自动申请一个私有可见层；相机须包含角色获得的层。默认 Camera3D 的完整 20 层可直接使用。第 1 层用于共同舞台，剩余 19 层用于人物隔离。相同方向和阴影设置的角色共享方向光；方向光预算仍受每个视图最多 8 盏的引擎约束，并计算宿主方向光的占用。`isolation_available` 可观察隔离层是否可用。

把角色移出树再挂回时，会保留并恢复资源。暂存但不释放的 actor 仍占用层租约。确定不再使用时调用 `queue_free()`；不要长期缓存无限数量的脱树 actor。主光是世界空间方向，旋转整个 actor 不会同时旋转光源。

角色模型中的 Camera3D、Light3D、WorldEnvironment 会从新实例中去掉，避免模型携带的捕获环境干扰宿主。动画、骨骼和普通子节点保留。角色脚本必须允许这种纯模型使用方式。

## 可调接口

| 接口 | 默认值 | 建议范围 / 单位 |
| --- | --- | --- |
| `light_yaw` | -45 | 世界空间方位角，度 |
| `light_elevation` | 45 | 高度角，通常 5..80 度 |
| `set_ramp_mix(value)` | 0.45 | 0..1，暖 / 冷 Ramp 混合 |
| `set_sdf_feather(value)` | 0.015 | 0..0.15，角度归一化后的阈值羽化 |
| `set_shadow_strength(value)` | 0.28 | 0..0.65，实时 CSM 强度 |
| `set_hair_highlight(value)` | 0.30 | 0..1，头发高光带 |
| `set_hair_contact(value)` | 0.35 | 0..1，刘海对脸部的接触阴影 |
| `set_fill_strength(value)` | 0 | 0..1，局部补光 |
| `set_rim_strength(value)` | 0.10 | 0..0.5，深度轮廓光 |
| `set_outline_width(value)` | 1.0 | 0..3，最终 viewport 的逻辑像素 |
| `set_depth_quality(value)` | 2 | 0 性能，1 均衡，2 完整；前两者是有损选择 |
| `get_world_bounds()` | — | 当前可见几何的世界 AABB |
| `pick_surface(origin, direction)` | — | 世界空间射线；返回最近几何命中 Dictionary |

`material_profile.specular_exponents` 是 8 个完整浮点高光指数，默认全部为 24，允许 0.01..4096。它们是明确的美术预设。`capture_verified` 默认 false；不要把这些值描述成从原游戏恢复的参数。

更底层的多视图、TAA、外部 RID 和 depth array 支持放在 `runtime/` 中，继续使用同一份生产实现。普通接入不需要启用它们。输出级调色 UI 和实验室应用仍由宿主项目负责。

## 文件与交付边界

- `npr_character*.gd`：对外角色、资产定义、材质输入接口。
- `runtime/`：阴影、深度、几何状态、拾取、灯光和多视图基础设施。
- `shaders/`：唯一的生产 shader 实现和公共 include。
- `materials/`：数值参数模板和建议环境；模板中的贴图由构建器填入。
- `config/`、`install.gd`、`plugin.gd`：一次性安装工具。
- `examples/`：程序生成的人物和完整可运行场景。
- `AI_CONTEXT.md`、`asset_contract.json`：面向建模 AI 的约定。

模型和纹理必须遵循约定。模块负责已有渲染技术的稳定组合；具体角色的面部 SDF、高光遮罩、法线和颜色设计仍需美术制作。示例人偶用于接口和渲染检查，不是完整的角色美术作品。
