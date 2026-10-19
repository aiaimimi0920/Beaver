"""CPU source-field sampling; never represents final native mesh acceptance."""

import ast
import math
from curve_profile import delta

FUNCTIONS = {
    "curve",
    "width",
    "nose_profile_factor",
    "nose",
    "lip_shoulder_support",
    "lip_detail_depth",
    "base_depth",
    "rim_depth",
    "depth",
}


def scope(source, c, fit, amount):
    helpers = (
        {
            "angular_coordinate",
            "angular_derivative",
            "eye_warp",
            "eye_warp_derivative",
            "upper_shape",
            "upper_shape_derivative",
            "lower_coordinate",
            "lower_shape",
        }
        if (c.get("eye_upper_asymmetry", 0) or c.get("eye_lower_shoulder", 0) or c.get("eye_upper_angularity", 0))
        else set()
    )
    if c.get("mouth_seam_bow_height", 0):
        helpers |= {"mouth_profile_shift", "mouth_rest_seam_y"}
    required = FUNCTIONS | helpers
    nodes = [
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name in required
    ]
    if {n.name for n in nodes} != required:
        raise ValueError("Depth source contract changed")
    result = dict(
        C=c,
        H=c["height_m"],
        Y0=c["origin_y_m"],
        Z0=c["chin_depth_m"],
        exp=math.exp,
        sqrt=math.sqrt,
    )
    exec(
        compile(
            ast.Module(body=nodes, type_ignores=[]), "<analytic functions>", "exec"
        ),
        result,
    )
    for name in ("base_depth", "rim_depth"):
        original = result[name]
        result[name] = lambda x, y, fn=original: fn(x, y) + delta(
            x, y, c, result["curve"], fit, amount
        )
    return result


def measures(z, step):
    slopes = [(b - a) / step for a, b in zip(z, z[1:])]
    curvature = [
        (b - a) / step / (1 + ((a + b) / 2) ** 2) ** 1.5
        for a, b in zip(slopes, slopes[1:])
    ]
    return dict(
        curvature_variation=sum(abs(b - a) for a, b in zip(curvature, curvature[1:])),
        peak_curvature=max(map(abs, curvature), default=0),
        extrema=sum(a * b < 0 for a, b in zip(slopes, slopes[1:])),
    )


def diagnose(source, c, fit, amount):
    before, after = [scope(source, c, fit, a) for a in (0.0, amount)]
    rows = []
    for field in ("base_depth", "depth"):
        for axis, stations in (
            ("vertical", (0.02, 0.03, 0.035, 0.04, 0.045, 0.05, 0.055)),
            ("transverse", (0.28, 0.36, 0.44, 0.52, 0.60, 0.68)),
            ("diagonal", (-1.0, 1.0)),
        ):
            for station in stations:
                if axis == "vertical":
                    coords = [
                        (
                            station,
                            c["origin_y_m"] + (0.10 + 0.80 * i / 800) * c["height_m"],
                        )
                        for i in range(801)
                    ]
                elif axis == "transverse":
                    coords = [
                        (0.075 * i / 750, c["origin_y_m"] + station * c["height_m"])
                        for i in range(751)
                    ]
                else:
                    coords = [
                        (
                            0.02 + 0.04 * i / 800,
                            c["origin_y_m"]
                            + (0.48 + station * 0.24 * (2 * i / 800 - 1))
                            * c["height_m"],
                        )
                        for i in range(801)
                    ]
                step = math.dist(coords[0], coords[1])
                old, new = [
                    measures([s[field](x, y) for x, y in coords], step)
                    for s in (before, after)
                ]
                rows.append(
                    dict(
                        field=field,
                        axis=axis,
                        station=station,
                        before=old,
                        after=new,
                        variation_ratio=(
                            new["curvature_variation"] / old["curvature_variation"]
                            if old["curvature_variation"]
                            else None
                        ),
                    )
                )
    return dict(
        scope="analytic source fields, including boundary transitions; not final native geometry",
        acceptance="diagnostic only; broad curvature and local ripple require separate visual interpretation",
        cuts=rows,
    )
