"""Shape the original lower eyelash flank without moving its crest or roots."""
import math

def lower_flank_offset(t, fraction, drop, recede):
    assert drop>=0 and recede>=0
    longitudinal=max(0.0,1.0-(t/.95)**2)**2
    across=max(0.0,min(1.0,1.0-fraction))
    return recede*longitudinal*across,-drop*longitudinal*across

def shape_lower_flanks(parts):
    from face import H,C,STYLE
    report={}
    for obj in parts:
        if not obj.name.startswith('Upper lash '):continue
        columns={}
        for v in obj.data.vertices:columns.setdefault(round(abs(v.co.x),8),[]).append(v)
        maximum=0.0;count=0
        for x,column in columns.items():
            if len(column)==1:continue
            levels=sorted(set(round(v.co.z,7) for v in column))
            assert len(levels)==5
            lo,crest=levels[0],levels[2]
            t=(x/H-C['eye_center_x'])/C['eye_half_width']
            for v in column:
                if v.co.z>=crest-1e-7:continue
                fraction=(v.co.z-lo)/(crest-lo)
                dy,dz=lower_flank_offset(t,fraction,STYLE['accent_main_lower_flank_drop_m'],STYLE['accent_main_lower_flank_recede_m'])
                v.co.y+=dy;v.co.z+=dz
                maximum=max(maximum,math.hypot(dy,dz));count+=1
        obj.data.update()
        report[obj.name]={'moved_vertices':count,'maximum_displacement_m':maximum,'crest_and_upper_rail_preserved':True}
    return report
