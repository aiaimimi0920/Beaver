"""Keep verified feature charts from being overwritten by legacy local shaping."""


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    old = '    verts, jaw_fairing = fair_jaw_boundary(verts, shell_faces, fair_config, H, Y0, Z0)'
    if shell.count(old) != 1:
        raise ValueError('Initial jaw fairing stage changed')
    shell = shell.replace(old,"    jaw_fairing = {'status':'not_run','reason':'feature-flow source boundary owns geometry'}",1)
    old = ('    verts, final_boundary_fairing = fair_jaw_boundary(\n'
           '        verts, shell_faces, final_boundary_config, H, Y0, Z0\n    )')
    if shell.count(old) != 1:
        raise ValueError('Final jaw fairing stage changed')
    shell = shell.replace(old,"    final_boundary_fairing = {'status':'not_run','reason':'feature-flow source boundary owns geometry'}",1)
    out['shell.py'] = shell
    pipeline = out['pipeline.py']
    calls = {
        'ear_to_jaw_curve':'fit_ear_to_jaw_curve',
        'ear_jaw_rim_fairing':'fair_ear_jaw_rim',
        'ear_under_surface_relaxation':'relax_local_skin',
        'ear_under_front_envelope':'fit_front_width',
        'ear_under_shoulder':'fit_ear_under_shoulder',
        'ear_under_quadratic_fairing':'fair_skin_sections',
        'ear_lower_attachment_fold_fairing':'fair_attachment_folds',
    }
    import ast
    lines = pipeline.splitlines(keepends=True)
    edits = []
    for node in ast.walk(ast.parse(pipeline)):
        if not isinstance(node,ast.Assign) or len(node.targets) != 1:
            continue
        target = node.targets[0]
        if not (isinstance(target,ast.Subscript) and isinstance(target.value,ast.Name)
                and target.value.id == 'report' and isinstance(target.slice,ast.Constant)):
            continue
        name = target.slice.value
        if name not in calls:
            continue
        if not (isinstance(node.value,ast.Call) and isinstance(node.value.func,ast.Name)
                and node.value.func.id == calls[name]):
            raise ValueError('Legacy source stage ownership changed: '+name)
        indent = lines[node.lineno-1][:node.col_offset]
        edits.append((node.lineno-1,node.end_lineno,indent+f"report[{name!r}] = {{'status':'not_run','reason':'replaced by feature-flow source geometry'}}\n"))
    if len(edits) != len(calls):
        raise ValueError('Missing legacy shaping stage isolation')
    for start,end,text in sorted(edits,reverse=True):
        lines[start:end] = [text]
    pipeline = ''.join(lines)
    automatic = "        bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method='BEAUTY', ngon_method='BEAUTY')"
    if pipeline.count(automatic) != 1:
        raise ValueError('Automatic export conversion stage changed')
    pipeline = pipeline.replace(automatic,
        "        from ear_surface import triangulate_valid_skin\n"
        "        report['paired_source_automatic_triangulation'] = triangulate_valid_skin(data,bm)\n"+automatic,1)
    before_return = '        report["ear_post_return"] = add_final_return(members[0].data)'
    if pipeline.count(before_return) != 1:
        raise ValueError('Source return construction stage changed')
    snapshot = "        _return_data = members[0].data\n        report['feature_flow_pre_return_geometry'] = {\n            'scope':'Diagnostic geometry before posterior return; not task completion evidence',\n            'vertices':[tuple(v.co) for v in _return_data.vertices],\n            'faces':[tuple(p.vertices) for p in _return_data.polygons],\n            'regions':[int(v.value) for v in _return_data.attributes['NormalRegion'].data],\n            'mouth_delta':[tuple(v.vector) for v in _return_data.attributes['MouthOpenDelta'].data]}\n        Path(beaver_output(OUT+'generation_report.json')).write_text(json.dumps(report,indent=2))\n"
    # Construct the new strip after all retained ear silhouette shaping, so
    # its verified boundary-normal rows are not deformed by legacy passes.
    anchor = '        from ear_topology import retopologize_flow_ears'
    if pipeline.count(anchor) != 1:
        raise ValueError('Feature-flow ear assembly boundary changed')
    pipeline = pipeline.replace(before_return,'',1)
    pipeline = pipeline.replace(anchor,snapshot+before_return+'\n'+anchor,1)
    blink_call = '        report["eyelid_closure"] = build_blink_shapes(members[0], semantic_part_ordinals, C,'
    if pipeline.count(blink_call) != 1:
        raise ValueError('Feature-flow blink calibration boundary changed')
    blink_setup = "        _flow_blink_calibration = dict(C)\n"
    blink_setup += "        _flow_blink_calibration['lateral_blink_follow'] = min(C.get('lateral_blink_follow',.85),.67)\n"
    blink_setup += "        report['feature_flow_lateral_blink_follow'] = {'requested':C.get('lateral_blink_follow',.85),'effective':_flow_blink_calibration['lateral_blink_follow'],'scope':'Lateral accents only; neutral geometry and core upper/lower closure unchanged'}\n"
    pipeline = pipeline.replace(blink_call,blink_setup+blink_call.replace('ordinals, C,','ordinals, _flow_blink_calibration,'),1)
    out['pipeline.py'] = pipeline
    return out
