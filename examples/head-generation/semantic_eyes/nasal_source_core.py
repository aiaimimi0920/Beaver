"""Five source-relative nasal guides with bounded, local curvature supports."""
from collections import Counter, defaultdict
import numpy as np
from scipy.interpolate import PchipInterpolator
from nasal_source_geometry import cross2
from nasal_source_template import TEMPLATE


def capture_nasal_source_anchors(vertices, levels, lateral, columns):
    """Called during this build, after all preceding source-grid adjustments."""
    records = []
    for height in (1.614,1.608,1.6045,1.599,1.590):
        if levels.count(height) != 1 or lateral.count(.87) != 1 or columns != len(lateral):
            raise ValueError('Nasal guide grid station is not unique')
        index = levels.index(height)*columns+lateral.index(.87)
        records.append({'height':height,'lateral':.87,'position':list(map(float,vertices[index]))})
    return records


def local_cycle(quads):
    counts = Counter(tuple(sorted((a,b))) for face in quads
                     for a,b in zip(face,face[1:]+face[:1]))
    adjacency = defaultdict(set)
    for (a,b),count in counts.items():
        if count > 2:
            raise ValueError('Nonmanifold nasal core')
        if count == 1:
            adjacency[a].add(b); adjacency[b].add(a)
    if any(len(v) != 2 for v in adjacency.values()) or 1 not in adjacency[0]:
        raise ValueError('Nasal core boundary contract changed')
    cycle = [0,1]
    while True:
        nxt, = adjacency[cycle[-1]]-{cycle[-2]}
        if nxt == 0:
            break
        if nxt in cycle:
            raise ValueError('Multiple nasal core boundaries')
        cycle.append(nxt)
    if len(cycle) != len(adjacency):
        raise ValueError('Nasal core has a disconnected boundary')
    return cycle


def make_core(surface, vertices, outer_ids, features):
    full,points,lineage = [],[],[]
    t13 = np.linspace(0.,1.,13)
    t17 = np.linspace(1/16,1.,17)
    nose_height = float(features['nose_peak'][2])
    eye_height = features['eye_height']
    if eye_height-nose_height <= .01:
        raise ValueError('Nasal/eye anatomical landmarks overlap')
    def add(xy):
        ids,weights,point = surface.lineage(float(xy[0]),float(xy[1]))
        index = len(points)
        points.append(point.copy()); lineage.append((ids,weights))
        return index
    for row,profile in enumerate(TEMPLATE['guide_profiles']):
        outer = vertices[outer_ids[row]]
        inner_x = float(outer[0])*profile['nasal_to_outer_x_ratio']
        inner_height = nose_height+TEMPLATE['nose_eye_height_ratios'][row]*(eye_height-nose_height)
        rise = PchipInterpolator(profile['progress'],profile['rise'])(t13)
        xy13 = np.column_stack((inner_x+(float(outer[0])-inner_x)*(1-t13),
                                float(outer[2])+(inner_height-float(outer[2]))*rise))
        full.append(xy13)
        for xy in PchipInterpolator(t13,xy13,axis=0)(t17):
            add(xy)
    full = np.asarray(full)
    seed_cells = [np.array([full[r,c],full[r,c+1],full[r+1,c+1],full[r+1,c]])
                  for r in range(4) for c in range(12)]
    main = np.asarray(points).copy()
    midpoint = {}
    for row in range(4):
        for col in (14,15,16):
            fraction = TEMPLATE['auxiliary_fractions'][row*3+col-14]
            if not .25 <= fraction <= .75:
                raise ValueError('Nasal auxiliary sampling fraction escaped its trial range')
            midpoint[row,col] = add((1-fraction)*main[row*17+col,[0,2]]
                                   +fraction*main[(row+1)*17+col,[0,2]])
    quads = []
    for row in range(4):
        for col in range(13):
            quads.append([row*17+col,row*17+col+1,(row+1)*17+col+1,(row+1)*17+col])
        for col in (14,15):
            quads.extend([[row*17+col,row*17+col+1,midpoint[row,col+1],midpoint[row,col]],
                          [midpoint[row,col],midpoint[row,col+1],(row+1)*17+col+1,(row+1)*17+col]])
    middle_points = {}
    for group,row in enumerate((0,2)):
        a = [row*17+13,(row+1)*17+13,(row+2)*17+13]
        b = [row*17+14,midpoint[row,14],(row+1)*17+14,midpoint[row+1,14],(row+2)*17+14]
        def along(chain,u):
            value = u*(len(chain)-1)
            index = min(int(value),len(chain)-2)
            fraction = value-index
            return ((1-fraction)*points[chain[index]][[0,2]]
                    +fraction*points[chain[index+1]][[0,2]])
        interior = []
        for local,u in enumerate((.35,.5,.65)):
            fraction = TEMPLATE['transition_fractions'][group*3+local]
            if not .2 <= fraction <= .7:
                raise ValueError('Nasal transition fraction escaped its trial range')
            interior.append(add((1-fraction)*along(a,u)+fraction*along(b,u)))
        left,middle,right = interior
        a0,a1,a2 = a; b0,b1,b2,b3,b4 = b
        middle_points[row+1] = middle
        quads.extend([[a0,a1,middle,left],[middle,b2,b1,left],[b1,b0,a0,left],
                      [a1,a2,right,middle],[middle,right,b3,b2],[right,a2,b4,b3]])
    points = np.asarray(points)
    for face in quads:
        q = points[face][:,[0,2]]
        if sum(cross2(a,b) for a,b in zip(q,np.roll(q,-1,axis=0))) < 0:
            face.reverse()
    if len(points) != 103 or len(quads) != 80:
        raise ValueError('Nasal core slot cardinality changed')
    expected = [[index for kind,index in face] for face in TEMPLATE['quads'][:80]]
    if quads != expected or any(kind != 'core' for face in TEMPLATE['quads'][:80] for kind,_ in face):
        raise ValueError('Nasal core and transition template disagree')
    cycle = local_cycle(quads)
    if len(cycle) != 44:
        raise ValueError('Nasal short-support boundary changed')
    lines = []
    edges = {tuple(sorted((a,b))) for q in quads for a,b in zip(q,q[1:]+q[:1])}
    for row in range(5):
        line = list(range(row*17,row*17+17))
        if row in middle_points:
            line.insert(14,middle_points[row])
        if any(tuple(sorted((a,b))) not in edges for a,b in zip(line,line[1:])):
            raise ValueError('A primary nasal guide is not an actual edge path')
        lines.append(line)
    return points,quads,cycle,lineage,seed_cells,lines
