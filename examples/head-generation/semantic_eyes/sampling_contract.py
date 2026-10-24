"""Explicit first-pass lash sampling controls; no post-export decimation."""
import math

SAMPLING_LIMITS = {
    "sampling_first_pass": (0, 0, 1),
    "sampling_surface_subdivision": (1, 0, 1),
    "sampling_budget_trial": (0, 0, 1),
    "sampling_section_blink": (0, 0, 1),
    "sampling_ear_budget": (0, 0, 1),
    "sampling_surface_fairing": (0, 0, 1),
    "sampling_detail_budget": (0, 0, 1),
    "sampling_outline_refinement": (0, 0, 1),
    "sampling_density_layout": (0, 0, 1),
    "sampling_rim_quality_guard": (0, 0, 0),
    "sampling_shell_columns": (13, 9, 13),
    "sampling_brow_divisions": (0, 0, 64),
    "sampling_lid_stations": (0, 0, 33),
    "sampling_iris_samples": (0, 0, 64),
    "sampling_iris_rings": (0, 0, 4),
    "sampling_pocket_samples": (0, 0, 64),
    "sampling_main_stations": (21, 9, 21),
    "sampling_companion_stations": (17, 5, 17),
    "sampling_lateral_gap": (0.065, 0.065, 0.22),
    "sampling_wing_gap": (0.025, 0.025, 0.22),
}

# Shared first-pass game-face recipe. Budget is measured on the complete exported Face.
# Actual count is composition-dependent; this is not a post-export simplifier.
GAME_FACE_SAMPLING = {
    "sampling_first_pass": 1, "sampling_density_layout": 1,
    "sampling_shell_columns": 9, "sampling_brow_divisions": 16,
    "sampling_lid_stations": 17, "sampling_iris_samples": 24,
    "sampling_iris_rings": 2, "sampling_pocket_samples": 24,
    "sampling_main_stations": 9, "sampling_companion_stations": 5,
    "sampling_lateral_gap": .22, "sampling_wing_gap": .22,
    "sampling_surface_subdivision": 0, "sampling_section_blink": 1,
    "sampling_ear_budget": 1, "sampling_budget_trial": 0,
}

def validate_sampling(request):
    if not isinstance(request, dict) or set(request) - SAMPLING_LIMITS.keys():
        raise ValueError("Expected documented first-pass sampling controls")
    result = {k: limits[0] for k, limits in SAMPLING_LIMITS.items()}
    for key, value in request.items():
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ValueError("Sampling control must be finite: " + key)
        if not SAMPLING_LIMITS[key][1] <= value <= SAMPLING_LIMITS[key][2]:
            raise ValueError("Sampling control outside bounds: " + key)
        if key in ("sampling_iris_samples", "sampling_pocket_samples") and value not in (0, 24, 32, 40, 48, 56, 64):
            raise ValueError("Ocular samples must retain cardinal angles and stay within validated trial bounds")
        if key == "sampling_iris_rings" and value not in (0, 2, 3, 4):
            raise ValueError("Iris sampling must retain both bevel radii")
        if key == "sampling_brow_divisions" and value not in (0, 12, 16, 24, 32, 40, 48, 56, 64):
            raise ValueError("Unsupported brow sampling division count")
        if key == "sampling_shell_columns" and value not in (9, 13):
            raise ValueError("Shell columns must preserve the documented semantic anchors")
        if key in ("sampling_outline_refinement", "sampling_detail_budget", "sampling_surface_fairing", "sampling_ear_budget", "sampling_section_blink", "sampling_budget_trial", "sampling_surface_subdivision", "sampling_first_pass", "sampling_rim_quality_guard", "sampling_density_layout") and value not in (0, 1):
            raise ValueError("First-pass mode must be numeric zero or one")
        if key == "sampling_lid_stations" and value not in (0, 17, 25, 33):
            raise ValueError("Lid sampling must preserve corners and midpoint")
        if key.endswith("_stations") and value != 0 and (value != int(value) or int(value) % 2 != 1):
            raise ValueError("Station counts must be odd integers, preserving the midpoint")
        result[key] = int(value) if key.endswith("_stations") else float(value)
    if (result["sampling_surface_subdivision"] == 0 or result["sampling_surface_fairing"] or result["sampling_budget_trial"] or result["sampling_detail_budget"] or result["sampling_outline_refinement"]) and not result["sampling_density_layout"]:
        raise ValueError("Source budget controls require density layout")
    if result["sampling_surface_fairing"] and result["sampling_surface_subdivision"] != 0:
        raise ValueError("Source cage fairing requires unsubdivided output")
    if result["sampling_outline_refinement"] and result["sampling_surface_subdivision"]:
        raise ValueError("Outline refinement requires the unsubdivided source cage")
    return result


def lid_station_plan(count, angular=False):
    count = int(validate_sampling({"sampling_lid_stations": count})["sampling_lid_stations"] or 33)
    stations = {-1 + 2*i/(count-1) for i in range(count)}
    if angular:
        stations.update((-.55, .55))
    return tuple(sorted(stations))
