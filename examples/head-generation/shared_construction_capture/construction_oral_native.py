"""Explicit oral allocation handoff and real pre-join donor fields; no surgery grant."""
from construction_topology import cycle, digest, stable_id
from construction_markers import fields
from construction_native import _object_key, _mesh_key, _topology_digest, _state_for, join_objects
from construction_oral_contract import expand_half_boundary, bind_oral_ring

ORALS = {}


def begin_oral(journal, slots):
    slots = list(slots)
    if any(type(i) is not int or not 0 <= i < len(journal.points) for i in slots):
        raise ValueError("Oral handoff requires actual allocated skin slots")
    ids = [journal.points[i]['point_id'] for i in slots]
    seams = [journal.points[i]['point_id'] for i in slots if i in journal.seam]
    expanded = expand_half_boundary(ids, seams)
    return {'skin_namespace': journal.namespace, 'half_point_ids': ids, 'seam_point_ids': seams,
            'expanded_stations': expanded, 'source_allocation_slots': slots}


def bind_oral(obj, handoff, rings, vertices, faces):
    if handoff is None:
        return
    key = _object_key(obj)
    if key in ORALS or any(row['name'] == obj.name for row in ORALS.values()):
        raise ValueError("Oral allocation object identity collision")
    if not rings or any(len(ring) != len(handoff['expanded_stations']) for ring in rings):
        raise ValueError("Oral rings lost explicit parent station coverage")
    flat = [i for ring in rings for i in ring]
    if any(type(i) is not int for i in flat) or flat != list(range(len(vertices))):
        raise ValueError("Oral allocation ordinals must cover the actual vertex stream")
    data = obj.data
    if len(data.vertices) != len(vertices) or len(data.polygons) != len(faces):
        raise ValueError("Oral mesh helper changed allocation topology")
    if any(cycle(list(face.vertices)) != cycle(list(source)) for face, source in zip(data.polygons, faces)):
        raise ValueError("Oral mesh helper changed oriented source faces")
    namespace = digest(['oral_allocation', handoff])
    ids = [None] * len(flat)
    for ring, slots in enumerate(rings):
        for station, i in enumerate(slots):
            ids[i] = stable_id(namespace, 'point', [ring, station])
    packet = {'schema': 'ORAL_ALLOCATION_CAPTURE_V1', 'handoff': handoff, 'point_ids': ids,
              'ring_slots': [list(ring) for ring in rings], 'faces': [list(face) for face in faces],
              'source_namespace': namespace, 'field_authority': False, 'solver_ready': False}
    packet['sha256'] = digest(packet)
    ORALS[key] = {'object': obj, 'name': obj.name, 'mesh_key': _mesh_key(obj),
                  'topology_sha256': _topology_digest(obj), 'packet': packet}


def oral_state(obj):
    row = ORALS.get(_object_key(obj))
    if row is None:
        return None
    packet = row['packet']
    if packet['sha256'] != digest({k: v for k, v in packet.items() if k != 'sha256'}):
        raise ValueError("Changed original oral allocation receipt")
    if row['object'] is not obj or row['name'] != obj.name or row['mesh_key'] != _mesh_key(obj):
        raise ValueError("Oral constructor object replaced or renamed")
    if row['topology_sha256'] != _topology_digest(obj):
        raise ValueError("Oral allocation topology changed without a receipt")
    return row


def join_with_oral_capture(members):
    oral_rows = [(i, obj, oral_state(obj)) for i, obj in enumerate(members)]
    oral_rows = [row for row in oral_rows if row[2] is not None]
    if len(oral_rows) != 1:
        raise ValueError("Scoped oral join requires one actual captured oral constructor")
    ordinal, oral, oral_record = oral_rows[0]
    handoff = oral_record['packet']['handoff']
    skin_rows = [(i, obj, _state_for(obj)) for i, obj in enumerate(members)]
    skin_rows = [row for row in skin_rows if row[2] is not None and
                 row[2]['packet']['source_namespace'] == handoff['skin_namespace']]
    if len(skin_rows) != 1:
        raise ValueError("Oral handoff has no unique live constructor shell")
    skin_ordinal, skin, state = skin_rows[0]
    role = state['packet']['construction_roles'].get('mouth/ring/0')
    if not role or role['segments'] != [handoff['half_point_ids']] or not role['complete_declared_path']:
        raise ValueError("Oral source handoff was lost during skin compaction")
    if any(_mesh_key(obj) == _mesh_key(other) for i, obj in enumerate(members) for other in members[i + 1:]):
        raise ValueError("Shared native mesh datablock cannot define separate donor parts")
    from construction_oral_fields import donor_schemas
    schemas = donor_schemas([fields(obj) for obj in members])
    skin_fields, oral_fields = fields(skin), fields(oral)
    if digest(skin_fields['matrix_world']) != digest(oral_fields['matrix_world']):
        raise ValueError("Scoped oral donor fixture requires identical object frames")
    skin_ids = list(state['mirrored_point_ids'])
    semantic = oral_record['packet']
    packets = join_objects(members)
    if len(packets) != 1 or packets[0] is not state['packet']:
        raise ValueError("Oral capture requires exactly one construction shell packet")
    source = packets[0]['native_join']['source']
    part = source['parts'][ordinal]
    oral_ids = source['points'][part['point_offset']:part['point_offset'] + part['counts'][0]]
    boundary = bind_oral_ring(handoff['half_point_ids'], handoff['seam_point_ids'],
                             handoff['skin_namespace'], oral_ids, semantic['ring_slots'][0])
    receipt = {'schema': 'ORAL_PREJOIN_DONORS_V1', 'boundary': boundary, 'oral_allocation': semantic,
               'skin_point_ids': skin_ids, 'skin_fields': skin_fields, 'oral_fields': oral_fields, 'point_schemas': schemas,
               'skin_part_ordinal': skin_ordinal, 'oral_part_ordinal': ordinal,
               'join_source_sha256': packets[0]['native_join']['source_sha256'],
               'scope': 'declared oral boundary only; ear and later surgery remain unadmitted'}
    receipt['sha256'] = digest(receipt)
    packets[0]['oral_join'] = receipt
    ORALS.pop(_object_key_value(oral_record), None)
    return packets


def _object_key_value(record):
    # A consumed join object cannot be dereferenced. Locate its retained registry row.
    keys = [key for key, row in ORALS.items() if row is record]
    if len(keys) != 1:
        raise ValueError("Consumed oral constructor registry identity changed")
    return keys[0]
