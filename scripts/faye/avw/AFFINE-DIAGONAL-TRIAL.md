# 保持字段不变的局部对角线试算

`affine_diagonal_trial.py` 是 AVW 的隔离研究步骤，不是默认建模流程。
它只在四个顶点的全部字段满足同一仿射插值、四边形凸且不改变覆盖范围时换边。
它不增加顶点或面，不改顶点字段，保留指定运动支撑点与阴影保护边界。
四姿态长宽比只用于提出候选，不能代替全部验证。

```powershell
python scripts/faye/avw/affine_diagonal_trial.py `
  --source "C:\Users\mjc\BeaverWorkspace\script-validation\avw-unified-20261112-v3" `
  --validation-root "C:\Users\mjc\BeaverWorkspace\validation" `
  --output "C:\Users\mjc\BeaverWorkspace\script-validation\my-diagonal-trial"
```

输出必须是不存在的新目录，不能位于输入目录内。原输入及默认模型不会被修改。
随后用显式 `FAYE_VALIDATION_ROOT` 和 `FAYE_OUTPUT_ROOT` 运行现有的
internal、eval、interface、finalaudit、edges、bounds、distance、fields_detail、
target、camera、trace、check 模块，再用 Blender 后台执行 render、close。
不要在新候选上调用 build 或再次进行 interfacefix，它们会重新构造候选。
图像需人工检查；本工具不会设置视觉验收成功。

## 2026-11-12 隔离结果

- 换了五条局部对角线，区域仍为 302 面。全部 22 顶点字段未改变。
- 一处局部最大长宽比由 33.29 降至 6.06；四姿态全局最坏值仍为 173.77。
- 338 个区域及接口面、2,187 个离散姿态未见浮点非正面积；内外 T 结点为零。
- 固定阴影探针未增加遮挡；这不是所有视角/所有姿态的证明。
- 两个实际 Blender 诊断渲染步骤成功，但整脸与近景仍有明显扇形、长斜线和不均匀过渡。
- 因此只确认局部三角形形状改善，**整脸视觉未通过**；未新增原生生成、GLB 或 MiDot 验收。
- 数值验证输出与仓库模块独立再运行结果逐数组相同。原脚本/数据保留。

局部换边不能解决整体方向带设计问题。后续应保持已验证运动支撑，研究相邻面带的整体连接，
而不是删除关键支撑来隐藏扇形。
