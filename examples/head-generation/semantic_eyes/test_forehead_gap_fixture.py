"""Synthetic author-owned annular rails; no character fixture or historical IDs."""
import numpy as np
from forehead_flow_rule import forehead_chart


def make_forehead_gap_fixture():
    radii = (.15, .13, .11, .09, .07)
    angles = (1.4, 1.2, 1., .8, .6, .4, .25, .15, 0., -.1, -.25, -.4, -.7, -1., -1.3)
    names = ('red', 'orange', 'green', 'blue', 'purple')
    rows, world = [], []
    for radius in radii:
        chart = [(u, 1.65+radius) for u in (0., .01, .02, .03)]
        chart += [(.03+radius*np.cos(a), 1.65+radius*np.sin(a)) for a in angles]
        row = []
        for u, height in chart:
            row.append(len(world))
            world.append((.2*np.sin(u/.2), -.2*np.cos(u/.2), height))
        rows.append(row)
    width = len(rows[0])
    faces = [[rows[r][c], rows[r+1][c], rows[r+1][c+1], rows[r][c+1]]
             for r in range(len(rows)-1) for c in range(width-1)]
    ends = (.25, .15, -.25, -.7, -1.3)
    paths = {name: rows[i][:5+angles.index(end)]
             for i, (name, end) in enumerate(zip(names, ends))}
    references = {name: path[:] for name, path in paths.items()}
    references['green'] = rows[2][:5+angles.index(.15)]
    old = np.asarray(world)
    author = {'vertices': old.tolist(), 'faces': faces, 'regions': [1]*len(faces),
              'report': {'semantic_rows': [1.5, 1.6, 1.645, 1.65, 1.68],
                         'head_assembly_y_offset_m': 0.}}
    proposal = {'vertices': old.tolist(), 'source_chart': forehead_chart(old).tolist(),
                'source_faces': faces, 'paths': paths, 'lineage': {},
                'protected': sorted({v for path in paths.values() for v in (path[0], path[-1])})}
    return author, proposal, references
