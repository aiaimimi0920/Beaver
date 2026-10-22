"""Small shared analytic functions emitted into native and CPU analysis sources."""


def researched_muzzle(x, y):
    d = C["nose_design"]
    if d["kind"] != "pad-muzzle":
        return 0.0
    h = (y - Y0) / H
    # Upper muzzle is nasal/perinasal, not a replacement animal jaw or mouth.
    lo, hi = 0.17, 0.36
    if h <= lo or h >= hi:
        return 0.0
    t = (h - lo) / (hi - lo)
    vertical = 16 * t * t * (1 - t) * (1 - t)
    lateral = max(0.0, 1.0 - (x / (H * d["muzzle_width"])) ** 2) ** 2
    return H * d["muzzle_projection"] * vertical * lateral
