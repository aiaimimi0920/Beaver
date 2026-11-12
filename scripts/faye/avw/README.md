# 菲 AVW 研究脚本

这是一组从现有菲建模工作流纳入主仓库的 **研究生成与验证工具**，不是 Beaver 原生生成入口，也不是已验收模型。AVW 当前整脸视觉仍未通过；这里的数值试算不能代替 GLB、MiDot 或视觉验收。

## 运行

使用 Python 3.12，在独立虚拟环境安装本目录 `requirements.txt`。不要把依赖或模型数据提交到仓库。

```powershell
python -m pip install -r scripts/faye/avw/requirements.txt
python scripts/faye/avw/run.py `
  --validation-root "C:\Users\mjc\BeaverWorkspace\validation" `
  --output "C:\Users\mjc\BeaverWorkspace\script-validation\avw-new-run"
```

输出目录必须是尚不存在的新目录，并且位于输入 validation 目录之外。脚本不覆盖历史候选、不替换默认模型、不移动源数据。输入是现有 AVD、ASY、ATD、ATC、APK 的十个文件，完整相对清单在 `run.py:INPUTS`；这些研究数据没有随源码发布。缺文件时在开始前报错。

需要 Blender 诊断渲染时附加：

```powershell
--blender "C:\Users\mjc\BeaverWorkspace\tools\blender-4.3.2-windows-x64\blender.exe"
```

Blender 使用后台工厂场景，只写新输出目录，不接触打开的模型工程。图片仍须人工查看，程序不会宣称视觉通过。Windows/Linux 都必须使用各自平台的依赖和 Blender；不复制 Windows 可执行文件到 Linux。

## 模块及顺序

- `support_context.py`：提取 ave/avf/avg/avh/avi 前缀中构建所需的边界、方向带、站位上下文，去除历史审计目录写入。
- `build.py`：保留 source5407 内部运动支撑，生成几何与全部属性字段。
- `internal.py`、`interfacefix.py`、`interface.py`、`edges.py`：检查并协调内外接口、生成实际边列表。由 runner 按顺序执行，不能随意重复接口修正。
- `eval.py`、`finalaudit.py`、`bounds.py`：离散姿态、接口和连续多项式估计；浮点计算不是严格区间证明。
- `distance.py`、`closest_surface.py`、`fields_detail.py`、`target.py`：有限采样表面距离、属性和 MouthOpen 对比。
- `camera.py`、`trace.py`：固定射线/阴影探针；不是整幅图视觉验收。
- `render.py`、`close.py`：全脸及近景诊断图片。
- `paths.py`、`run.py`：显式数据路径、新目录保护、逐步日志及 `RUN.json` 输入哈希。

原用户目录中的脚本和模型保留未动，旧路径仍可使用；后续修改应在主仓库此目录进行。`provenance.json` 保存原脚本文件名与迁入时 SHA-256。没有导入静态写结论的 `avw-finish.py`，也没有把旧结果冒充此次运行结果；旧 run/postrun/status 包装已由新入口替代。

## 检查

```powershell
python -m unittest discover -s scripts/faye/avw -p "test_*.py"
python -m compileall -q scripts/faye/avw
python -m black --check scripts/faye/avw
npm run check:effective-lines
```

`RUN.json` 分别记录实际步骤退出码和输入是否变化，并始终保留 `native_generated=false`、`glb_verified=false`、`visual_accepted=false`。CPU 通过不改变这些标记。

## 纳管验证（2026-11-12）

在独立输出目录重跑全部 15 个数值步骤及 2 个 Blender 渲染步骤，均退出 0，十个输入文件 SHA-256 保持不变。full-fields 的 22 个字段、full-geometry 的 faces、region 的 faces/uv 与原 AVW 逐数组完全相同。8 个保护/几何单测、Python 语法及 Black 检查通过。迁入最大模块 build.py 为 343 有效行；代码结构检查无新增违规。这证明工作流迁入没有改变该候选，不代表新的布线改进、原生生成、GLB 或视觉验收通过。
