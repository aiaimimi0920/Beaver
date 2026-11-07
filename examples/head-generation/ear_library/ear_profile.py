"""Inject researched lateral contours while preserving the evaluated root chain."""

import inspect
import ast


def researched_ear_envelope(t, u, envelope, config):
    peak = config["researched_ear_point_peak"]
    point = t / peak if t <= peak else (1 - t) / (1 - peak)
    a = max(0., min(1., (u - .22) / .78))
    weight = a * a * (3 - 2 * a) * config["researched_ear_point_strength"]
    return envelope * (1 - weight) + point * weight


def researched_ear_tip_lift(t, u, config):
    from math import sin, pi
    a = max(0., min(1., (u - .22) / .78))
    return config["researched_ear_tip_height"] * sin(pi * t) * a * a * (3 - 2 * a)


def replace_once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Researched ear source contract changed: " + old[:70])
    return source.replace(old, new, 1)


def bounded_fairing(original, proposed, selected, valid):
    """Backtrack actual displacement, never relax the orientation/area gate."""
    for attempt in range(16):
        scale = 0.5 ** attempt
        candidate = list(original)
        for i in selected:
            candidate[i] = proposed[i] if attempt == 0 else tuple(
                a + scale * (b - a) for a, b in zip(original[i], proposed[i])
            )
        if valid(candidate):
            return candidate, scale
    valid(original)  # Restore the native mesh before reporting failure.
    raise AssertionError("No orientation-preserving lower ear fairing step")


def adapt_boundary(source, design):
    if design is None:
        return source
    old = "    for i in selected:data.vertices[i].co=result[i]\n    data.update()\n"
    new = '''    def valid_fairing(candidate):
        for i in selected:data.vertices[i].co=candidate[i]
        data.update()
        return all(p.area>1e-12 and p.normal.dot(n)>0
                   for p,n in zip(data.polygons,old_normals)
                   if any(i in selected for i in p.vertices))
    result, fairing_scale = bounded_fairing(original,result,selected,valid_fairing)
'''
    source = replace_once(source, old, new)
    source = replace_once(
        source,
        "return {'selected_vertices':len(selected),'maximum_displacement_m':maximum,",
        "return {'fairing_scale':fairing_scale,'selected_vertices':len(selected),'maximum_displacement_m':maximum,",
    )
    source = replace_once(
        source, "def support_offset(v, q, amplitude):",
        "def support_offset(v, q, amplitude, center=.26):",
    )
    source = replace_once(source, "abs((v-.26)/.08)", "abs((v-center)/.08)")
    source = replace_once(
        source, "support_offset(v,q,C['ear_root_posterior_support_m'])",
        "support_offset(v,q,C['ear_root_posterior_support_m'],C['ear_root_min_height'])",
    )
    source = replace_once(
        source, "'compact_height_range':[.18,.34]",
        "'compact_height_range':[C['ear_root_min_height']-.08,C['ear_root_min_height']+.08]",
    )
    source += "\n\n" + inspect.getsource(bounded_fairing)
    compile(source, "<researched ear boundary>", "exec")
    return source


def researched_ear_root_radius(config):
    if config["researched_ear_kind"] not in ("long-leaf", "side-droop"):
        return .012
    return min(.020, .012 * max(1., config["ear_outward_span"] / .118))


def adapt_topology(source, design):
    """Prefer the legacy locality; large ears may use a bounded fallback search."""
    if design is None:
        return source
    nodes = [node for node in ast.parse(source).body
             if isinstance(node, ast.FunctionDef) and node.name == 'improve_lower_root_diagonals']
    if len(nodes) != 1:
        raise ValueError('Expected one root diagonal quality function')
    lines = source.splitlines(keepends=True)
    first,last = nodes[0].lineno-1,nodes[0].end_lineno
    prefix,suffix = ''.join(lines[:first]),''.join(lines[last:])
    source = ''.join(lines[first:last])
    replacements = (
        ("    import bmesh\n", "    import bmesh\n    from face import C\n    search_radius = researched_ear_root_radius(C)\n"),
        ("root=bm.verts[i];candidates=[]", "root=bm.verts[i];candidates=[];expanded=[]\n        fan_before=min(triangle_quality(*[v.co for v in f.verts]) for f in root.link_faces)"),
        ("            if max((v.co-root.co).length for v in (a,b,c,d))>.012:continue",
         "            candidate_radius=max((v.co-root.co).length for v in (a,b,c,d))\n            if candidate_radius>search_radius:continue"),
        ("            candidates.append((q1-q0,edge,q0,q1,normal.copy()))",
         "            target=candidates if candidate_radius<=.012 else expanded\n            target.append((q1-q0,edge,q0,q1,normal.copy(),candidate_radius))"),
        ("        assert candidates,", "        if not candidates:candidates=expanded\n        assert candidates,"),
        ("_,edge,q0,q1,normal=max(candidates,key=lambda x:x[0])",
         "_,edge,q0,q1,normal,selected_radius=max(candidates,key=lambda x:x[0])"),
        ("        assert actual>q0+.05",
         "        assert actual>q0+.05\n        fan_after=min(triangle_quality(*[v.co for v in f.verts]) for f in root.link_faces)\n        assert fan_after>=fan_before-1e-9,'Root fan quality regressed'\n        assert selected_radius<=search_radius<=.020"),
        ("results.append({'before_quality':q0,",
         "results.append({'search_radius_limit_m':search_radius,'selected_radius_m':selected_radius,'expanded_search_used':selected_radius>.012,'root_fan_minimum_before':fan_before,'root_fan_minimum_after':fan_after,'before_quality':q0,"),
    )
    for old, new in replacements:
        source = replace_once(source, old, new)
    source = replace_once(source, "root=bm.verts[i];candidates=[];expanded=[]",
                          "root=bm.verts[i];candidates=[];expanded=[];diagnostics=[]")
    source = replace_once(source, "            if q1<=q0+.05:continue",
                          "            diagnostics.append({'before':q0,'after':q1,'radius':candidate_radius})\n            if q1<=q0+.05:continue")
    source = replace_once(source, "assert candidates,'No safe lower-root quality improvement found'",
                          "assert candidates,('No safe lower-root quality improvement found',diagnostics)")
    source = prefix + source + suffix + "\n\n" + inspect.getsource(researched_ear_root_radius)
    compile(source, "<researched ear topology locality>", "exec")
    return source


def adapt(source, design):
    if design is None:
        return source
    source = replace_once(
        source,
        '    outward = envelope * config["ear_outward_span"] * curve(u, 1, config["ear_outward_profile"])',
        '    outward = researched_ear_envelope(t, u, envelope, config) * config["ear_outward_span"] * curve(u, 1, config["ear_outward_profile"])',
    )
    source = replace_once(
        source, '- config["ear_posterior_span"] * envelope',
        '- config["ear_posterior_span"] * researched_ear_envelope(t, 1., envelope, config)',
    )
    old = "                    lift = (ear_arc_height(t, low, high) - root.z) * sqrt(max(0, sin(pi * u / 2)))"
    source = replace_once(source, old, old + "\n                    lift += H * researched_ear_tip_lift(t, u, C)")
    old = '            obj["ear_detail"] = True'
    source = replace_once(source, old, old + '\n            obj["ear_design_kind"] = C["researched_ear_kind"]\n            obj["ear_design_sha256"] = C["researched_ear_design_sha256"]')
    source += "\n\n" + inspect.getsource(researched_ear_envelope)
    source += "\n\n" + inspect.getsource(researched_ear_tip_lift)
    compile(source, "<researched lateral ear>", "exec")
    return source

