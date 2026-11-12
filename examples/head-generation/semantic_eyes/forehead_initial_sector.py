"""Immutable author-sector witnesses for pinned nonterminal guide transits."""
import numpy as np


def forehead_initial_pinned_sector_obstructions(source_chart,paths,coordinate_pins,pivot,existing_count,tolerance=1e-12):
    chart=np.asarray(source_chart,float);xy=chart[:,[0,2]]if chart.shape[1]==3 else chart;pivot=np.asarray(pivot,float);pins=set(coordinate_pins);rows=[]
    if pivot.shape!=(2,)or len(xy)<existing_count or not np.isfinite(xy).all():raise ValueError('Invalid immutable author sector')
    terminals={path[-1]for path in paths.values()}
    for role,path in paths.items():
        terminal=path[-1]
        if terminal<0 or terminal>=existing_count:raise ValueError('Initial terminal must have original author ownership')
        if terminal not in pins:raise ValueError('Immutable terminal-sector proof requires a coordinate-pinned terminal')
        endpoint=xy[terminal]-pivot
        if endpoint[0]<=tolerance or endpoint[1]<-tolerance:continue
        terminal_angle=float(np.arctan2(endpoint[1],endpoint[0]))
        for vertex in path[1:-1]:
            if vertex>=existing_count or vertex not in pins or vertex in terminals:continue
            radial=xy[vertex]-pivot
            if radial[0]<-tolerance:continue
            angle=float(np.arctan2(radial[1],radial[0]))
            if angle<terminal_angle-tolerance:
                rows.append({'role':role,'vertex':int(vertex),'terminal':int(terminal),'pin_angle_radians':angle,'terminal_angle_radians':terminal_angle,'angle_deficit_degrees':float(np.degrees(terminal_angle-angle)),'proof':'A fixed nonterminal lies beyond its fixed terminal in the strictly decreasing radial sector; moving intermediate births cannot reverse fixed endpoint order'})
    return {'blocked_transits':sorted({row['vertex']for row in rows}),'witnesses':rows,'scope':'Conservative aggregate transit exclusions only; coordinate pins, original vertices, fields and edges are retained unchanged'}
