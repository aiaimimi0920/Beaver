"""Brow-only silhouette grammars; identity keeps the retained source byte-for-byte."""

import ast
import math


def adapt_brow_profiles(source, angularity=0.0, peak=0.5, band=0.0):
    for value, low, high in ((angularity, 0, 1), (peak, 0.3, 0.75), (band, 0, 1)):
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not low <= value <= high
        ):
            raise ValueError("Invalid bounded eyebrow profile")
    if angularity == 0 and band == 0:
        return source
    old = 'STYLE["brow_arch_m"] * sin(pi * t)'
    if source.count(old) != 1:
        raise ValueError("Brow centerline source contract changed")
    source = source.replace(old, 'STYLE["brow_arch_m"] * brow_profile_arch(t)', 1)
    tree = ast.parse(source)
    nodes = [
        n
        for n in tree.body
        if isinstance(n, ast.FunctionDef) and n.name == "brow_lane_offset"
    ]
    if len(nodes) != 1:
        raise ValueError("Brow width source contract changed")
    node = nodes[0]
    lines = source.splitlines(keepends=True)
    width = f"""def brow_lane_offset(t, lane):
    original = H * C["brow_thickness"] * sin(pi * t) ** 0.7
    exponent = 0.7 - 0.3 * C.get("brow_head_rounding", 0) * (1-t)**3
    exponent = (1-{band!r}) * exponent + {band!r} * 0.18
    factor = 1 + (1-{band!r}) * C["brow_inner_fullness"] * (1-2*t)
    width = 0.0 if t <= 0 or t >= 1 else H * C["brow_thickness"] * max(0, sin(pi*t)) ** exponent
    return original * 0.5 + (lane-0.5) * width * factor
"""
    source = (
        "".join(lines[: node.lineno - 1]) + width + "".join(lines[node.end_lineno :])
    )
    source += f"""

def brow_profile_arch(t):
    peak = {peak!r}
    wedge = t/peak if t <= peak else (1-t)/(1-peak)
    return (1-{angularity!r}) * sin(pi*t) + {angularity!r} * wedge
"""
    compile(source, "<brow silhouette profiles>", "exec")
    return source
