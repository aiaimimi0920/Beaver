"""C2 nasal shaping from semantic compactness, preserving evaluated skin continuity."""
WIDTH_CONTRACTION = .12
TIP_RETRACTION_M = .000944

def silhouette_delta(x,height):
    """Semantic apex/underside controls measured from side-silhouette slopes."""
    apex_radius = .024 if height < .298 else .034
    apex = .0022*bump((height-.298)/apex_radius)
    lower_radius = .021 if height < .242 else .028
    lower = .0011*bump((height-.242)/lower_radius)
    shoulder = -.0005*bump((height-.270)/.014)
    return (apex+lower+shoulder)*bump(x/.017)


def bump(t):
    return max(0.0,1.0-t*t)**3 if abs(t)<1 else 0.0

def nasal_delta(x,height):
    weight = bump(x/.022)*bump((height-.286)/.075)
    return -x*WIDTH_CONTRACTION*weight, TIP_RETRACTION_M*weight - silhouette_delta(x,height)

def refine_nose(data):
    from face import H,Y0
    old = [v.co.copy() for v in data.vertices]
    normals = [p.normal.copy() for p in data.polygons]
    regions = data.attributes.get('NormalRegion')
    assert regions is not None
    front_ids = {i for p in data.polygons if regions.data[p.index].value == 1 for i in p.vertices}
    chosen = []
    maximum = 0.0
    for v in data.vertices:
        if v.index not in front_ids: continue
        dx,dy = nasal_delta(v.co.x,(v.co.z+.021-Y0)/H)
        if abs(dx)+abs(dy)<1e-12: continue
        v.co.x += dx;v.co.y += dy
        chosen.append(v.index)
        maximum = max(maximum,(dx*dx+dy*dy)**.5)
    assert maximum<.003 and chosen
    data.update()
    selected = set(chosen)
    assert all((v.co-p).length<1e-9 for v,p in zip(data.vertices,old) if v.index not in selected)
    for poly,n in zip(data.polygons,normals):
        if any(i in selected for i in poly.vertices):
            assert poly.area>1e-12 and poly.normal.dot(n)>.8, 'Nose face inverted'
    return {'apex_support_h':[.274,.332],'apex_center_h':.298,'apex_projection_m':.0022,'lower_curve_center_h':.242,'width_contraction':WIDTH_CONTRACTION,'tip_retraction_m':TIP_RETRACTION_M,'selected_vertices':len(chosen),'max_delta_m':maximum,'outside_nose_unchanged':True,'continuous_falloff':'C2 polynomial bump','parameter_stage':'after ear topology; before normals'}
