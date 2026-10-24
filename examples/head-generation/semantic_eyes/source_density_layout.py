"""Shared first-pass facial density layout, before subdivision or export.

Feature endpoints and the outer silhouette remain anchors. The layout does not
collapse a finished mesh and has no character name or exported vertex IDs.
"""
import inspect
import math


def density_rows(levels):
    """Retain all calibrated row coordinates; topology sets local resolution.

Mouth/eye patch boundaries remain unchanged. Detail stays in their local rings
instead of propagating another thin horizontal strip across both cheeks.
"""
    expected = (
        1.544, 1.551, 1.560, 1.570, 1.580, 1.590, 1.599, 1.6045,
        1.608, 1.614, 1.623, 1.633, 1.644, 1.655, 1.664, 1.673,
        1.685, 1.701, 1.720, 1.740, 1.760, 1.780,
    )
    if tuple(levels) != expected:
        raise ValueError("Unknown complete semantic row contract")
    return list(levels)


def chin_column_indices(columns):
    """Keep the two outer control intervals to preserve silhouette tangents."""
    if columns < 5 or columns % 2 != 1:
        raise ValueError("Unsupported chin columns")
    return list(range(0, columns-2, 2)) + [columns-2, columns-1]


def chin_strip_faces(row, columns):
    """Join coarse interior chin rows to fine feature rows without T junctions."""
    if row not in (0, 1, 2):
        raise ValueError("Unsupported chin strip")
    indices = chin_column_indices(columns)
    result = []
    for left, right in zip(indices, indices[1:]):
        a, b = row*columns+left, row*columns+right
        c, e = (row+1)*columns+left, (row+1)*columns+right
        if row < 2 or right-left == 1:
            result.append((a, b, e, c))
        else:
            d = c+1
            result.extend(((a, b, e, d), (a, d, c)))
    return result


def subnasal_band_faces(row, columns, nose_column):
    """Coarsen cheek-only vertical cells, keeping nose and ear-root row samples.

At each coarse/fine interface the midpoint remains connected by a quad and
triangle pair. No row is removed from either outer boundary or nasal support.
"""
    first, last = nose_column, columns-4
    if not 1 <= first < last < columns-2:
        raise ValueError("Insufficient cheek transition width")
    result = []
    for i in range(columns-1):
        if i < first or i > last:
            for j in (row,row+1):
                a = j*columns+i
                result.append((a,a+1,a+columns+1,a+columns))
            continue
        a, b = row*columns+i, row*columns+i+1
        d, c = (row+2)*columns+i, (row+2)*columns+i+1
        if i == first:
            middle = (row+1)*columns+i
            result.extend(((a,b,c,middle),(middle,c,d)))
        elif i == last:
            middle = (row+1)*columns+i+1
            result.extend(((a,b,middle,d),(middle,c,d)))
        else:
            result.append((a,b,c,d))
    return result


def once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Density layout source contract changed: " + old[:70])
    return source.replace(old,new,1)


def tongue_budget(style):
    """Keep cardinal tongue rings; allocate hidden detail to denser skin sampling."""
    if not style.get("sampling_detail_budget", 0):
        return 16, 9
    columns = int(style.get("sampling_shell_columns", 13))
    if columns not in (9, 13):
        raise ValueError("Unsupported skin/interior budget allocation")
    return 12, 5 if columns == 13 else 7


def oral_ring_fractions(kind, style):
    """Spend one extra local ring only in the optional denser-skin profile."""
    if kind == "eye":
        return [0, 0.30, 0.65]
    if kind != "mouth":
        raise ValueError("Unknown source patch kind")
    if style.get("sampling_detail_budget", 0) and style.get("sampling_shell_columns", 13) == 13:
        return [0, 0.50, 0.75]
    return [0, 0.30]


def adapt(sources):
    result = dict(sources)
    shell = result['shell.py']
    helpers = '\n\n'.join(inspect.getsource(f) for f in
                            (density_rows, chin_column_indices, chin_strip_faces, subnasal_band_faces, oral_ring_fractions))
    # Face owns shared geometric sampling primitives; shell owns assembly.
    result['face.py'] = result['face.py'] + '\n\n' + helpers + '\n'
    shell = once(shell,'    assert min(b - a for a, b in zip(levels, levels[1:])) >= 0.00349',
                 '    levels = density_rows(levels)\n    assert min(b - a for a, b in zip(levels, levels[1:])) >= 0.00349')
    shell = once(shell, '    for j in range(rows - 1):\n        for i in range(cols - 1):',
                 '    for j in range(rows - 1):\n'
                 '        if j < 3:\n'
                 '            shell_faces.extend(chin_strip_faces(j, cols))\n'
                 '            continue\n'
                 '        if j in (6, 7):\n'
                 '            if j == 6: shell_faces.extend(subnasal_band_faces(j, cols, lateral.index(0.18)))\n'
                 '            continue\n'
                 '        for i in range(cols - 1):')
    shell = once(shell, '    base = [verts[i] for i in range(cols)]\n    previous = list(range(cols))',
                 '    chin_columns = chin_column_indices(cols)\n'
                 '    base = [verts[i] for i in chin_columns]\n'
                 '    previous = chin_columns')
    shell = once(shell, '            q = nominal[i]', '            q = nominal[chin_columns[i]]')
    shell = once(shell, '        for i in range(cols - 1):\n            shell_faces.append((previous[i + 1]',
                 '        for i in range(len(base) - 1):\n            shell_faces.append((previous[i + 1]')
    shell = once(shell,'for blend in ([0, 0.30, 0.65] if kind == "eye" else [0, 0.30, 0.65]):',
                 'for blend in oral_ring_fractions(kind, STYLE):')
    shell = once(shell,'        "semantic_rows": levels,',
                 '        "density_layout": {"version": 6, "stage": "source sampling", "mouth_rings": len(oral_ring_fractions("mouth", STYLE)), "rows": len(levels), "chin_columns": len(chin_columns)},\n'
                 '        "semantic_rows": levels,')
    from source_outline_sampling import refined_outline
    result['ear_topology.py'] = result.get('ear_topology.py','')+'\n\n'+inspect.getsource(refined_outline)+'\n'
    shell = 'from ear_topology import refined_outline\n'+shell
    shell = once(shell, '    side_count = len(shell_faces) - front_count - underside_count',
                 '    side_count = len(shell_faces) - front_count - underside_count\n'
                 '    outline_report = None\n'
                 '    if STYLE.get("sampling_outline_refinement",0):\n'
                 '        chain = [j*cols+cols-1 for j in range(len(side_path)-1, rows)] + [(rows-1)*cols+i for i in range(cols-2,-1,-1)]\n'
                 '        ear_anchors = [i for i in chain if C["ear_root_min_height"] <= (verts[i][2]-Y0)/H <= C["ear_root_max_height"]]\n'
                 '        protected = set(ear_anchors[:2]+ear_anchors[-1:])\n'
                 '        outline_report = {"stage": "deferred source boundary strip"}')
    shell = once(shell, '        "jaw_boundary_fairing": jaw_fairing,',
                 '        "source_outline_sampling": outline_report,\n        "face_authoring_ngons": sum(len(f)>4 for f in shell_faces),\n        "jaw_boundary_fairing": jaw_fairing,')
    shell = once(shell, '    face["chin_side_face_count"] = side_count',
                 '    if STYLE.get("sampling_outline_refinement",0): face["source_outline_chain"] = chain\n    face["chin_side_face_count"] = side_count')
    result['shell.py'] = shell
    if 'mouth.py' in result:
        result['mouth.py'] = once(result['mouth.py'], '    cols = 16\n    rows = 9',
                                 '    cols, rows = tongue_budget(STYLE)')
        result['mouth.py'] += '\n\n'+inspect.getsource(tongue_budget)+'\n'
    if 'pipeline.py' not in result:
        return result
    pipeline = result['pipeline.py']
    from source_cage_fairing import fair_source_cage, sample_source_cage
    from source_polygon_validity import polygon_diagonals, triangulate_valid_skin, verify_skin_triangles
    result['ear_surface.py'] += '\n\n'+inspect.getsource(polygon_diagonals)+'\n\n'+inspect.getsource(triangulate_valid_skin)+'\n\n'+inspect.getsource(verify_skin_triangles)+'\n'
    result['ear_topology.py'] += '\n\n'+inspect.getsource(fair_source_cage)+'\n'
    result['ear_surface.py'] += '\n\n'+inspect.getsource(sample_source_cage)+'\n'
    pipeline = once(pipeline, 'from shell import build_face', 'from shell import build_face\nfrom ear_topology import fair_source_cage\nfrom ear_surface import sample_source_cage')
    pipeline = once(pipeline, '        bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method="BEAUTY", ngon_method="BEAUTY")',
                    '        if STYLE.get("sampling_detail_budget",0) and STYLE.get("sampling_shell_columns",13)==13:\n            from ear_surface import triangulate_valid_skin\n            report["paired_skin_triangulation"]=triangulate_valid_skin(members[0].data,bm)\n        bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method="BEAUTY", ngon_method="BEAUTY")')
    pipeline = once(pipeline, '        # Honest geometry-consistent normals; no unmatched analytic shading field.',
                    '        if STYLE.get("sampling_detail_budget",0) and STYLE.get("sampling_shell_columns",13)==13:\n            from ear_surface import verify_skin_triangles\n            report["paired_skin_triangle_verification"]=verify_skin_triangles(members[0].data)\n        # Honest geometry-consistent normals; no unmatched analytic shading field.')
    pipeline = once(pipeline, 'sub = shell.modifiers.new("Semantic quad surface refinement", "SUBSURF")',
                    'if STYLE.get("sampling_surface_fairing",0):\n    report["source_cage_fairing"]=(sample_source_cage(shell, fair_source_cage) if STYLE.get("sampling_detail_budget",0) and STYLE.get("sampling_shell_columns",13)==13 else fair_source_cage(shell))\nsub = shell.modifiers.new("Semantic quad surface refinement", "SUBSURF")')
    pipeline = once(pipeline, 'sub.levels = STYLE["surface_subdivision_levels"]',
                    'sub.levels = int(STYLE.get("sampling_surface_subdivision", STYLE["surface_subdivision_levels"]))')
    pipeline = once(pipeline, '"evaluated_effect": measure_crease_effect(shell, new_crease_edges),',
                    '"evaluated_effect": measure_crease_effect(shell, new_crease_edges) if sub.levels else {"status": "not_applicable", "reason": "unsubdivided source budget trial"},')
    pipeline = once(pipeline, '            bpy.ops.object.modifier_apply(modifier=m.name)',
                    '            if m.type == "SUBSURF" and m.levels == 0:\n                o.modifiers.remove(m)\n            else:\n                bpy.ops.object.modifier_apply(modifier=m.name)')
    for field, call in (("ear_source_fan_reflow", "reflow_source_fans"), ("ear_root_diagonal_quality", "improve_lower_root_diagonals")):
        old = f'        report["{field}"]={call}(data,report["ear_root_fit"])'
        new = ('        try:\n    '+old+'\n'
               '        except AssertionError as error:\n'
               '            if not STYLE.get("sampling_budget_trial", 0): raise\n'
               f'            report["{field}"]={{"status": "failed", "error": str(error)}}\n'
               '            report["budget_trial_quality_pending"] = True')
        pipeline = once(pipeline, old, new)
    runtime = result.get('semantic_eye_runtime.py', '')
    if 'def build_blink_shapes' in runtime:
        runtime = once(runtime, 'def build_blink_shapes(obj, ordinals, calibration, ink_floor_closure=False):',
                       'def build_blink_shapes(obj, ordinals, calibration, ink_floor_closure=False, budget_trial=False):')
        runtime = once(runtime, '    report["continuous_orientation"] = verify_pose_orientation(obj)',
                       '    try:\n        report["continuous_orientation"] = verify_pose_orientation(obj)\n'
                       '    except AssertionError as error:\n        if not budget_trial: raise\n'
                       '        report["continuous_orientation"] = {"status": "failed", "error": str(error), "acceptance": False}')
        pipeline = once(pipeline, 'build_blink_shapes(members[0], semantic_part_ordinals, C, ink_floor_closure=',
                        'build_blink_shapes(members[0], semantic_part_ordinals, C, budget_trial=bool(STYLE.get("sampling_budget_trial", 0)), ink_floor_closure=')
        lines = [line for line in pipeline.splitlines(keepends=True) if 'report["eyelid_closure"] = build_blink_shapes(' in line]
        if len(lines) != 1: raise ValueError('Unexpected blink result boundary')
        pipeline = once(pipeline, lines[0], lines[0]+'        if report["eyelid_closure"]["continuous_orientation"].get("status") == "failed":\n            report["budget_trial_quality_pending"] = True\n')
        result['semantic_eye_runtime.py'] = runtime
    from source_outline_stage import refine_outline_after_fairing, validate_final_outline_triangles
    result['crease_rules.py'] = result.get('crease_rules.py','')+'\n\n'+inspect.getsource(refine_outline_after_fairing)+'\n\n'+inspect.getsource(validate_final_outline_triangles)+'\n'
    pipeline = once(pipeline, 'from eye_socket import fit_pocket_clearance',
                    'if STYLE.get("sampling_outline_refinement",0):\n'
                    '    from crease_rules import refine_outline_after_fairing\n'
                    '    report["source_outline_sampling"] = refine_outline_after_fairing(shell, front_count, C, H, Y0)\n'
                    'from eye_socket import fit_pocket_clearance')
    pipeline = once(pipeline, '            region = 1 if poly.index < front_count else 2',
                    '            source_regions = part.data.attributes.get("OutlineSourceRegion")\n'
                    '            region = source_regions.data[poly.index].value if source_regions else 1 if poly.index < front_count else 2')
    pipeline = once(pipeline, '        report["flat_geometric_triangles"] = sum(not poly.use_smooth for poly in data.polygons)',
                    '        report["flat_geometric_triangles"] = sum(not poly.use_smooth for poly in data.polygons)\n'
                    '        if STYLE.get("sampling_outline_refinement",0):\n'
                    '            from crease_rules import validate_final_outline_triangles\n'
                    '            report["final_outline_geometry"] = validate_final_outline_triangles(data, report["source_outline_sampling"])')
    from source_triangle_fairing import fair_final_source_triangles
    result['ear_surface.py'] = result.get('ear_surface.py','')+'\n\n'+inspect.getsource(fair_final_source_triangles)+'\n'
    pipeline = once(pipeline, '        normals = [Vector((0, 0, 0)) for _ in data.vertices]',
                    '        if STYLE.get("sampling_surface_fairing",0):\n'
                    '            from ear_surface import fair_final_source_triangles\n'
                    '            report["final_source_triangle_fairing"] = fair_final_source_triangles(data,C,H,Y0,report["head_assembly_y_offset_m"],project_uv=faceuv)\n'
                    '            report["ear_shared_seams_after_final_fairing"] = verify_ear_seams(data,report["ear_root_fit"])\n'
                    '        normals = [Vector((0, 0, 0)) for _ in data.vertices]')
    center_guard = 'if abs(data.vertices[index].co.x) < 1e-7'
    assert pipeline.count(center_guard) == 2
    pipeline = pipeline.replace(center_guard, center_guard + ' and not (region in (1, 2) and STYLE.get("sampling_detail_budget", 0) and STYLE.get("sampling_shell_columns", 13) == 13)')
    pipeline = once(pipeline, '                normals[index] += poly.normal * poly.area',
                    '                if region in (1,2) or not (STYLE.get("sampling_detail_budget",0) and STYLE.get("sampling_shell_columns",13)==13):\n'
                    '                    normals[index] += poly.normal * poly.area')
    result['pipeline.py'] = pipeline
    return result
