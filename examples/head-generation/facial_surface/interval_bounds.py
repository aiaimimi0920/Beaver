"""Outward-rounded interval envelopes for the inherited analytic support field.

Packaging-time CPU only. A small coefficient margin covers floating arithmetic;
this is a conservative engineering certificate, not formal theorem proving.
"""

import math


class I:
    def __init__(self, lo, hi=None):
        self.lo = float(lo)
        self.hi = float(lo if hi is None else hi)
        if self.lo > self.hi:
            raise ValueError("Reversed interval")

    def __add__(self, other):
        o = as_i(other)
        return outward(self.lo + o.lo, self.hi + o.hi)

    __radd__ = __add__

    def __neg__(self):
        return I(-self.hi, -self.lo)

    def __sub__(self, o):
        return self + -as_i(o)

    def __rsub__(self, o):
        return as_i(o) + -self

    def __mul__(self, o):
        o = as_i(o)
        v = [a * b for a in (self.lo, self.hi) for b in (o.lo, o.hi)]
        return outward(min(v), max(v))

    __rmul__ = __mul__

    def __truediv__(self, o):
        o = as_i(o)
        if o.lo <= 0 <= o.hi:
            raise ValueError("Interval division across zero")
        return self * outward(1 / o.hi, 1 / o.lo)

    def __pow__(self, p):
        if self.lo < 0 and p != int(p):
            raise ValueError("Fractional power of negative interval")
        values = [self.lo**p, self.hi**p]
        if self.lo <= 0 <= self.hi and p > 0:
            values.append(0.0)
        return outward(min(values), max(values))

    def absolute(self):
        return I(
            0 if self.lo <= 0 <= self.hi else min(abs(self.lo), abs(self.hi)),
            max(abs(self.lo), abs(self.hi)),
        )

    def clamp(self, lo, hi):
        return I(min(hi, max(lo, self.lo)), min(hi, max(lo, self.hi)))

    def exp(self):
        return outward(math.exp(self.lo), math.exp(self.hi))


def as_i(x):
    return x if isinstance(x, I) else I(x)


def outward(lo, hi):
    return I(math.nextafter(lo, -math.inf), math.nextafter(hi, math.inf))


def curve(v, axis, points):
    v = as_i(v)
    xs = [p[0] for p in points]
    ys = [p[axis] for p in points]
    slopes = [(b - a) / (y - x) for x, y, a, b in zip(xs, xs[1:], ys, ys[1:])]
    ds = (
        [slopes[0]]
        + [0 if a * b <= 0 else 2 * a * b / (a + b) for a, b in zip(slopes, slopes[1:])]
        + [slopes[-1]]
    )
    lo = max(xs[0], min(xs[-1], v.lo))
    hi = max(xs[0], min(xs[-1], v.hi))
    ranges = []
    for j in range(len(xs) - 1):
        if hi < xs[j] or lo > xs[j + 1]:
            continue
        h = xs[j + 1] - xs[j]
        t = I(max(lo, xs[j]), min(hi, xs[j + 1]))
        t = (t - xs[j]) / h
        a = 2 * ys[j] - 2 * ys[j + 1] + h * (ds[j] + ds[j + 1])
        b = -3 * ys[j] + 3 * ys[j + 1] - h * (2 * ds[j] + ds[j + 1])
        value = ((a * t + b) * t + h * ds[j]) * t + ys[j]
        ranges.append(value)
    return outward(min(r.lo for r in ranges) - 1e-12, max(r.hi for r in ranges) + 1e-12)


def raw_without_origin(x, h, c):
    """Inherited base_depth−Z0, including the low-height section blend."""
    x, h = as_i(x), as_i(h)
    H = c["height_m"]
    lateral = x.absolute() / H
    width = (c["height_m"] * curve(h, 1, c["width_stations"])).clamp(0.001, math.inf)
    q = (x.absolute() / width).clamp(0, 1)
    front = curve(h, 1, c["continuous_front_profile"])
    back = curve(h, 1, c["continuous_back_profile"])
    falloff = (
        c["continuous_lateral_linear"] * q
        + (1 - c["continuous_lateral_linear"]) * q ** c["continuous_lateral_power"]
    )
    consistent = front * (1 - falloff) + back * falloff
    if h.lo < 0.1:
        sections = [
            (height, curve(q * points[-1][0], 1, points))
            for height, points in c["cross_sections"]
        ]
        # Hermite shape interpolation is monotone between neighboring values.
        # Their complete interval hull bounds every interpolated section.
        section = I(min(v.lo for _, v in sections), max(v.hi for _, v in sections))
        b = ((h - 0.02) / 0.08).clamp(0, 1)
        blend = I(b.lo * b.lo * (3 - 2 * b.lo), b.hi * b.hi * (3 - 2 * b.hi))
        consistent = section * (1 - blend) + consistent * blend
    z = H * consistent
    points = c["nose_stations"]
    if h.hi > points[0][0] and h.lo < points[-1][0]:
        active = I(max(h.lo, points[0][0]), min(h.hi, points[-1][0]))
        nh = H * curve(active, 1, points)
        nw = H * curve(active, 2, points)
        t = (x.absolute() / nw).clamp(0, 1)
        blend = (
            (c["nose_flare_end_height"] - active)
            / (c["nose_flare_end_height"] - c["nose_flare_start_height"])
        ).clamp(0, 1)
        nose = nh * (
            curve(t, 1, c["nose_lateral_profile"]) * (1 - blend)
            + curve(t, 1, c["nose_basal_profile"]) * blend
        )
        if h.lo < points[0][0] or h.hi > points[-1][0]:
            nose = I(min(0, nose.lo), max(0, nose.hi))
        z += nose
    for prefix in ("lower_lip", "upper_lip"):
        lateral_shape = (
            -((lateral / c[prefix + "_lateral_width"]) ** c[prefix + "_lateral_power"])
        ).exp()
        vertical = (-(((h - c[prefix + "_height"]) / c[prefix + "_width"]) ** 2)).exp()
        z += H * c[prefix + "_relief"] * lateral_shape * vertical
    z -= (
        H
        * c["mouth_seam_recess"]
        * (-((x / 0.02) ** 4)).exp()
        * (-(((h - c["mouth_height"]) / c["mouth_seam_width"]) ** 2)).exp()
    )
    shoulder = 1 - (-((lateral / c["lip_shoulder_center_fade"]) ** 2)).exp()
    shoulder *= (
        -(
            (
                (lateral - c["lip_shoulder_lateral_center"])
                / c["lip_shoulder_lateral_width"]
            )
            ** 2
        )
        - ((h - c["lip_shoulder_height"]) / c["lip_shoulder_height_width"]) ** 2
    ).exp()
    return z + H * c["lip_shoulder_relief"] * shoulder


def rim_without_origin(t, c):
    t = as_i(t)
    return c["height_m"] * (
        c["eye_inner_depth"] * (1 - t) / 2
        + c["eye_outer_depth"] * (1 + t) / 2
        + c["eye_rim_vertical_bulge"] * (1 - t * t).clamp(0, math.inf)
    )


def aperture_gap_bound(c, subdivisions=2048):
    bounds = []
    for upper in (False, True):
        for i in range(subdivisions):
            t = I(-1 + 2 * i / subdivisions, -1 + 2 * (i + 1) / subdivisions)
            arch = c["eye_upper_arch"] if upper else -c["eye_lower_arch"]
            horizontal = (
                c["eye_horizontal_power"] if upper else c["eye_lower_horizontal_power"]
            )
            power = c["eye_upper_power"] if upper else c["eye_lower_power"]
            shape = (1 - t.absolute() ** horizontal).clamp(0, 1) ** power
            h = c["eye_corner_y"] + c["eye_corner_slope"] * t + arch * shape
            x = c["height_m"] * (c["eye_center_x"] + c["eye_half_width"] * t)
            gap = raw_without_origin(x, h, c) - rim_without_origin(t, c)
            bounds.append(gap)
    return dict(
        minimum_m=min(b.lo for b in bounds),
        maximum_m=max(b.hi for b in bounds),
        subdivisions_per_arc=subdivisions,
        scope="continuous aperture-boundary support gap, excluding later mesh operations",
    )


def orbital_weight(x, h, c):
    t = (as_i(x) / c["height_m"] - c["eye_center_x"]) / c["eye_half_width"]
    cy = c["eye_corner_y"] + c["eye_corner_slope"] * t
    relative = as_i(h) - cy
    weights = []
    for upper in (False, True):
        if upper and relative.hi < 0 or not upper and relative.lo > 0:
            continue
        vertical = (
            I(max(0, relative.lo), max(0, relative.hi))
            if upper
            else I(min(0, relative.lo), min(0, relative.hi))
        )
        arch = c["eye_upper_arch"] if upper else c["eye_lower_arch"]
        power = c["eye_upper_power"] if upper else c["eye_lower_power"]
        horizontal = (
            c["eye_horizontal_power"] if upper else c["eye_lower_horizontal_power"]
        )
        # Both summands are analytically nonnegative; outward rounding at zero
        # may produce a negative subnormal lower endpoint. Intersect that
        # known domain before applying the square root.
        squared = (
            t.absolute() ** horizontal + (vertical / arch).absolute() ** (1 / power)
        ).clamp(0, math.inf)
        r = squared**0.5
        weights.append((-(((r - 1) / c["orbital_band_width"]) ** 2)).exp())
    w = I(min(v.lo for v in weights), max(v.hi for v in weights))
    a = (
        (1 + c["orbital_lateral_fade"] - t.absolute()) / c["orbital_lateral_fade"]
    ).clamp(0, 1)
    fade = I(a.lo * a.lo * (3 - 2 * a.lo), a.hi * a.hi * (3 - 2 * a.hi))
    return (w * fade).clamp(0, 1)


def surface_motion_bound(c, base_bound, beta, steps=80):
    """Continuous field bound over 0<=height<=H and 0<=x<=max width."""
    xmax = c["height_m"] * max(row[1] for row in c["width_stations"])
    maximum = 0.0
    for j in range(steps):
        h = I(j / steps, (j + 1) / steps)
        for i in range(steps):
            x = I(xmax * i / steps, xmax * (i + 1) / steps)
            w = orbital_weight(x, h, c)
            t = (x / c["height_m"] - c["eye_center_x"]) / c["eye_half_width"]
            gap = raw_without_origin(x, h, c) - rim_without_origin(t, c)
            weighted = gap.absolute().hi * w.hi
            maximum = max(maximum, weighted)
    return dict(
        maximum_depth_displacement_m=base_bound + beta * maximum + 1e-9,
        base_component_bound_m=base_bound,
        weighted_orbit_gap_bound_m=maximum,
        domain_blender_x_height_m=[
            [0, xmax],
            [c["origin_y_m"], c["origin_y_m"] + c["height_m"]],
        ],
        subdivisions_per_axis=steps,
        scope="analytic skin/support depth only; no claim for downstream fitting or collisions",
    )
