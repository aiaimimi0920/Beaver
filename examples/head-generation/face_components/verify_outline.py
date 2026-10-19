"""Bind the broad-chin optional-fairing contract to native reports."""

import math


def verify_outline_guard(snapshot, report):
    if not any(
        t.get("id") == "broad-blunt-chin-v3"
        for t in snapshot.get("family_templates", [])
    ):
        return
    for key, limit in [
        ("jaw_boundary_fairing", 0.018 * 0.236),
        ("final_return_boundary_fairing", 0.008 * 0.236),
    ]:
        r = report.get(key, {})
        factor = r.get("outline_backtracking_factor")
        if isinstance(factor, bool) or factor not in (
            1,
            0.5,
            0.25,
            0.125,
            0.0625,
            0.03125,
            0,
        ):
            raise ValueError("Missing or invalid outline backtracking report")
        status = "applied" if factor else "skipped_no_safe_shape_step"
        if r.get("outline_fairing_status") != status:
            raise ValueError("Outline fairing status mismatch")
        turns, bounds = r.get("outline_turns_radians"), r.get(
            "reference_turn_limits_radians"
        )
        if (
            not isinstance(turns, list)
            or not isinstance(bounds, list)
            or len(turns) != 4
            or len(bounds) != 4
        ):
            raise ValueError("Incomplete outline chain report")
        if any(
            not isinstance(v, (int, float))
            or isinstance(v, bool)
            or not math.isfinite(v)
            for v in turns + bounds
        ):
            raise ValueError("Nonfinite outline report")
        if any(bound != 0 or turn > bound + 1e-9 for turn, bound in zip(turns, bounds)):
            raise ValueError("New front outline reversal")
        displacement = r.get("maximum_displacement_m")
        if (
            not isinstance(displacement, (int, float))
            or isinstance(displacement, bool)
            or not math.isfinite(displacement)
            or not 0 <= displacement <= limit + 1e-12
        ):
            raise ValueError("Outline displacement budget exceeded")
        if not factor and displacement != 0:
            raise ValueError("Skipped fairing must have zero displacement")
