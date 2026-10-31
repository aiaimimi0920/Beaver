"""Validate quad ears before and after automatic export, including the skin join."""
import inspect


def validate_ear_source_quads(data, ear_regions):
    from ear_boundary import source_quad_normal_dots
    points = [tuple(v.co) for v in data.vertices]
    quads = []
    minimum = 1.0
    regions = data.attributes['NormalRegion']
    for polygon in data.polygons:
        if regions.data[polygon.index].value not in ear_regions:
            continue
        if len(polygon.vertices) != 4:
            raise ValueError('Ear source contains a non-quad face')
        ids = tuple(polygon.vertices)
        quality = min(source_quad_normal_dots(points, ids))
        if quality < .49:
            raise ValueError('Final source ear quad folds: ' + str((polygon.index, quality)))
        minimum = min(minimum, quality)
        quads.append((polygon.index, ids))
    if not quads:
        raise ValueError('No authored ear quads to validate')
    return quads, {'checked_quads':len(quads),'minimum_both_diagonal_dot':minimum}


def reject_ear_skin_crossings(data, ear_regions):
    regions = data.attributes['NormalRegion']
    selected = [p for p in data.polygons if regions.data[p.index].value in ear_regions | {1,2}]
    pairs = strict_skin_crossings([tuple(v.co) for v in data.vertices],
                                 [tuple(p.vertices) for p in selected])
    ear_pairs = [(selected[a].index, selected[b].index) for a,b in pairs
                 if (regions.data[selected[a].index].value in ear_regions
                     or regions.data[selected[b].index].value in ear_regions)]
    if ear_pairs:
        raise ValueError('Ear self/skin strict crossings: ' + str(ear_pairs[:16]))
    return {'ear_related_strict_crossings':0,
            'scope':'neutral ear self and ear/skin nonadjacent strict crossings; excludes coplanar/shared-vertex contacts and pose proof'}


def adapt(sources):
    out = dict(sources)
    pipeline = out['pipeline.py']
    anchor = 'main_lash_front_regions = set()'
    assert pipeline.count(anchor) == 1
    pipeline = pipeline.replace(anchor, anchor + '\n_source_ear_regions = set()', 1)
    anchor = '        regions.data[poly.index].value = region'
    assert pipeline.count(anchor) == 1
    pipeline = pipeline.replace(anchor, '        if part.get("ear_detail",False): _source_ear_regions.add(region)\n' + anchor, 1)
    anchor = '        bm = bmesh.new()\n        bm.from_mesh(data)'
    assert pipeline.count(anchor) == 1
    pipeline = pipeline.replace(anchor, "        from ear_surface import validate_ear_source_quads\n        _source_ear_quads,report['source_ear_quad_checks'] = validate_ear_source_quads(data,_source_ear_regions)\n" + anchor, 1)
    anchor = "        report['automatic_quad_cells'] = verify_export_quad_cells(data,_source_skin_quads)"
    assert pipeline.count(anchor) == 1
    pipeline = pipeline.replace(anchor, anchor + "\n        report['automatic_ear_quad_cells'] = verify_export_quad_cells(data,_source_ear_quads)\n        from ear_surface import reject_ear_skin_crossings\n        report['ear_skin_crossings'] = reject_ear_skin_crossings(data,_source_ear_regions)", 1)
    out['pipeline.py'] = pipeline
    out['ear_surface.py'] += '\n\n' + inspect.getsource(validate_ear_source_quads)
    out['ear_surface.py'] += '\n\n' + inspect.getsource(reject_ear_skin_crossings)
    return out
