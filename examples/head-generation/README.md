# Original semantic head-generation recipe

For the current experimental face-only workflow, see
[Face-only creation and visual review](face-only-workflow.md). It documents the
private baseline inputs, packaging and export contracts, and bounded native
review evidence. Case709 remains the primary internal candidate; this does not
replace the separately user-confirmed664 baseline below or imply final approval.

Historical review candidate:185. See review-results.md for actual checks and limitations.

Workflow: owner natural language → authoring brief → semantic planner mapping → face_style.json → packaged Beaver generation → native white/wire/render review. Do not run Blender from this packaging script. Generate bundle.py --output request.json and pass it through the existing Beaver blender.start tool.

Run local pure regressions with python3 -m unittest discover -q in this directory. The native project and private reference assets are not included in this source example.

The accepted baseline eye/head parameter freeze is checked by frozen177-contract.json. Eye-accent geometry is original, parameterized and kept separate from frozen regions. No reference mesh/UV/pixels enter the generator.

## 当前已确认切片：664

最新源码与完整纯测试位于 [checkpoints/current](checkpoints/current/README.md)，替代先前529候选记录。662保留最新几何，664叠加已确认的连续定向SDF。新的生成入口为 [sdf360](sdf360/README.md)。2026-10-15用户确认当前效果并要求同步两仓库；NPRCharacterFrame可选SDF实现已独立提交，Beaver包按该提交精确固定。旧章节/历史检查点不代表当前完成状态。
