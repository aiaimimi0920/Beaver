"""Deterministic skin POINT ownership only on the shared oral boundary."""
import inspect


def weld_point_value(item, kind):
    field = {'FLOAT':'value','INT':'value','BOOLEAN':'value',
             'FLOAT_VECTOR':'vector','FLOAT2':'vector','FLOAT_COLOR':'color','BYTE_COLOR':'color'}[kind]
    value = getattr(item,field)
    return tuple(value) if field in ('vector','color') else value


def weld_zero_value(kind):
    if kind in ('FLOAT_COLOR','BYTE_COLOR'): return (0,0,0,0)
    if kind == 'FLOAT_VECTOR': return (0,0,0)
    if kind == 'FLOAT2': return (0,0)
    return False if kind == 'BOOLEAN' else 0


def capture_oral_skin_donors(members, target):
    from mathutils.kdtree import KDTree
    skin = [o for o in members if o.get('skin_detail') and o.vertex_groups.get('Continuous facial shell')]
    oral = [o for o in members if o.name.startswith('Oral cavity inner lips and back wall')]
    if len(skin) != 1 or len(oral) != 1:
        raise ValueError('Expected unique authored skin and oral source donors')
    skin,oral = skin[0],oral[0]
    schemas = {}
    for obj in members:
        for attr in obj.data.attributes:
            if attr.domain != 'POINT' or attr.name == 'position' or attr.name.startswith('.'): continue
            if attr.data_type not in ('FLOAT','INT','BOOLEAN','FLOAT_VECTOR','FLOAT2','FLOAT_COLOR','BYTE_COLOR'):
                raise ValueError('Unsupported skin weld POINT type: '+attr.data_type)
            if attr.name in schemas and schemas[attr.name] != attr.data_type:
                raise ValueError('Conflicting point attribute schemas')
            schemas[attr.name] = attr.data_type
    if schemas.get('Color') != 'FLOAT_COLOR': raise ValueError('Missing source Color contract')
    skin_regions = {int(v.value) for v in skin.data.attributes['NormalRegion'].data}
    oral_regions = {int(v.value) for v in oral.data.attributes['NormalRegion'].data}
    if not skin_regions <= {1,2} or skin_regions & oral_regions:
        raise ValueError('Unexpected authored skin/oral region mapping')
    transform = target.matrix_world.inverted()
    sx,ox = transform @ skin.matrix_world,transform @ oral.matrix_world
    if any(abs(matrix[i][j]-(1.0 if i==j else 0.0))>1e-9 for matrix in (sx,ox) for i in range(3) for j in range(3)):
        raise ValueError("Source donor vectors require identical object axes and scale")
    tree = KDTree(len(oral.data.vertices))
    for vertex in oral.data.vertices: tree.insert(ox @ vertex.co,vertex.index)
    tree.balance(); records = []
    for vertex in skin.data.vertices:
        position = sx @ vertex.co
        hits = tree.find_range(position,1e-6)
        if not hits: continue
        if len(hits) != 1: raise ValueError('Ambiguous pre-weld oral donor')
        values = {}
        for name,kind in schemas.items():
            attr = skin.data.attributes.get(name)
            values[name] = weld_point_value(attr.data[vertex.index],kind) if attr else weld_zero_value(kind)
        groups = {skin.vertex_groups[g.group].name:float(g.weight) for g in vertex.groups}
        if groups.get('Continuous facial shell') != 1.0:
            raise ValueError('Skin donor lost its authored membership')
        records.append({'position':tuple(position),'values':values,'groups':groups})
    if not records: raise ValueError('No shared oral boundary donors found')
    return {'records':records,'schemas':schemas,'skin_regions':skin_regions,'oral_regions':oral_regions}


def restore_oral_skin_donors(obj, snapshot):
    from mathutils.kdtree import KDTree
    data = obj.data; regions = data.attributes['NormalRegion']
    membership = [set() for _ in data.vertices]
    for poly in data.polygons:
        for index in poly.vertices: membership[index].add(int(regions.data[poly.index].value))
    targets = [i for i,rs in enumerate(membership) if rs & snapshot['skin_regions'] and rs & snapshot['oral_regions']]
    records = snapshot['records']
    if len(targets) != len(records): raise ValueError('Shared oral weld cardinality changed')
    tree = KDTree(len(records))
    for index,record in enumerate(records): tree.insert(record['position'],index)
    tree.balance(); seen = set(); changed = {}; group_changes = 0
    for index in targets:
        before_position = tuple(data.vertices[index].co)
        hits = tree.find_range(data.vertices[index].co,1e-6)
        if len(hits) != 1 or hits[0][1] in seen:
            raise ValueError('Ambiguous post-weld skin donor correspondence')
        donor = hits[0][1]; seen.add(donor); record = records[donor]
        for name,kind in snapshot['schemas'].items():
            attr = data.attributes.get(name)
            if not attr or attr.domain != 'POINT' or attr.data_type != kind:
                raise ValueError('Weld changed point attribute schema: '+name)
            item = attr.data[index]; value = record['values'][name]
            if weld_point_value(item,kind) != value: changed[name] = changed.get(name,0)+1
            field = {'FLOAT':'value','INT':'value','BOOLEAN':'value','FLOAT_VECTOR':'vector',
                     'FLOAT2':'vector','FLOAT_COLOR':'color','BYTE_COLOR':'color'}[kind]
            setattr(item,field,value)
        before_groups = {obj.vertex_groups[g.group].name:float(g.weight) for g in data.vertices[index].groups}
        if before_groups != record['groups']:
            group_changes += 1
            for group_index in [g.group for g in data.vertices[index].groups]: obj.vertex_groups[group_index].remove([index])
            for name,weight in record['groups'].items():
                group = obj.vertex_groups.get(name)
                if group is None: raise ValueError('Missing joined donor group: '+name)
                group.add([index],weight,'REPLACE')
        if tuple(data.vertices[index].co) != before_position:
            raise ValueError('POINT ownership restoration moved geometry')
    data.update()
    return {'boundary_vertices':len(targets),'point_attributes_changed':changed,'group_memberships_changed':group_changes,
            'geometry_changed':False,'loop_attributes_changed':False,'oral_only_changed':False,
            'source_policy':'Unique pre-weld skin donor owns POINT attributes and groups only at shared skin/oral boundary'}


def adapt(sources):
    out = dict(sources); pipeline = out['pipeline.py']
    before = '    bpy.context.view_layer.objects.active = members[0]\n    bpy.ops.object.join()'
    after = '        bm.to_mesh(members[0].data)\n        bm.free()\n        from ear_boundary import fair_lower_ear_boundaries'
    if pipeline.count(before) != 1 or pipeline.count(after) != 1:
        raise ValueError('Source weld ownership insertion contract changed')
    pipeline = pipeline.replace(before,
        '    _oral_skin_donors = capture_oral_skin_donors(members,members[0])\n'+before,1)
    pipeline = pipeline.replace(after,
        '        bm.to_mesh(members[0].data)\n        bm.free()\n'
        '        report["oral_skin_point_ownership"] = restore_oral_skin_donors(members[0],_oral_skin_donors)\n'
        '        from ear_boundary import fair_lower_ear_boundaries',1)
    out['pipeline.py'] = pipeline
    out['face.py'] += '\n\n'+'\n\n'.join(inspect.getsource(f) for f in
        (weld_point_value,weld_zero_value,capture_oral_skin_donors,restore_oral_skin_donors))+'\n'
    return out
