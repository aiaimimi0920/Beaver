"""Original procedural silhouettes, not copied character assets; meters locally."""

import math


def brow_area(points):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(points, points[1:] + points[:1])) / 2


def brow_clean(points):
    result = []
    for point in points:
        if not result or math.dist(point, result[-1]) > 1e-10:
            result.append(tuple(point))
    if len(result) > 1 and math.dist(result[0], result[-1]) < 1e-10:
        result.pop()
    return result if brow_area(result) > 0 else result[::-1]


def brow_cross(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def brow_segments_touch(a, b, c, d):
    values = [brow_cross(a, b, c), brow_cross(a, b, d), brow_cross(c, d, a), brow_cross(c, d, b)]
    if values[0] * values[1] < -1e-24 and values[2] * values[3] < -1e-24:
        return True
    for value, start, end, point in zip(values, (a, a, c, c), (b, b, d, d), (c, d, a, b)):
        if abs(value) <= 1e-12 and all(min(start[k], end[k]) - 1e-12 <= point[k] <= max(start[k], end[k]) + 1e-12 for k in (0, 1)):
            return True
    return False


def brow_validate_polygon(points):
    if len(points) < 3 or brow_area(points) < 1e-10 or any(not math.isfinite(v) for p in points for v in p):
        raise ValueError("Invalid brow polygon area or coordinates")
    count = len(points)
    for i in range(count):
        a, b = points[i], points[(i + 1) % count]
        if math.dist(a, b) < 1e-9:
            raise ValueError("Collapsed brow edge")
        for j in range(i + 2, count):
            if i == 0 and j == count - 1:
                continue
            c, d = points[j], points[(j + 1) % count]
            if brow_segments_touch(a, b, c, d):
                raise ValueError("Self-intersecting brow outline")


def brow_sweep(path, widths):
    left, right = [], []
    for i, ((x, y), width) in enumerate(zip(path, widths)):
        a, b = path[max(0, i - 1)], path[min(len(path) - 1, i + 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        length = math.hypot(dx, dy)
        if length < 1e-10:
            raise ValueError("Collapsed brow sweep tangent")
        nx, ny = -dy / length * width / 2, dx / length * width / 2
        left.append((x + nx, y + ny))
        right.append((x - nx, y - ny))
    return brow_clean(right + left[::-1])


def brow_curl(length, width, turns):
    path = []
    for i in range(65):
        t = i / 64
        theta = (1 - t) * math.tau * turns
        radius = length * (0.035 + 0.165 * t)
        path.append((length * 0.24 + radius * math.cos(theta), length * 0.08 + radius * math.sin(theta)))
    start = path[-1]
    control, end = (length * 0.60, -length * 0.10), (length, 0.0)
    for i in range(1, 33):
        t = i / 32
        path.append(tuple((1 - t) ** 2 * start[k] + 2 * t * (1 - t) * control[k] + t * t * end[k] for k in (0, 1)))
    widths = [width * max(0, math.sin(math.pi * i / (len(path) - 1))) ** 0.5 for i in range(len(path))]
    return brow_sweep(path, widths)


def brow_sampling_divisions(design, style):
    explicit = style.get("sampling_brow_divisions", 0)
    if explicit:
        return explicit
    return 32 if style.get("sampling_first_pass", 0) and design["kind"] == "angled" else 0


def brow_polygons(design, span, base_width, sampling_divisions=0):
    kind = design["kind"]
    if isinstance(sampling_divisions, bool) or sampling_divisions not in (0, 16, 24, 32, 40, 48, 56, 64):
        raise ValueError("Invalid brow sampling divisions")
    if sampling_divisions and kind in ("symbols", "curl"):
        raise ValueError("This brow family requires its own sampling plan")
    length = span * design["length_scale"]
    width = base_width * design["width_scale"]
    offset = (span - length) * 0.5
    if kind == "symbols":
        count = design["detail_count"]
        step = length / count
        polygons = [[(i * step, 0), (i * step + step * 0.68, 0), (i * step + step * 0.34, width * 2)] for i in range(count)]
    elif kind == "curl":
        polygons = [brow_curl(length, width, design["curl_turns"])]
    else:
        divisions = int(sampling_divisions) if sampling_divisions else 64
        samples = {i / divisions for i in range(divisions + 1)} | {design["peak"]}
        count = design["detail_count"]
        if kind in ("multipoint", "fringe"):
            samples |= {i / (count * 2) for i in range(count * 2 + 1)}
        lower, upper = [], []
        for t in sorted(samples):
            peak = design["peak"]
            arch_factor = t / peak if t <= peak else (1 - t) / (1 - peak)
            if kind != "angled":
                arch_factor = math.sin(math.pi * t)
            rise = 0 if kind in ("straight", "leaf", "oval") else length * design["arch_ratio"] * arch_factor
            head = 1 - 0.5 * design["head_rounding"]
            tail = 1 - 0.5 * design["tail_rounding"]
            if kind == "oval":
                head = tail = 0.5
            band = width * (2 * t) ** head * (2 * (1 - t)) ** tail
            detail = 0
            if kind in ("multipoint", "fringe"):
                triangle = 1 - abs(2 * ((t * count) % 1) - 1)
                detail = band * design["detail_amplitude"] * triangle
                if kind == "fringe":
                    detail *= 1 - design["detail_variation"] * (0.5 + 0.5 * math.sin((int(t * count) + 1) * 2.399))
            lower.append((length * t, rise - band / 2 - (detail * 0.35 if kind == "fringe" else 0)))
            upper.append((length * t, rise + band / 2 + detail))
        polygons = [brow_clean(lower + upper[::-1])]
    result = [brow_clean([(x + offset, y) for x, y in polygon]) for polygon in polygons]
    for polygon in result:
        brow_validate_polygon(polygon)
    return result
