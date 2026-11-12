# 外侧方向带连接试算

这是隔离研究变体，不替换默认模型。`run.py` 默认 `--band-connections baseline`，仍复现 AVW；显式选择 `compact-outer` 才省略中间两排之间外侧的一条斜向连接。原边界、运动支撑、保护分区及字段插值来源不变。

```powershell
python scripts/faye/avw/run.py `
  --validation-root C:\Users\mjc\BeaverWorkspace\validation `
  --output C:\Users\mjc\BeaverWorkspace\script-validation\my-compact-outer `
  --band-connections compact-outer `
  --blender C:\Users\mjc\BeaverWorkspace\tools\blender-4.3.2-windows-x64\blender.exe
```

输出必须为新的隔离目录；研究输入和第三方模型数据不提交到仓库。

## 2026-11-12 实际检查

- 区域从 302 面降至 300 面，未删除关键运动支撑。
- 336 个变化区域/接口面 × 2,187 离散姿态，没有非正面积；最终内外接口 T 结点均为零。
- 固定源点的表面误差以及 22 字段最大分量误差与 AVW 相同（浮点容差内），包括法线、切线、形变字段；这不等于它们相对原模型误差为零。原有全局最差长宽比未改善。
- 固定阴影探针没有新增遮挡。候选到源的采样点数随面数变化，不能直接将该方向 RMS 的变化称为改善。
- 15 个数值步骤退出码为 0，十个输入哈希不变。模块输出的字段、几何及区域数组与隔离试算逐数组一致。
- 实际 Blender 全脸和近景图已检查：外侧少一条连接，但仍有明显扇形、长斜线和不均匀过渡，视觉未通过。
- 未进行 Beaver 原生生成、GLB 或 MiDot 验证，未替换默认模型。

另一隔离方案将两个中间跨行连接直接对齐到同列，面数反而增至 308，近景更乱，已拒绝，没有作为可选模式纳入。后续要协调运动支撑分区与方向带，而不是继续机械对齐站位。
