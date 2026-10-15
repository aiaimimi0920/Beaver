"""Native-only optional-component selection before atlas/normal/morph assembly."""
import bpy
from mouth import store_mouth_deltas

SUPPLEMENTAL_PREFIXES = (
    'Inner forked lash blade ', 'Upper swept lash tuft ', 'Attached outer lash wing ',
    'Outer lateral eye liner ', 'Outer companion lash ', 'Lower outer lash tuft ',
    'Embedded lateral child ',
)


def select_components(parts, parameters):
    removed = []
    for obj in list(parts):
        omit = (parameters['supplemental_lashes'] == 0 and obj.name.startswith(SUPPLEMENTAL_PREFIXES))
        omit = omit or (parameters['lid_fold'] == 0 and obj.name.startswith('Tapered upper eyelid fold '))
        if omit:
            removed.append(obj.name)
            parts.remove(obj)
            bpy.data.objects.remove(obj, do_unlink=True)
    # Evaluated lash reconstruction replaces mesh datablocks and may discard
    # their earlier delta attributes. Recreate only missing data before joining.
    for obj in parts:
        attr = obj.data.attributes.get('MouthOpenDelta')
        if attr is None:
            store_mouth_deltas(obj)
        else:
            assert attr.domain == 'POINT' and len(attr.data) == len(obj.data.vertices)
    return {'removed_components': removed, 'selection_stage': 'after attachment checks; before atlas and normal regions',
            'preselection_checks_are_not_final_component_counts': True}


def final_runtime_evidence(path):
    import hashlib
    from pathlib import Path
    from collections import Counter
    obj = bpy.data.objects.get('Face')
    assert obj is not None and obj.type == 'MESH'
    attr = obj.data.attributes.get('NormalRegion')
    assert attr is not None and attr.domain == 'FACE'
    counts = Counter(int(item.value) for item in attr.data)
    assert counts[47] > 0 and counts[48] > 0, 'Missing preserved ear normal regions'
    return {'runtime_blend_sha256': hashlib.sha256(Path(path).read_bytes()).hexdigest(),
            'final_normal_region_counts': dict(sorted(counts.items())),
            'ear_region_ids': [47, 48], 'ear_region_face_counts': {47: counts[47], 48: counts[48]}}
