"""Add conforming support stars where redistributed rails span the temple surface."""
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from forehead_flow_rule import forehead_chart
from forehead_star_topology import ForeheadStarTopology, forehead_edge_counts


def refine_forehead_surface_supports(old, original_faces, regions, proposal):
    old = np.asarray(old, float)
    source = AuthorQuadSurface(np.asarray(proposal['source_chart']), proposal['source_faces'],
                               [1]*len(proposal['source_faces']))
    source.vertices = old
    class FrozenBuilder(ForeheadStarTopology):
        def add_point(self, chart_point, owner_ids):
            donors, weights, point = source.lineage(chart_point[0], chart_point[2])
            index = len(self.points)
            self.points.append(np.asarray(point, np.float32).astype(float).tolist())
            self.chart.append(list(map(float, chart_point)))
            self.lineage[index] = {'donors': list(donors), 'weights': list(weights)}
            return index
    topology = proposal['topology']
    builder = FrozenBuilder.__new__(FrozenBuilder)
    builder.old = old
    builder.original = [list(q) for q in original_faces]
    builder.regions = list(regions)
    builder.points = np.asarray(proposal['vertices']).tolist()
    builder.old_chart = forehead_chart(old)
    builder.chart = builder.old_chart.tolist()+[None]*(len(builder.points)-len(old))
    builder.faces = proposal['faces']
    builder.owners = topology['face_owners']
    builder.paths = proposal['paths']
    builder.lineage = dict(proposal['lineage'])
    builder.operations = topology['operations']
    builder.original_edges = forehead_edge_counts(original_faces)
    builder.edge_parent = {tuple(x['edge']): x['parent'] for x in topology['edge_lineage']}
    builder.surfaces = {}
    for i, row in builder.lineage.items():
        builder.chart[i] = (np.asarray(row['weights']) @ builder.old_chart[row['donors']]).tolist()
    binding = topology['roles']['original_binding']
    purple, blue = binding['positive_paths']['purple'], binding['positive_paths']['blue']
    bend = topology['roles']['original_corner_roles']['purple'][0]['station']
    pole = binding['side_pole_evidence'][0]
    if purple[bend+1] != pole['point']:
        raise ValueError('Unrecognized temple support pole')
    pivots = [purple[bend-1], blue[bend-1], purple[bend+1], blue[bend-2],
              pole['rejected_inward_support']]
    initial_count = len(builder.points)
    for pivot in pivots:
        builder.refine_star(pivot)
    changed = [i for i, (q, owner) in enumerate(zip(builder.faces, builder.owners))
               if q != original_faces[owner]]
    old_ids = sorted(set(proposal['affected_old']) | {builder.owners[i] for i in changed})
    new_ids = sorted(set(proposal['affected_new']) | set(changed))
    topology.update(vertices=builder.points, chart=builder.chart, faces=builder.faces,
                    face_owners=builder.owners, lineage=builder.lineage, paths=builder.paths,
                    operations=builder.operations,
                    retiled_old_faces=sorted({builder.owners[i] for i in changed}),
                    retiled_new_faces=changed,
                    edge_lineage=[{'edge': list(e), 'parent': p} for e, p in builder.edge_parent.items()])
    extra = set(range(initial_count, len(builder.points)))
    proposal.update(vertices=np.asarray(builder.points), faces=builder.faces,
                    lineage=builder.lineage, paths=builder.paths, affected_old=old_ids,
                    affected_new=new_ids, topology=topology,
                    moving=sorted(set(proposal['moving']) | extra),
                    support=sorted(set(proposal['support']) | extra))
    return proposal
