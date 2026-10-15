"""Explicit deterministic Chinese intent contract; not an NLP model."""
from controls import PRESETS, validate

ALIASES = {"基准": "baseline", "柔和宽脸": "soft_wide", "修长脸": "slender"}
PROMPT = """生成一个具有独立可编辑部件和 MouthOpen 形态键的风格化人脸。
本版本只支持全局宽、高、深的有界比例调整；沿用既有拓扑和五官布局。
意图必须显式提供 preset（baseline / soft_wide / slender；中文可用基准、柔和宽脸、修长脸），
或者 proportions（width 0.92–1.08，height 0.94–1.06，depth 0.96–1.04）。
不能同时提供两种方式。未提供的比例为 1。不得把未知诉求默认为基准。
改变年龄、身份、局部五官、拓扑或发型等诉求不受此版本支持，须明确报告。
这是确定性参数契约，不宣称理解任意自然语言或从零生成新身份。
"""


def select_intent(intent):
    if not isinstance(intent, dict) or len(intent) != 1:
        raise ValueError("Supply exactly one explicit preset or proportions field")
    if "preset" in intent:
        name = intent["preset"]
        if not isinstance(name, str):
            raise ValueError("Preset must be a supported string")
        name = ALIASES.get(name, name)
        if name not in PRESETS:
            raise ValueError(f"Unsupported preset: {name}")
        return {"preset": name, "proportions": dict(PRESETS[name])}
    if "proportions" in intent:
        return {"preset": "explicit", "proportions": validate(intent["proportions"])}
    raise ValueError("Unsupported intent; this version supports global proportions only")
