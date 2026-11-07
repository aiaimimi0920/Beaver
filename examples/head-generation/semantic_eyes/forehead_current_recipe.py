"""Reproducible five-flow placement on the explicitly regularized author surface."""
import numpy as np
from nasal_source_domain import current_source_features
from forehead_retile_roles import prepare_forehead_star_topology
from forehead_retile_fair import fair_forehead_guides
from forehead_retile_support import refine_forehead_supports
from forehead_redistribute import redistribute_forehead_guides
from forehead_fit_supports import fit_forehead_supports
from forehead_surface_supports import refine_forehead_surface_supports
from forehead_fit_density import fit_forehead_density
from forehead_retile_mirror import mirror_forehead_retile


def prepare_current_forehead(vertices, faces, regions, ordinals, report, open_delta):
    features = current_source_features(vertices, faces, regions, ordinals)
    topology = prepare_forehead_star_topology(vertices, faces, regions, report, features)
    proposal = fair_forehead_guides(vertices, faces, regions, topology, features)
    proposal = refine_forehead_supports(vertices, open_delta, proposal)
    initial_count = len(proposal['vertices'])
    proposal = redistribute_forehead_guides(vertices, faces, regions, report, features, proposal)
    proposal = fit_forehead_supports(vertices, open_delta, proposal)
    proposal = refine_forehead_surface_supports(vertices, faces, regions, proposal)
    proposal['source_chart'] = np.asarray(proposal['source_chart'])
    # Every new surface sample participates, including samples on the guide rails.
    extra = set(range(initial_count, len(proposal['vertices'])))
    proposal['support'] = sorted(set(proposal['support']) | extra)
    proposal['moving'] = sorted(set(proposal['moving']) | extra)
    proposal = fit_forehead_supports(vertices, open_delta, proposal, sparse_warm=True)
    proposal = fit_forehead_density(vertices, open_delta, proposal)
    proposal = fit_forehead_density(vertices, open_delta, proposal, focus=True)
    bases = [ordinals['Ear hollow pinna '+str(side)] for side in (-1, 1)]
    owners = {a+offset: b+offset for a, b in ((bases[0], bases[1]), (bases[1], bases[0]))
              for offset in (0, 100, 110)}
    proposal['topology']['roles']['green_endpoint_policy'] = (
        'Pinned actual fitted upper pinna root, resolved in assembly coordinates')
    return mirror_forehead_retile(vertices, faces, regions, proposal, owners, open_delta)
