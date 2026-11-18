"""Experimental separation of source shaping from automatic export triangulation."""


def adapt(sources):
    out = dict(sources)
    pipeline = out['pipeline.py']
    start = pipeline.index('        bm = bmesh.new()\n        bm.from_mesh(members[0].data)',pipeline.index('report["ear_under_quadratic_fairing"]'))
    end = pipeline.index('        # Honest geometry-consistent normals;',start)
    automatic = '''        # Automatic runtime conversion: no neutral-shape edits follow this step.
        report['pre_export_source_polygons'] = {
            'quads':sum(len(p.vertices)==4 for p in data.polygons),
            'triangles':sum(len(p.vertices)==3 for p in data.polygons),
            'ngons':sum(len(p.vertices)>4 for p in data.polygons)}
        bpy.ops.wm.save_as_mainfile(filepath=beaver_output(OUT+'aster_head_editable.blend'),check_existing=False)
        _source_skin_quads=[(p.index,tuple(p.vertices)) for p in data.polygons if len(p.vertices)==4 and data.attributes['NormalRegion'].data[p.index].value in (1,2)]
        bm = bmesh.new()
        bm.from_mesh(data)
        bmesh.ops.triangulate(bm, faces=list(bm.faces), quad_method='BEAUTY', ngon_method='BEAUTY')
        bm.to_mesh(data)
        bm.free()
        data.update()
        from ear_surface import verify_skin_triangles
        report['automatic_export_triangle_verification'] = verify_skin_triangles(data)
'''
    pipeline = pipeline[:start]+pipeline[end:]
    start = pipeline.index('        from ear_topology import improve_lower_root_diagonals, reflow_source_fans')
    end = pipeline.index('        from nose_refine import refine_nose',start)
    pipeline = pipeline[:start]+"        report['triangle_topology_mutations'] = 'disabled; source quad workflow'\n"+pipeline[end:]
    start = pipeline.index('        if STYLE.get("sampling_surface_fairing",0):\n            from ear_surface import fair_final_source_triangles')
    end = pipeline.index('        normals = [Vector((0, 0, 0)) for _ in data.vertices]',start)
    nose_check = ('        from researched_nose_runtime import validate_final_attachments\n'
                  '        report["nose_final_attachment_checks"] = validate_final_attachments(data)\n')
    if nose_check in pipeline:
        assert pipeline.count(nose_check) == 1
        pipeline = pipeline.replace(nose_check, '', 1)
        start = pipeline.index('        if STYLE.get("sampling_surface_fairing",0):\n            from ear_surface import fair_final_source_triangles')
        end = pipeline.index('        normals = [Vector((0, 0, 0)) for _ in data.vertices]',start)
        automatic += nose_check
    pipeline = pipeline[:start]+automatic+pipeline[end:]
    out['pipeline.py'] = pipeline
    # A zero defect count is a valid result, not a reason to reintroduce old folds.
    out['ear_fold.py'] = out['ear_fold.py'].replace("        if iteration==0:assert selected_edges>=2, 'Expected both lower attachment folds'",'        # No precondition that an old attachment defect must still exist.')
    return out
