"""Shared optional upper-arch coordinate warp; physical eye layout is unchanged."""


def eye_warp(t, c):
    a = c.get("eye_upper_asymmetry", 0.0)
    if not a or abs(t) >= 1:
        return t
    return t + a * (1 - t * t) ** 3


def eye_warp_derivative(t, c):
    a = c.get("eye_upper_asymmetry", 0.0)
    if not a or abs(t) >= 1:
        return 1.0
    return 1 - 6 * a * t * (1 - t * t) ** 2


def upper_shape(t, c):
    u = eye_warp(t, c)
    return max(0.0, 1 - abs(u) ** c["eye_horizontal_power"]) ** c["eye_upper_power"]


def upper_shape_derivative(t, c):
    u = eye_warp(t, c)
    if abs(u) >= 1:
        raise ValueError(
            "Upper profile derivative is singular at the aperture endpoints"
        )
    if u == 0:
        return 0.0
    h, p = c["eye_horizontal_power"], c["eye_upper_power"]
    return (
        -p
        * h
        * (1 if u > 0 else -1)
        * abs(u) ** (h - 1)
        * (1 - abs(u) ** h) ** (p - 1)
        * eye_warp_derivative(t, c)
    )
