# 664 源码交接

2026-10-15用户确认当前头部SDF效果可以，并要求同步Beaver与NPRCharacterFrame。此次提交只包含自有代码、配置、文档和测试；没有上传银狼参考资产、用户凭据、私有运行数据库、截图或导入缓存。

- 最新几何生成源码：examples/head-generation/checkpoints/current（输出662）。
- SDF修复生成与方向测试：examples/head-generation/sdf360（输出664）。
- 实际对比、旋转、变焦、光源、白模/线框和采集：examples/head-review。
- 可复用插件材质：NPRCharacterFrame的shaders/face/sdf360；Beaver的npr-characters包按provenance精确同步357个上游文件。
- 既有src/direct已包含恢复工作台、原生预览和任务/结果交接，本次保留较新的仓库实现，不用旧恢复副本覆盖。

验证：当前配方356项纯测试、原始示例106项、SDF9项、原生预览桥7项。原生664烘焙6输出SHA、250实际图及10方向图像检查已通过，左右45°实际观察已完成。代码发布层另做语法、资源路径和来源校验。完整产品/装扮/Windows发布不在本轮范围。

发布环境未安装cargo或gdscript-post-check。已使用gdtoolkit安全格式化/解析替代后者；npm run check:effective-lines本机因cargo缺失无法完成，保留CI实际结果，不能用本条文档宣称该门禁通过。eye_accents保留623有效行（CI词法扫描实测）的有限例外及356项保护测试，没有放宽基线或新增700行以上源文件。

恢复所需授权基线和资源输入见当前检查点README。源码同步不能替代模型恢复包；模型和冻结几何输入仍在用户私有恢复包中。

CI首次发现Python 3.11/3.12空type_params元数据差异，以及保守行数估计与正式词法统计不一致。已保留原始函数SHA合同并增加版本无关AST规范化；例外绑定正式实测623行及同一源SHA，未调整扫描基线。
