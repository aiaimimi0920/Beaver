# 原创定向 SDF 生成

这是用户确认的664修复：将眼窝/眼睑/嘴唇细微凹凸从面部光照代理中排除，生成连续平滑阴影。24个方位×7个仰角，单图块256²，2px边界；精确欧氏距离场在Shader中先插值再阈值。

运行 `python bundle.py --output request.json [--workspace PROJECT]`，再由已有Beaver任务执行请求。依赖Python、NumPy以及Beaver管理的Blender；本命令不启动模型服务或Blender。几何及冻结恢复输入见 ../checkpoints/current/README.md。

当前定义使用head-review示例的项目级SDF材质；NPRCharacterFrame也提供可复用的addons/npr_character_frame/shaders/face/sdf360/sdf_materials.gd。两者采用相同采样逻辑，后者作为插件直接使用。

测试：`python -m unittest discover -s . -p 'test_*.py'`。线性数据贴图，不开mipmap，不进行sRGB颜色解码。模型局部坐标为X右、Y上、Z前，UV2遵循该配方的原始脸部高度映射。

仅单主光；不包含通用投影遮挡、多灯合成或第三方SDF贴图。输出和实际画面验收分开。
