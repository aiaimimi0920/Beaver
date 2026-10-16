"""Packaging-time displacement-plane fit; preserves existing lens relief."""

import math
import numpy as np


def component_box(c):
    H = c["height_m"]
    x = [
        (-c["iris_half_width"], c["iris_half_width"]),
        (-1.08 * c["pupil_half_width"], 1.08 * c["pupil_half_width"]),
        (-c["pupil_accent_width"], c["pupil_accent_width"]),
    ]
    y = [
        (-c["iris_half_height"], c["iris_half_height"]),
        (-1.08 * c["pupil_half_height"], 1.08 * c["pupil_half_height"]),
        (0.010 - c["pupil_accent_height"], 0.010 + c["pupil_accent_height"]),
    ]
    # Catchlight source uses local offsets relative to the iris center.
    for side in (-1, 1):
        x.append(
            (
                side * c["catchlight_offset_x"] - c["catchlight_half_width"],
                side * c["catchlight_offset_x"] + c["catchlight_half_width"],
            )
        )
    y.append(
        (
            c["catchlight_offset_y"] - c["catchlight_half_height"],
            c["catchlight_offset_y"] + c["catchlight_half_height"],
        )
    )
    return [
        [H * min(a for a, b in x), H * max(b for a, b in x)],
        [H * min(a for a, b in y), H * max(b for a, b in y)],
    ]


def fit_plane(c, old, new, samples=257):
    center = [
        c["height_m"] * c["iris_center_x"],
        c["origin_y_m"] + c["height_m"] * c["iris_center_y"],
    ]
    rows, before, after = [], [], []
    for upper in (False, True):
        for i in range(samples):
            x, y = old["eye_contour"](1, -1 + 2 * i / (samples - 1), upper)
            rows.append([x - center[0], y - center[1], 1.0])
            before.append(old["rim_depth"](x, y))
            after.append(new["rim_depth"](x, y))
    matrix = np.asarray(rows)
    before, after = np.asarray(before), np.asarray(after)
    coefficients = np.linalg.lstsq(matrix, after - before, rcond=None)[0]
    baseline = np.linalg.lstsq(matrix, before, rcond=None)[0]
    residual_old = before - matrix @ baseline
    residual_new = after - matrix @ (baseline + coefficients)
    box = component_box(c)
    bound = max(abs(coefficients @ [x, y, 1.0]) for x in box[0] for y in box[1])
    return dict(
        center=center,
        coefficients=coefficients.tolist(),
        domain_relative_m=box,
        continuous_plane_displacement_bound_m=float(bound + 1e-12),
        displacement_fit_residual_max_m=float(max(abs(residual_new - residual_old))),
        baseline_rim_plane_residual_max_m=float(max(abs(residual_old))),
        final_rim_plane_residual_max_m=float(max(abs(residual_new))),
        displacement_tilt_degrees=math.degrees(
            math.atan(math.hypot(*coefficients[:2]))
        ),
        samples_per_arc=samples,
        clearance="Not certified by plane fitting; native limbus/socket/lid and posed checks required",
    )
