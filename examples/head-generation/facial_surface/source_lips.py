"""Opt-in lip relief and lateral falloff; no mouth contour or motion edits."""

import copy
import math


def lateral_difference_bound():
    """Bound |exp(-u^2)-exp(-u^4)| for all real u via t=u^2.

    Both exponentials are monotone on each positive interval. Beyond t=32
    their absolute difference is at most exp(-32). Floating/libm caveats apply.
    """
    bound = math.exp(-32)
    for index in range(32000):
        a, b = index / 1000, (index + 1) / 1000
        lower = math.exp(-b) - math.exp(-a * a)
        upper = math.exp(-a) - math.exp(-b * b)
        bound = max(bound, abs(lower), abs(upper))
    return bound + 1e-9


def apply_lip_relief(style, parameters):
    if parameters is None:
        parameters = {}
    allowed = {
        "upper_lip_relief_scale",
        "upper_lip_lateral_power",
        "lower_lip_relief_scale",
        "upper_lip_bow_height",
    }
    if not isinstance(parameters, dict) or set(parameters) - allowed:
        raise ValueError("Expected only documented upper-lip controls")
    scale = parameters.get("upper_lip_relief_scale", 1.0)
    if isinstance(scale, bool) or not isinstance(scale, (int, float)):
        raise ValueError("Upper-lip scale must be finite numeric")
    if not math.isfinite(scale) or not 0.85 <= scale <= 1.0:
        raise ValueError("Upper-lip scale must be in [0.85, 1.0]")
    lower_scale = parameters.get("lower_lip_relief_scale", 1.0)
    bow = parameters.get("upper_lip_bow_height", 0.0)
    for name, value, low, high in (
        ("lower_lip_relief_scale", lower_scale, 1.0, 1.25),
        ("upper_lip_bow_height", bow, 0.0, 0.003),
    ):
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not low <= value <= high
        ):
            raise ValueError("Invalid lip control: " + name)
    result = copy.deepcopy(style)
    calibration = result["calibration"]
    old_power = calibration["upper_lip_lateral_power"]
    new_power = parameters.get("upper_lip_lateral_power", old_power)
    if (
        isinstance(new_power, bool)
        or not isinstance(new_power, (int, float))
        or new_power not in (2, 4)
    ):
        raise ValueError("Upper-lip lateral power must be 2 or 4")
    if scale == 1 and new_power == old_power and lower_scale == 1 and bow == 0:
        return result, None
    before = calibration["upper_lip_relief"]
    positive = (
        before,
        calibration["height_m"],
        calibration["upper_lip_width"],
        calibration["upper_lip_lateral_width"],
        calibration["lower_lip_relief"],
        calibration["mouth_half_width"],
    )
    if any(
        isinstance(v, bool)
        or not isinstance(v, (int, float))
        or not math.isfinite(v)
        or v <= 0
        for v in positive
    ):
        raise ValueError("Expected finite positive authored lip calibration")
    if isinstance(old_power, bool) or old_power not in (2, 4):
        raise ValueError("Lip bound requires authored lateral power 2 or 4")
    calibration["upper_lip_relief"] = before * scale
    calibration["upper_lip_lateral_power"] = new_power
    lower_before = calibration["lower_lip_relief"]
    calibration["lower_lip_relief"] = lower_before * lower_scale
    if bow:
        calibration["upper_lip_bow_height"] = bow
    factor = 0.0 if new_power == old_power else lateral_difference_bound()
    # Triangle inequality: amplitude difference plus lateral shape difference.
    bound = calibration["height_m"] * before * ((1 - scale) + scale * factor) + 1e-9
    shift_bound = min(1.0, math.sqrt(2 / math.e) * bow / calibration["upper_lip_width"])
    bound += calibration["height_m"] * (
        before * scale * shift_bound + lower_before * abs(lower_scale - 1)
    )
    return result, {
        "lower_scale": lower_scale,
        "upper_bow_height": bow,
        "lower_before": lower_before,
        "lower_after": lower_before * lower_scale,
        "parameters": {
            "upper_lip_relief_scale": scale,
            "upper_lip_lateral_power": new_power,
            "lower_lip_relief_scale": lower_scale,
            "upper_lip_bow_height": bow,
        },
        "before": before,
        "after": before * scale,
        "lateral_power_before": old_power,
        "lateral_power_after": new_power,
        "lateral_factor_bound": factor,
        "analytic_depth_change_bound_m": bound,
        "scope": "Upper/lower analytic lip relief and upper ridge height; seam and motion algorithms unchanged, cavity depth may follow relief; native neutral and mouth poses require review",
    }


def adapt_lip_source(source, calibration):
    if not calibration.get("upper_lip_bow_height", 0):
        return source
    old = 'y - Y0 - H * C["upper_lip_height"]'
    radius = '(0.7 * H * C["mouth_half_width"])'
    ratio = "(x / " + radius + ")"
    bow = (
        '(H * C["upper_lip_bow_height"] * (27/4) * '
        + ratio
        + "**2 * (1-"
        + ratio
        + "**2)**2 if abs(x) < "
        + radius
        + " else 0.0)"
    )
    if source.count(old) != 1:
        raise ValueError("Upper lip ridge source contract changed")
    result = source.replace(old, old + " - " + bow, 1)
    compile(result, "<bowed lip ridge>", "exec")
    return result
