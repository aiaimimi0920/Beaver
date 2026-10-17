"""Experimental local eyelid closure; neutral mesh and non-eye geometry stay fixed."""

from contour_warp import upper_shape


def closure_delta(x, y, kind, c):
    h = c["height_m"]
    t = (abs(x) / h - c["eye_center_x"]) / c["eye_half_width"]
    if abs(t) >= 1.0:
        return 0.0
    center = c["origin_y_m"] + h * (c["eye_corner_y"] + c["eye_corner_slope"] * t)
    upper = (
        center
        + h
        * c["eye_upper_arch"]
        * max(0, 1 - abs(t) ** c["eye_horizontal_power"]) ** c["eye_upper_power"]
    )
    if c.get("eye_upper_asymmetry", 0):
        upper = center + h * c["eye_upper_arch"] * upper_shape(t, c)
    lower = (
        center
        - h
        * c["eye_lower_arch"]
        * max(0, 1 - abs(t) ** c["eye_lower_horizontal_power"]) ** c["eye_lower_power"]
    )
    seam = center - 0.0015 * (1 - t * t)
    if kind == "upper_ink":
        # y is the final generated column floor, not the analytic aperture.
        return seam - y
    if kind == "upper":
        return seam - upper
    if kind == "lower":
        return seam - lower
    if kind != "skin":
        return 0.0
    if lower < y < upper:
        return (seam - y) * 0.98
    if y >= center:
        distance = max(0, y - upper)
        delta = (seam - upper) * 0.98
        reach = 0.024
    else:
        distance = max(0, lower - y)
        delta = (seam - lower) * 0.98
        reach = 0.018
    u = min(1.0, distance / reach)
    weight = 1 - u * u * u * (10 + u * (-15 + 6 * u))
    return delta * weight


def build_blink_shapes(obj, ordinals, calibration, ink_floor_closure=False):
    data = obj.data
    regions = data.attributes["NormalRegion"]
    tags = {}
    for name, ordinal in ordinals.items():
        if ink_floor_closure and name.startswith("Upper lash "):
            tags[ordinal] = "upper_ink"
        elif name.startswith(
            ("Upper lash ", "Upper skin lid ", "Upper swept lash tuft ")
        ):
            tags[ordinal] = "upper"
        elif name.startswith(("Lower lash ", "Lower skin lid ")):
            tags[ordinal] = "lower"
    memberships = [set() for _ in data.vertices]
    for polygon in data.polygons:
        region = regions.data[polygon.index].value
        kind = "skin" if region == 1 else tags.get(region % 1000)
        if kind:
            for i in polygon.vertices:
                memberships[i].add(kind)
    ink_floors = {}
    for vertex in data.vertices:
        if "upper_ink" in memberships[vertex.index]:
            station = round(vertex.co.x, 8)
            ink_floors[station] = min(
                ink_floors.get(station, float("inf")), vertex.co.z + 0.021
            )
    report = {}
    for side, name in [(-1, "blink.L"), (1, "blink.R")]:
        key = obj.shape_key_add(name=name)
        changed = []
        for vertex in data.vertices:
            if side * vertex.co.x <= 0 or not memberships[vertex.index]:
                continue
            # A shared skin/lash vertex follows the explicit lid boundary.
            kinds = memberships[vertex.index]
            kind = (
                "upper_ink"
                if "upper_ink" in kinds
                else (
                    "upper"
                    if "upper" in kinds
                    else "lower" if "lower" in kinds else "skin"
                )
            )
            height = (
                ink_floors[round(vertex.co.x, 8)]
                if kind == "upper_ink"
                else vertex.co.z + 0.021
            )
            dy = closure_delta(vertex.co.x, height, kind, calibration)
            if abs(dy) > 1e-10:
                key.data[vertex.index].co.z += dy
                changed.append(abs(dy))
        key.value = 0.0
        assert changed, "Blink did not affect any eyelid vertices"
        report[name] = {
            "moved_vertices": len(changed),
            "maximum_displacement_m": max(changed),
            "neutral_weight": 0.0,
        }
    report["continuous_orientation"] = verify_pose_orientation(obj)
    report["scope"] = (
        "Experimental independent eyelid shapes; native posed visual acceptance pending"
    )
    return report


"""Minimum of a quadratic over the closed unit square, in floating-point arithmetic."""


def quadratic_minimum(a, b, c, d, e, f):
    # a + b*x + c*y + d*x*x + e*x*y + f*y*y
    points = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
    if d > 0:
        for y in (0.0, 1.0):
            x = -(b + e * y) / (2 * d)
            if 0 < x < 1:
                points.append((x, y))
    if f > 0:
        for x in (0.0, 1.0):
            y = -(c + e * x) / (2 * f)
            if 0 < y < 1:
                points.append((x, y))
    determinant = 4 * d * f - e * e
    if d > 0 and f > 0 and determinant > 0:
        x = (e * c - 2 * f * b) / determinant
        y = (e * b - 2 * d * c) / determinant
        if 0 < x < 1 and 0 < y < 1:
            points.append((x, y))
    return min(
        (a + b * x + c * y + d * x * x + e * x * y + f * y * y, x, y) for x, y in points
    )


def verify_pose_orientation(obj):
    from math import isfinite

    data = obj.data
    keys = data.shape_keys.key_blocks
    basis = [v.co.copy() for v in keys["Basis"].data]
    deltas = {
        name: [v.co - basis[i] for i, v in enumerate(keys[name].data)]
        for name in ("MouthOpen", "blink.L", "blink.R")
    }
    lowest = (float("inf"), 0, 0, -1)
    for polygon in data.polygons:
        assert len(polygon.vertices) == 3, "Pose gate requires actual triangles"
        ids = list(polygon.vertices)
        assert not (
            any(deltas["blink.L"][i].length > 1e-10 for i in ids)
            and any(deltas["blink.R"][i].length > 1e-10 for i in ids)
        ), "One triangle couples both eye controls"
        p0, p1, p2 = [basis[i] for i in ids]
        e1, e2 = p1 - p0, p2 - p0
        normal = e1.cross(e2)
        scale = normal.length_squared
        assert scale > 1e-24, "Degenerate neutral triangle"
        blink = [deltas["blink.L"][i] + deltas["blink.R"][i] for i in ids]
        mouth = [deltas["MouthOpen"][i] for i in ids]
        b1, b2 = blink[1] - blink[0], blink[2] - blink[0]
        m1, m2 = mouth[1] - mouth[0], mouth[2] - mouth[0]
        coefficients = [
            1.0,
            normal.dot(b1.cross(e2) + e1.cross(b2)) / scale,
            normal.dot(m1.cross(e2) + e1.cross(m2)) / scale,
            normal.dot(b1.cross(b2)) / scale,
            normal.dot(b1.cross(m2) + m1.cross(b2)) / scale,
            normal.dot(m1.cross(m2)) / scale,
        ]
        assert all(isfinite(v) for v in coefficients), "Nonfinite pose coefficients"
        value, blink_weight, mouth_weight = quadratic_minimum(*coefficients)
        assert value > 1e-5, (
            "Eyelid or mouth pose folds a triangle: "
            f"triangle={polygon.index}, vertices={ids}, "
            f"orientation_ratio={value:.9g}, blink={blink_weight:.9g}, "
            f"mouth={mouth_weight:.9g}"
        )
        lowest = min(lowest, (value, blink_weight, mouth_weight, polygon.index))
    return {
        "triangles": len(data.polygons),
        "minimum_orientation_ratio": lowest[0],
        "worst_blink_weight": lowest[1],
        "worst_mouth_weight": lowest[2],
        "worst_triangle": lowest[3],
        "scope": "Floating-point analytic quadratic minimum over blink and MouthOpen [0,1]; no triangle couples both eyes. Not an intersection proof.",
    }
