"""Backtrack optional fairing without adding new frontal boundary reversals."""

import math


def outline_turns(vertices, ids):
    points = [vertices[i] for i in ids]
    if len(points) < 3 or any(not math.isfinite(v) for p in points for v in p):
        raise ValueError("Invalid outline chain")
    slopes = []
    for a, b in zip(points, points[1:]):
        dx, dz = b[0] - a[0], b[2] - a[2]
        if dz <= 1e-10 or math.hypot(dx, dz) <= 1e-10:
            raise ValueError("Outline chain lost height order or segment length")
        slopes.append(math.atan2(dx, dz))
    return [b - a for a, b in zip(slopes, slopes[1:])]


def guard_outline(original, proposed, ids, reference):
    if len(original) != len(proposed):
        raise ValueError("Fairing changed vertex count")
    limits = [max(0.0, x) for x in outline_turns(reference, ids)]
    # Both fairing passes compare to the same pre-fairing geometry.
    outline_turns(original, ids)
    rejected = []
    for factor in (1.0, 0.5, 0.25, 0.125, 0.0625, 0.03125, 0.0):
        candidate = [
            tuple(a + factor * (b - a) for a, b in zip(p, q))
            for p, q in zip(original, proposed)
        ]
        if any(not math.isfinite(v) for p in candidate for v in p):
            rejected.append({"factor": factor, "reason": "nonfinite_coordinate"})
            continue
        try:
            turns = outline_turns(candidate, ids)
        except ValueError as error:
            rejected.append({"factor": factor, "reason": str(error)})
            continue
        if all(t <= bound + 1e-9 for t, bound in zip(turns, limits)):
            displacement = max(
                (math.dist(p, q) for p, q in zip(original, candidate)), default=0
            )
            return candidate, {
                "outline_backtracking_factor": factor,
                "outline_fairing_status": (
                    "applied" if factor else "skipped_no_safe_shape_step"
                ),
                "maximum_displacement_m": displacement,
                "outline_turns_radians": turns,
                "rejected_outline_steps": rejected,
                "reference_turn_limits_radians": limits,
                "scope": "Front outer-row projection only; all downstream native guards remain required",
            }
        rejected.append(
            {
                "factor": factor,
                "turn_indices": [
                    i
                    for i, (t, bound) in enumerate(zip(turns, limits))
                    if t > bound + 1e-9
                ],
            }
        )
    raise ValueError("No valid outline-preserving fairing proposal")
