"""Analytic compact mouth support; runs in the emitted face namespace."""


def researched_mouth_support(x, y):
    d = C["mouth_design"]
    v = ((y - Y0) / H - 0.14) / 0.10
    u = x / (H * d["muzzle_half_width"])
    if abs(u) >= 1 or abs(v) >= 1:
        return 0.0
    return H * d["muzzle_projection"] * (1 - u * u) ** 3 * (1 - v * v) ** 3
