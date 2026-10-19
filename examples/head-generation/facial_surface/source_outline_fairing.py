"""Apply the explicit broad-chin template's extra optional-fairing guard."""

import inspect
from source_outline_guard import outline_turns, guard_outline
from source_mouth_templates import replace_once


def adapt_outline_fairing(source):
    helpers = inspect.getsource(outline_turns) + "\n" + inspect.getsource(guard_outline)
    source = replace_once(
        source, "from face import *\n", "from face import *\n\n" + helpers + "\n"
    )
    source = replace_once(
        source,
        '    return result, {"selected_vertices": len(selected), "maximum_displacement_m": maximum, "proposed_displacement_m": proposed}',
        '    result, outline_report = guard_outline(vertices, result, config["outline_chain_ids"], config["outline_reference"])\n'
        '    return result, {"selected_vertices": len(selected), "maximum_displacement_m": maximum, "proposed_displacement_m": proposed, **outline_report}',
    )
    source = replace_once(
        source,
        "    verts, jaw_fairing = fair_jaw_boundary(verts, shell_faces, C, H, Y0, Z0)",
        "    fair_config = dict(C)\n"
        '    fair_config["outline_chain_ids"] = [r["row"]*cols+cols-1 for r in outline_trace["construction"]]\n'
        '    fair_config["outline_reference"] = list(verts)\n'
        "    verts, jaw_fairing = fair_jaw_boundary(verts, shell_faces, fair_config, H, Y0, Z0)",
    )
    return replace_once(
        source,
        "    final_boundary_config = dict(C)",
        "    final_boundary_config = dict(fair_config)",
    )
