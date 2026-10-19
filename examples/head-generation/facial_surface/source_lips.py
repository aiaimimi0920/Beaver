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
    allowed = {"upper_lip_relief_scale", "upper_lip_lateral_power"}
    if not isinstance(parameters, dict) or set(parameters) - allowed:
        raise ValueError("Expected only documented upper-lip controls")
    scale = parameters.get("upper_lip_relief_scale", 1.0)
    if isinstance(scale, bool) or not isinstance(scale, (int, float)):
        raise ValueError("Upper-lip scale must be finite numeric")
    if not math.isfinite(scale) or not 0.85 <= scale <= 1.0:
        raise ValueError("Upper-lip scale must be in [0.85, 1.0]")
    result = copy.deepcopy(style)
    calibration = result["calibration"]
    old_power = calibration["upper_lip_lateral_power"]
    new_power = parameters.get("upper_lip_lateral_power", old_power)
    if isinstance(new_power, bool) or not isinstance(new_power, (int, float)) \
            or new_power not in (2, 4):
        raise ValueError("Upper-lip lateral power must be 2 or 4")
    if scale == 1 and new_power == old_power:
        return result, None
    before = calibration["upper_lip_relief"]
    positive = (before, calibration["height_m"], calibration["upper_lip_width"],
                calibration["upper_lip_lateral_width"])
    if any(isinstance(v, bool) or not isinstance(v, (int, float))
           or not math.isfinite(v) or v <= 0 for v in positive):
        raise ValueError("Expected finite positive authored lip calibration")
    if isinstance(old_power, bool) or old_power not in (2, 4):
        raise ValueError("Lip bound requires authored lateral power 2 or 4")
    calibration["upper_lip_relief"] = before * scale
    calibration["upper_lip_lateral_power"] = new_power
    factor = 0.0 if new_power == old_power else lateral_difference_bound()
    # Triangle inequality: amplitude difference plus lateral shape difference.
    bound = calibration["height_m"] * before * ((1 - scale) + scale * factor) + 1e-9
    return result, {
        "parameters": {"upper_lip_relief_scale": scale,
                       "upper_lip_lateral_power": new_power},
        "before": before,
        "after": before * scale,
        "lateral_power_before": old_power,
        "lateral_power_after": new_power,
        "lateral_factor_bound": factor,
        "analytic_depth_change_bound_m": bound,
        "scope": "Upper-lip analytic relief only; native neutral and mouth poses require review",
    }
