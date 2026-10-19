"""Opt-in bounded whole-field upper-ear junction adaptation; unchanged safety gates."""


def adapt_upper_join(source, enabled=False):
    if not enabled:
        return source
    start = source.index("    changed=set()\n", source.index("def fit_upper_join"))
    end = source.index("    for report,mapping in zip(reports,maps):", start)
    source = (
        source[:start]
        + """    displacements=[upper_join_offset(v.co,anchors[0 if v.co.x<0 else 1]) for v in data.vertices]
    changed={i for i,d in enumerate(displacements) if abs(d)>1e-12}
    attempts=[];accepted_scale=None
    for scale in (1.0,.5,.25,.125):
        for i,v in enumerate(data.vertices):
            v.co=old[i].copy()
            if i in changed:v.co.z+=displacements[i]*scale
        data.update()
        bad=[p.index for p,n in zip(data.polygons,normals) if changed.intersection(p.vertices) and (p.area<1e-12 or p.normal.dot(n)<=0)]
        attempts.append({'scale':scale,'invalid_faces':bad})
        if not bad:
            accepted_scale=scale
            break
    if accepted_scale is None:
        for i,v in enumerate(data.vertices):v.co=old[i].copy()
        data.update()
        raise AssertionError(('Upper-ear bounded junction adaptation failed',attempts))
"""
        + source[end:]
    )
    marker = "'shared_root_moved_together':True,'topology_unchanged':True}"
    if source.count(marker) != 1:
        raise ValueError("Upper junction report contract changed")
    source = source.replace(
        marker,
        "'shared_root_moved_together':True,'topology_unchanged':True,'requested_scale':1.0,'accepted_scale':accepted_scale,'attempts':attempts,'minimum_scale':.125}",
    )
    compile(source, "<bounded ear junction>", "exec")
    return source
