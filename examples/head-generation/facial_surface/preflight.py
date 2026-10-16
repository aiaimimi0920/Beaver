"""CPU preparation and bounded support diagnostics for the experimental field."""

import ast
from study import scope, fit_profile
from transverse_fit import fit_response, continuous_base_bound
from hypothesis import validate
from runtime_field import base_delta
from ocular_fit import fit_plane
from interval_bounds import aperture_gap_bound, surface_motion_bound


def prepare(source, calibration, parameters):
    p = validate(parameters)
    fit = fit_profile(source, calibration)
    old = scope(source, calibration, fit, 0)
    node = next(
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef) and n.name == "eye_contour"
    )
    exec(
        compile(ast.Module(body=[node], type_ignores=[]), "<eye contour>", "exec"), old
    )
    data = dict(parameters=p, segments=fit["segments"], transverse=fit_response(0.0001))
    changed = dict(old)

    def correction(x, y):
        return base_delta(x, y, calibration, old["curve"], data)

    changed["rim_depth"] = (
        lambda x, y: old["rim_depth"](x, y)
        + correction(x, y)
        + p["orbit_shallowing"] * (old["base_depth"](x, y) - old["rim_depth"](x, y))
    )
    data["ocular_plane"] = fit_plane(calibration, old, changed)
    # Profile fit's bound is already in meters, independently of applied strength.
    bound = continuous_base_bound(
        calibration,
        old["curve"],
        fit["certified_max_profile_offset_m"],
        p,
        data["transverse"],
    )
    aperture = aperture_gap_bound(calibration)
    limits = surface_motion_bound(calibration, bound, p["orbit_shallowing"])
    limits["aperture_rim_displacement_bound_m"] = bound + p["orbit_shallowing"] * max(
        abs(aperture["minimum_m"]), abs(aperture["maximum_m"])
    )
    # Medial crease correction is disjoint from the orbital support band.
    from interval_bounds import curve as interval_curve, I

    c = calibration
    width_max = interval_curve(I(0.1, 0.9), 1, c["width_stations"]).hi * c["height_m"]
    inner = c["height_m"] * (
        c["eye_center_x"] - c["eye_half_width"] * (1 + c["orbital_lateral_fade"])
    )
    assert 0.2 * width_max < inner, "Medial correction overlaps eye support"
    differences = []
    for i in range(80):
        h = I(0.1 + i * 0.01, 0.1 + (i + 1) * 0.01)
        differences.append(
            (
                interval_curve(h, 1, c["continuous_back_profile"])
                - interval_curve(h, 1, c["continuous_front_profile"])
            )
            .absolute()
            .hi
        )
    center_bound = (
        c["height_m"]
        * abs(c["continuous_lateral_linear"])
        * 0.2
        * 27
        / 256
        * max(differences)
    )
    center_bound *= min(1.0, 2 * p["axial_smoothing"])
    limits["maximum_depth_displacement_m"] = max(
        limits["maximum_depth_displacement_m"], bound + center_bound
    )
    limits["medial_correction_bound_m"] = center_bound
    limits["medial_support_disjoint_from_orbit"] = True
    limits["medial_correction_scope"] = (
        "C2 radial collar, q<0.2; vertical collar h=0.1..0.9; nasal relief retained"
    )
    return data, dict(
        base_bound_m=bound + center_bound,
        aperture=aperture,
        surface=limits,
        ocular_plane=data["ocular_plane"],
        acceptance="Experimental native comparison only; clearances and posed geometry unverified",
    )
