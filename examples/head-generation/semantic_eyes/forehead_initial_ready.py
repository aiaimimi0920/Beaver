"""Bounded source-owned initialization and honest shared-atlas readiness."""
import copy
import numpy as np
from forehead_initial_construction import forehead_initial_construct
from forehead_initial_sector import forehead_initial_pinned_sector_obstructions
from forehead_author_atlas import ForeheadFrozenAuthorAtlas
from forehead_author_gap_samples import ForeheadFrozenGapResidual
from forehead_surface_batch import make_forehead_surface_batch
from nasal_source_geometry import AuthorQuadSurface


def forehead_initial_atlas_preflight(author, proposal, atlas):
    source = np.asarray(author['vertices'], float)
    points = np.asarray(proposal['vertices'], float)
    chart = np.asarray(proposal['source_chart'], float)
    if not np.array_equal(atlas.old, source) or len(chart) != len(source):
        raise ValueError('Initial atlas source ownership differs')
    parameters = np.vstack((chart, np.zeros((len(points)-len(source), 3))))
    for index, row in proposal['lineage'].items():
        parameters[int(index)] = np.asarray(row['weights']) @ chart[row['donors']]
    batch = make_forehead_surface_batch(chart, source, np.asarray(proposal['source_faces']))
    scalar = AuthorQuadSurface(chart, proposal['source_faces'], [1]*len(proposal['source_faces']))
    scalar.vertices = source.copy()
    evidence = []
    for count in (129, 1025):
        section = ForeheadFrozenGapResidual(atlas, proposal, count)
        result = section.evaluate(points, parameters, details=True)
        if not result['valid']:
            return {'ready': False, 'reason': result['reason'], 'checks': evidence}
        queries = np.concatenate([hits for row in result['details'] for hits in row['chart_hits']])
        if batch.sample(queries) is None:
            return {'ready': False, 'reason': 'Ambiguous or missing section source coverage', 'checks': evidence}
        for query in queries:
            try:
                scalar.lineage(*query)
            except ValueError:
                return {'ready': False, 'reason': 'Scalar section source coverage rejected', 'checks': evidence}
        evidence.append({'sections_per_chart': count, 'domains': len(result['details']),
                         'source_endpoints_checked': len(queries), 'batch_and_scalar_coverage': True})
    return {'ready': True, 'checks': evidence,
            'scope': 'Source atlas readiness only, not equal spacing or model acceptance'}


def initialize_forehead_shared(author, native_source_fields, features=None, *,
                               max_graph_attempts=3, construct_options=None,
                               provenance=None, atlas=None):
    """Rebuild only for proved immutable-sector obstructions, never mask tails.

    Coordinate pins remain fixed. Only nonterminal graph transit is excluded.
    A movable atlas defect returns an explicitly unready proposal for separate
    conditioning. A failed graph/birth solve has no accepted proposal.
    """
    if not isinstance(max_graph_attempts, int) or not 1 <= max_graph_attempts <= 4:
        raise ValueError('Invalid bounded initial graph attempt count')
    frame = ForeheadFrozenAuthorAtlas(author) if atlas is None else atlas
    options = copy.deepcopy(construct_options or {})
    if 'blocked_transits' in options:
        raise ValueError('Transit exclusions must come from immutable sector evidence')
    blocked, history = set(), []
    for attempt in range(max_graph_attempts):
        result = forehead_initial_construct(
            author['vertices'], author['faces'], author['regions'], author['report'],
            author.get('ordinals', {}), author['open_delta'], native_source_fields,
            features=features, provenance=provenance,
            blocked_transits=tuple(sorted(blocked)), **options)
        record = {'attempt': attempt+1, 'blocked_transits': sorted(blocked),
                  'construction_status': result['status']}
        history.append(record)
        if not result.get('admitted'):
            return {'admitted': False, 'status': result['status'], 'history': history,
                    'construction': result, 'scope': 'No committed initial proposal'}
        proposal = result['proposal']
        ready = forehead_initial_atlas_preflight(author, proposal, frame)
        record['atlas'] = ready
        if ready['ready']:
            proposal = copy.deepcopy(proposal)
            proposal.setdefault('initial_construction_admission', {})['matched_gap_readiness'] = 'verified'
            proposal['initial_atlas_preflight'] = ready
            proposal['initial_graph_history'] = copy.deepcopy(history)
            return {'admitted': True, 'status': 'initial_atlas_ready', 'local_init_only': True,
                    'proposal': proposal, 'history': history,
                    'scope': 'Initial geometry/fields/atlas only; final guide, shape, native and visual gates pending'}
        obstruction = forehead_initial_pinned_sector_obstructions(
            proposal['source_chart'], proposal['paths'], proposal['protected'],
            [frame.entry, frame.exit_h], len(author['vertices']))
        record['immutable_sector_obstructions'] = obstruction
        fresh = set(obstruction['blocked_transits'])-blocked
        if not fresh:
            proposal = copy.deepcopy(proposal)
            proposal.setdefault('initial_construction_admission', {})['matched_gap_readiness'] = 'rejected'
            proposal['initial_atlas_preflight'] = ready
            return {'admitted': False, 'local_init_only': True,
                    'status': 'initial_atlas_conditioning_required', 'proposal': proposal,
                    'history': history, 'scope': 'Do not invoke matched-gap fitting on this unready atlas'}
        blocked.update(fresh)
    return {'admitted': False, 'status': 'initial_graph_budget_exhausted',
            'history': history, 'scope': 'No final or atlas-ready model claimed'}
