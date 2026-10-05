"""Append a new Face while preserving frozen Body/Hair primitives and bytes."""
import copy, json, struct

def read_glb(raw):
    n = struct.unpack_from('<I', raw, 12)[0]
    doc = json.loads(raw[20:20 + n])
    start = 20 + n
    return (doc, raw[start + 8:] if len(raw) > start else b'')

def merge_face(base_raw, new_raw):
    base, oldbin = read_glb(base_raw)
    new, newbin = read_glb(new_raw)
    before = copy.deepcopy(base)
    assert len(base['buffers']) == len(new['buffers']) == 1
    offset = len(oldbin)
    vo = len(base.get('bufferViews', []))
    ao = len(base.get('accessors', []))
    for v in new.get('bufferViews', []):
        v['buffer'] = 0
        v['byteOffset'] = v.get('byteOffset', 0) + offset
        base.setdefault('bufferViews', []).append(v)
    for a in new.get('accessors', []):
        if 'bufferView' in a:
            a['bufferView'] += vo
        if 'sparse' in a:
            for key in ['indices', 'values']:
                a['sparse'][key]['bufferView'] += vo
        base.setdefault('accessors', []).append(a)
    io = len(base.get('images', []))
    so = len(base.get('samplers', []))
    to = len(base.get('textures', []))
    mo = len(base.get('materials', []))
    for im in new.get('images', []):
        if 'bufferView' in im:
            im['bufferView'] += vo
        assert 'uri' not in im, 'Self-contained GLB required'
        base.setdefault('images', []).append(im)
    base.setdefault('samplers', []).extend(new.get('samplers', []))
    for tex in new.get('textures', []):
        if 'source' in tex:
            tex['source'] += io
        if 'sampler' in tex:
            tex['sampler'] += so
        for e in tex.get('extensions', {}).values():
            if isinstance(e, dict) and 'source' in e:
                e['source'] += io
        base.setdefault('textures', []).append(tex)

    def textures(value):
        if isinstance(value, dict):
            for k, v in value.items():
                if k.endswith('Texture') and isinstance(v, dict) and ('index' in v):
                    v['index'] += to
                else:
                    textures(v)
        elif isinstance(value, list):
            for v in value:
                textures(v)
    for material in new.get('materials', []):
        textures(material)
        base.setdefault('materials', []).append(material)
    assert len(new['meshes']) == 1 and new['meshes'][0]['name'] == 'Face'
    face = copy.deepcopy(new['meshes'][0])
    for p in face['primitives']:
        p['attributes'] = {k: v + ao for k, v in p['attributes'].items()}
        if 'indices' in p:
            p['indices'] += ao
        if 'material' in p:
            p['material'] += mo
        p['targets'] = [{k: v + ao for k, v in target.items()} for target in p.get('targets', [])]
        assert p['targets'], 'MouthOpen morph lost'
    assert face.get('extras', {}).get('targetNames') == ['MouthOpen']
    index = next((n['mesh'] for n in base['nodes'] if n.get('name') == 'Face'))
    base['meshes'][index] = face
    for node in base['nodes']:
        if node.get('mesh') == index:
            node['weights'] = [0.0]
    for key in ['extensionsUsed', 'extensionsRequired']:
        if key in new:
            base[key] = sorted(set(base.get(key, []) + new[key]))
    frozen = {}
    for role in ['Body', 'Hair']:
        i = next((n['mesh'] for n in base['nodes'] if n.get('name') == role))
        frozen[role] = base['meshes'][i] == before['meshes'][i]
        assert frozen[role]
    base['buffers'][0]['byteLength'] = offset + len(newbin)
    j = json.dumps(base, separators=(',', ':')).encode()
    j += b' ' * (-len(j) % 4)
    b = oldbin + newbin
    b += b'\x00' * (-len(b) % 4)
    raw = struct.pack('<III', 1179937895, 2, 28 + len(j) + len(b)) + struct.pack('<II', len(j), 1313821514) + j + struct.pack('<II', len(b), 5130562) + b
    return (raw, {'frozen_primitives': frozen, 'original_binary_prefix_preserved': b[:offset] == oldbin, 'morph_names': face['extras']['targetNames'], 'new_materials_embedded': len(new.get('materials', []))})
