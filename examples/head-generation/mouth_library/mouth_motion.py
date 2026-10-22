"""Local authored motion fields in final Blender coordinates, not phonemes."""

import math

TARGETS = ("MouthRound", "MouthSpread", "MouthPucker")


def motion_delta(point, calibration, target):
    if target not in TARGETS:
        raise ValueError("Unknown mouth state")
    x, depth_axis, z = point
    h = calibration["height_m"]
    center = calibration["origin_y_m"] + h * calibration["mouth_height"] - 0.021
    u = x / (h * 0.18)
    v = (z - center) / (h * 0.08)
    if abs(u) >= 1 or abs(v) >= 1:
        return (0.0, 0.0, 0.0)
    weight = (1 - u * u) ** 3 * (1 - v * v) ** 3
    if target == "MouthRound":
        return (-0.32 * x * weight, -0.006 * h * weight, 0.0)
    if target == "MouthSpread":
        return (0.24 * x * weight, 0.0, 0.0)
    pucker_weight = weight * (1.0 - 0.5 * u * u)
    return (-0.08 * x * pucker_weight, -0.018 * h * pucker_weight, 0.0)


def validate_weights(weights):
    names = {"MouthOpen", *TARGETS}
    if not isinstance(weights, dict) or set(weights) - names:
        raise ValueError("Expected explicit mouth channel weights")
    for value in weights.values():
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not 0 <= value <= 1
        ):
            raise ValueError("Mouth weight outside [0,1]")
    return {name: float(weights.get(name, 0.0)) for name in sorted(names)}
