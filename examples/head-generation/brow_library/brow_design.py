"""Versioned, bounded brow-only designs; none preserves the legacy pipeline."""

import math

KINDS = {
    "straight", "arc", "angled", "leaf", "oval", "curl",
    "multipoint", "fringe", "symbols",
}
LIMITS = {
    "length_scale": (1.0, 0.12, 1.05),
    "width_scale": (1.0, 0.25, 3.0),
    "arch_ratio": (0.06, 0.0, 0.16),
    "peak": (0.6, 0.3, 0.75),
    "head_rounding": (0.5, 0.0, 1.0),
    "tail_rounding": (0.0, 0.0, 1.0),
    "detail_count": (3, 2, 12),
    "detail_amplitude": (0.6, 0.0, 1.0),
    "detail_variation": (0.4, 0.0, 1.0),
    "curl_turns": (1.0, 0.75, 1.2),
    "clearance_m": (0.00065, 0.0003, 0.0015),
    "thickness_m": (0.0002, 0.0001, 0.0004),
}


def validate(design):
    if design is None:
        return None
    allowed = set(LIMITS) | {"kind", "schema_version"}
    if not isinstance(design, dict) or set(design) - allowed:
        raise ValueError("Unknown brow design controls")
    if type(design.get("schema_version")) is not int or design["schema_version"] != 1:
        raise ValueError("Expected brow design schema 1")
    if not isinstance(design.get("kind"), str) or design["kind"] not in KINDS:
        raise ValueError("Unknown brow geometry kind")
    result = {"schema_version": 1, "kind": design["kind"]}
    for key, (default, low, high) in LIMITS.items():
        value = design.get(key, default)
        if (
            isinstance(value, bool)
            or not isinstance(value, (int, float))
            or not math.isfinite(value)
            or not low <= value <= high
        ):
            raise ValueError("Invalid bounded brow control: " + key)
        if key == "detail_count" and int(value) != value:
            raise ValueError("Brow detail count must be integral")
        result[key] = int(value) if key == "detail_count" else float(value)
    if result["thickness_m"] >= result["clearance_m"]:
        raise ValueError("Brow thickness must stay inside positive clearance")
    return result
