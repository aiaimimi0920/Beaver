"""Opt-in edits at guarded real constructor callsites; default text is untouched."""
import ast


def replace(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Constructor source boundary changed: " + old[:90])
    return source.replace(old, new, 1)


def function_edit(source, name, edits):
    definitions = [node for node in ast.parse(source).body if isinstance(node, ast.FunctionDef) and node.name == name]
    if len(definitions) != 1:
        raise ValueError("Unique constructor function required: " + name)
    node = definitions[0]
    lines = source.splitlines(keepends=True)
    text = "".join(lines[node.lineno - 1:node.end_lineno])
    for before, after in edits:
        text = replace(text, before, after)
    lines[node.lineno - 1:node.end_lineno] = [text]
    return "".join(lines)


def instrument(sources, recipe):
    out = dict(sources)
    face = function_edit(out["face.py"], "feature_chin_front_strip", [
        ("columns, sample):", "columns, sample, construction=None):"),
        ("    low = feature_chin_row_columns(row, columns)",
         "    _construction_start = len(vertices)\n    low = feature_chin_row_columns(row, columns)"),
        ("    return faces", "    if construction is not None:\n        construction.chin_strip(vertices, row, columns, low, high, _construction_start)\n    return faces")])
    face = function_edit(face, "feature_chin_side_strip", [
        ("columns, side_path):", "columns, side_path, construction=None):"),
        ("    left,middle,right = range(len(vertices),len(vertices)+3)",
         "    _construction_start = len(vertices)\n    left,middle,right = range(len(vertices),len(vertices)+3)"),
        ("    return faces,", "    if construction is not None:\n        construction.side_strip(vertices, _construction_start, [left, middle, right])\n    return faces,")])
    shell = function_edit(out["shell.py"], "add_posterior_return", [
        ("height, origin, depth_origin):", "height, origin, depth_origin, construction=None):"),
        ("    return new_vertices, new_faces, ids, len(new_faces) - len(faces)",
         "    if construction is not None:\n        construction.posterior(vertices, new_vertices, mapping)\n    return new_vertices, new_faces, ids, len(new_faces) - len(faces)")])
    edits = [
        ("    verts = []\n    chart_reference = []", f"    _construction = ConstructionJournal({recipe!r}, levels, lateral, calibration=C, style=STYLE)\n    verts = []\n    chart_reference = []"),
        ("            verts.append(feature_flow_surface_point(y, q))", "            verts.append(feature_flow_surface_point(y, q))\n"
         "            _construction.chart_point(j, i, verts, [y, q], [mapped_y, mapped_q])"),
        ("    # Feature-aligned patches replace the uniform lattice around eyes and mouth.",
         "    _construction.chart_rows(rows, cols)\n"
         "    # Feature-aligned patches replace the uniform lattice around eyes and mouth."),
        ("feature_flow_grid_point(y, col, lateral, nominal)))", "feature_flow_grid_point(y, col, lateral, nominal), construction=_construction))"),
        ("    lip_joint_reports = []", "    _construction.checkpoint('front_grid', verts, shell_faces)\n    lip_joint_reports = []"),
        ("        rings.append(boundary)", "        rings.append(boundary)\n        _construction.patch(kind, rings, closed, verts)"),
        ('    patch(eye_boundary, "eye", True)', '    patch(eye_boundary, "eye", True)\n    _construction.checkpoint("eye_patch", verts, shell_faces)'),
        ('    patch(mouth_boundary, "mouth", False)', '    patch(mouth_boundary, "mouth", False)\n    _construction.checkpoint("mouth_patch", verts, shell_faces)'),
        ("    front_count = len(shell_faces)", "    _construction.outer_stage(outer_flow_report, verts, shell_faces)\n    front_count = len(shell_faces)"),
        ("    underside_count = len(shell_faces) - front_count", "    _construction.underside(under_depth_rows, verts)\n"
         "    _construction.checkpoint('underside', verts, shell_faces)\n    underside_count = len(shell_faces) - front_count"),
        ("feature_chin_side_strip(verts, levels, cols, side_path)", "feature_chin_side_strip(verts, levels, cols, side_path, construction=_construction)"),
        ("    shell_faces.extend(side_flow_faces)", "    shell_faces.extend(side_flow_faces)\n    _construction.checkpoint('side_strip', verts, shell_faces)"),
        ("            verts, shell_faces, C, H, Y0, Z0\n        )", "            verts, shell_faces, C, H, Y0, Z0, construction=_construction\n        )"),
        ("    final_boundary_config = dict(fair_config)", "    _construction.checkpoint('posterior_return', verts, shell_faces)\n    final_boundary_config = dict(fair_config)"),
        ("    verts, shell_faces, flow_chains, flow_remap = compact_flow_vertices(verts, shell_faces, flow_chains)",
         "    _construction.before_compact(verts, shell_faces)\n"
         "    verts, shell_faces, flow_chains, flow_remap = compact_flow_vertices(verts, shell_faces, flow_chains)\n"
         "    _construction_packet = _construction.compact(verts, shell_faces, flow_remap)"),
        ('    face = mesh("Face shell editable half", verts, shell_faces, SKIN, mirror=True)',
         '    face = mesh("Face shell editable half", verts, shell_faces, SKIN, mirror=True)\n'
         '    bind_half(face, _construction_packet)'),
        ('        "template_outline_trace": outline_trace,', '        "construction_identity": _construction_packet,\n        "template_outline_trace": outline_trace,')]
    shell = function_edit(shell, "build_face", edits)
    shell = replace(shell, "def build_face():", "from construction_native import bind_half\nfrom construction_journal import ConstructionJournal\n\ndef build_face():")
    out.update({"face.py": face, "shell.py": shell})
    return out
