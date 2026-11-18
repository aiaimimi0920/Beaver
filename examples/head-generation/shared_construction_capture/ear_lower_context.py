"""Build exact lower-fairing lookup context from sealed producer and weld IDs."""
from copy import deepcopy
from construction_topology import digest
from ear_join import combined_targets, _sealed


def prepare_reports(packet, reports):
    """Return isolated report copies for the private identity-only instrument.

No coordinates select any source or target. Report names are bound to the
captured native part ordinal, not inferred from side or geometric proximity.
"""
    targets=combined_targets(packet)
    membership=packet['ear_membership'];_sealed(membership)
    if membership['targets']!=targets:raise ValueError('Ear membership belongs to another attachment context')
    weld=packet['native_weld'];before=weld['before']['point_ids']
    post=weld['mesh_quotient']['point_sources']
    images={}
    for target,group in enumerate(post):
        for source in group:
            pid=before[source]
            if pid in images:raise ValueError('Duplicate source image in actual Mesh quotient')
            images[pid]=target
    source=packet['native_join']['source'];captured=packet['ear_join'];out=[];used=set()
    for receipt,origin in zip(captured['ears'],captured['origins']):
        ordinal=origin['part_ordinal'];part=source['parts'][ordinal]
        # Native join source records object name; do not guess key aliases.
        name=part['name']
        matches=[r for r in reports if r['name']==name]
        if len(matches)!=1 or name in used:raise ValueError('Actual ear report/native part binding missing')
        used.add(name);report=deepcopy(matches[0]);ear=receipt['ear_source']
        lower=receipt['lower_bridge']['ear_index']
        faces=[f for f in ear['faces'] if lower in f]
        if len(faces)!=1 or len(faces[0])!=3:raise ValueError('Actual lower bridge is not its unique original triangle')
        binding=origin['evaluation_binding'];_sealed(binding)
        source_tuple=binding['lower_bridge_triangle']
        if len(source_tuple)!=3 or set(source_tuple)!=set(faces[0]):
            raise ValueError('Original report bridge order is not its actual allocated face')
        triangle=[images[ear['point_ids'][i]] for i in source_tuple]
        roots=[images[r['skin_point_id']] for r in receipt['pairs'] if r['allocation_kind']=='root_row']
        lower_target=images[ear['point_ids'][lower]]
        if len(roots)!=len(report['root_positions']) or lower_target not in triangle:
            raise ValueError('Actual report allocation coverage changed')
        report['_identity_bridge_triangle']=triangle
        report['_identity_root_indices']=roots
        report['_identity_lower']=lower_target
        out.append(report)
    if used!={r['name'] for r in reports} or len(reports)!=2:raise ValueError('Unbound extra ear report')
    return out,{'source_sha256':captured['sha256'],'membership_sha256':membership['sha256'],
                'contexts_sha256':digest(out),'native_fairing_validated':False,'model_admission':False}
