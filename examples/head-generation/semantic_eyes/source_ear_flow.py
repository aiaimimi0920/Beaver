"""Opt-in semantic ear source layout; no historical vertex-ID patches."""
import ast
from pathlib import Path


def flow_stage_source(original):
    start = original.index('def retopologize_final_ears(')
    prefix = original[start:original.index('    output = []',start)]
    prefix = prefix.replace('def retopologize_final_ears(', 'def retopologize_flow_ears(',1)
    return prefix+'''    output = []
    skin_reports = []
    for base,faces,vertices in specs:
        print('FEATURE_FLOW_EAR_BEGIN',base,flush=True)
        validate_lower_flow_interface(bm,faces,vertices)
        result = source_flow_ear_layout([tuple(v.co) for v in vertices],EAR_QUAD_EXPECTED_FACES)
        ear_report = apply_final_ear(bm,faces,vertices,result,base,C['ear_return_crease_normals'])
        generated = result.pop('_applied_vertices')
        skin_reports.append(flow_reconnect_skin(bm,vertices,generated,result))
        ear_report['upper_attachment_flow'] = result['upper_attachment_flow']
        ear_report['source_continuity'] = result['source_continuity']
        ear_report['lower_interface_redefinition'] = result['lower_interface_redefinition']
        output.append(ear_report)
        print('FEATURE_FLOW_EAR_READY',base,result['min_diagonal_pair_dot'],flush=True)
    cap_reports = retopologize_source_return_caps(bm,original_vertex_count,set(ordinals[r['name']] for r in reports))
    bm.normal_update(); bm.to_mesh(data); bm.free(); data.update()
    from face import faceuv
    uv2 = data.uv_layers.get('UV2')
    if uv2 is None:
        raise ValueError('Source projection layer UV2 is missing')
    for loop in data.loops:
        point = data.vertices[loop.vertex_index].co
        uv2.data[loop.index].uv = faceuv(point.x,point.z+.021)
    selected = {1,2} | {base+offset for base,_,_ in specs for offset in (0,100,110)}
    regions = data.attributes['NormalRegion']
    surface = [p for p in data.polygons if regions.data[p.index].value in selected]
    if any(len(p.vertices)!=4 for p in surface):
        raise ValueError('Feature-flow source skin/ear contains a non-quad')
    return {'ears':output,'return_caps':cap_reports,'upper_attachment_skin':skin_reports,
            'source_skin_ear_quads':len(surface),
            'scope':'New source layout; complete native pose and visual gates remain required'}
'''


def adapt(sources):
    out = dict(sources)
    root = Path(__file__).parent
    solver = (root/'source_ear_quad_solver.py').read_text()
    lines = solver.splitlines(keepends=True)
    excluded = set()
    for node in ast.parse(solver).body:
        if isinstance(node,ast.FunctionDef) and node.name in {
                'ear_quad_initialize','ear_quad_quality','ear_quad_inset','ear_quad_propose'}:
            excluded.update(range(node.lineno-1,node.end_lineno))
    solver = ''.join(line for i,line in enumerate(lines) if i not in excluded)
    mesh = (root/'ear_final_mesh.py').read_text()
    stage = flow_stage_source(mesh)
    mesh = mesh[:mesh.index('def retopologize_final_ears(')]
    marker = '            vertices.append(bm.verts.new(Vector(point),original_vertices[parent]))'
    if mesh.count(marker) != 1:
        raise ValueError('Ear point allocation contract changed')
    mesh = mesh.replace(marker,marker+'\n            flow_point_fields(bm,vertices[-1],original_vertices,weights[i])',1)
    marker = "    return {'quads':len(added)"
    if mesh.count(marker) != 1:
        raise ValueError('Ear source vertex ownership contract changed')
    mesh = mesh.replace(marker,"    result['_applied_vertices'] = vertices\n"+marker,1)
    chunks = [solver,(root/'ear_final_mapping.py').read_text(),mesh,
              (root/'ear_return_cap_solver.py').read_text(),(root/'ear_final_caps.py').read_text(),
              (root/'ear_flow_geometry.py').read_text(),(root/'ear_flow_mesh.py').read_text(),stage]
    out['ear_topology.py'] += '\n\n'+'\n\n'.join(chunks)
    source = out['ear_return.py']
    marker = "result = {'added_faces':len(new_faces),"
    if source.count(marker) != 1:
        raise ValueError('Posterior source vertex provenance contract changed')
    out['ear_return.py'] = source.replace(marker,"result = {'original_vertex_count':len(original),'added_faces':len(new_faces),",1)
    pipeline = out['pipeline.py']
    legacy = "report['ear_under_topology_refinement']=refine_local_skin(members[0].data)"
    if pipeline.count(legacy) != 1:
        raise ValueError('Legacy side topology stage contract changed')
    pipeline = pipeline.replace(legacy,"report['ear_under_topology_refinement']={'status':'replaced_by_feature_flow_source_layout'}",1)
    marker = '        report["ear_shared_seams"] = verify_ear_seams(data, report["ear_root_fit"])'
    if pipeline.count(marker) != 1:
        raise ValueError('Missing source seam verification stage')
    insert = '        from ear_topology import retopologize_flow_ears\n'
    insert += '        report["feature_flow_ear_source"] = retopologize_flow_ears(data,report["ear_root_fit"],semantic_part_ordinals,report["ear_post_return"]["original_vertex_count"])\n'
    pipeline = pipeline.replace(marker,insert+marker,1)
    marker = '        assert len(ear_root_ids) == expected_ear_roots'
    if pipeline.count(marker) != 1:
        raise ValueError('Missing root normal ownership gate')
    insert = marker+'\n        from ear_topology import source_flow_normal_ids\n'
    insert += '        upper_flow_ids = source_flow_normal_ids(data,report["feature_flow_ear_source"]["upper_attachment_skin"])\n'
    insert += '        ear_root_ids.update(upper_flow_ids)\n        report["upper_attachment_common_normal_vertices"] = sorted(upper_flow_ids)'
    out['pipeline.py'] = pipeline.replace(marker,insert,1)
    for name,source in out.items():
        if name.endswith('.py'):
            ast.parse(source,filename=name)
    return out
