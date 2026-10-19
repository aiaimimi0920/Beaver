"""Observe template shell boundary construction without changing geometry."""

from source_mouth_templates import replace_once


def instrument_outline(source):
    source = replace_once(
        source,
        "    verts = []\n    for j in range(rows):",
        "    verts = []\n    outline_trace = {'construction': [], 'underside': []}\n    for j in range(rows):",
    )
    source = replace_once(
        source,
        "            x, zz = jaw_return_position(x, yy, zz, q, depth(width(yy), yy))\n            verts.append(coord(x, yy, zz))",
        "            before_return = coord(x, yy, zz)\n"
        "            x, zz = jaw_return_position(x, yy, zz, q, depth(width(yy), yy))\n"
        "            if i == cols - 1 and (y - Y0) / H <= 0.22:\n"
        "                outline_trace['construction'].append({'row': j, 'h': (y-Y0)/H, 'before': before_return, 'after': coord(x, yy, zz)})\n"
        "            verts.append(coord(x, yy, zz))",
    )
    source = replace_once(
        source,
        "            verts.append(\n                (\n                    half_width * q,",
        "            if i == cols - 1:\n"
        "                outline_trace['underside'].append({'lift': lift, 'spread': spread, 'base_limit': base[-1][0]*spread, 'layer_limit': width(Y0+H*lift)*C['chin_return_boundary_inset'], 'selected_width': half_width})\n"
        "            verts.append(\n                (\n                    half_width * q,",
    )
    source = replace_once(
        source,
        "    verts, jaw_fairing = fair_jaw_boundary(verts, shell_faces, C, H, Y0, Z0)",
        "    trace_ids = sorted(set([r['row']*cols+cols-1 for r in outline_trace['construction']] + side_path))\n"
        "    outline_trace['vertex_ids'] = trace_ids\n"
        "    outline_trace['before_fairing'] = [verts[i] for i in trace_ids]\n"
        "    verts, jaw_fairing = fair_jaw_boundary(verts, shell_faces, C, H, Y0, Z0)\n"
        "    outline_trace['after_jaw_fairing'] = [verts[i] for i in trace_ids]",
    )
    source = replace_once(
        source,
        '    face = mesh("Face shell editable half", verts, shell_faces, SKIN, mirror=True)',
        "    outline_trace['after_final_fairing'] = [verts[i] for i in trace_ids]\n"
        '    face = mesh("Face shell editable half", verts, shell_faces, SKIN, mirror=True)',
    )
    return replace_once(
        source,
        '        "jaw_boundary_fairing": jaw_fairing,',
        '        "template_outline_trace": outline_trace,\n        "jaw_boundary_fairing": jaw_fairing,',
    )
