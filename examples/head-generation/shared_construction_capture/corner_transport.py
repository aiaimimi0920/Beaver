"""Private exact Mesh/BMesh directed-loop transport, no coordinate matching."""
import copy
from corner_ledger import CornerLedger
from joint_binding import digest, require

# Structural arrays are compared explicitly, never treated as transferable fields.
SYSTEM={'position','edge_verts','.edge_verts','.corner_vert','.corner_edge'}
SUPPORTED={'FLOAT','INT','BOOLEAN','FLOAT_VECTOR','FLOAT2','FLOAT_COLOR'}
PROPS={'FLOAT':'value','INT':'value','BOOLEAN':'value','FLOAT_VECTOR':'vector',
       'FLOAT2':'vector','FLOAT_COLOR':'color','BYTE_COLOR':'color'}


def scalar(value):
    return value if isinstance(value,(bool,int,float,str)) else list(value)


def mesh_state(mesh):
    require(mesh.shape_keys is None, 'Selftest transport does not admit shape keys')
    attributes={}
    for a in mesh.attributes:
        if a.name in SYSTEM:
            continue
        require(a.data_type in PROPS, 'Unsupported visible attribute '+a.name+':'+a.data_type)
        attributes[a.name]={'domain':a.domain,'type':a.data_type,
                            'values':[scalar(getattr(v,PROPS[a.data_type])) for v in a.data]}
    return {'points':[list(v.co) for v in mesh.vertices],
            'edges':[sorted(e.vertices) for e in mesh.edges],
            'faces':[list(p.vertices) for p in mesh.polygons],
            'corners':[[l.vertex_index,l.edge_index] for l in mesh.loops],
            'face_flags':[[p.material_index,p.use_smooth,p.hide,p.select] for p in mesh.polygons],
            'edge_flags':[[e.use_seam,e.use_edge_sharp,e.hide,e.select] for e in mesh.edges],
            'point_flags':[[v.hide,v.select] for v in mesh.vertices],
            'attributes':attributes,
            'uv_flags':{layer.name:[[v.pin_uv,v.select,v.select_edge] for v in layer.data]
                        for layer in mesh.uv_layers}}


def schema(mesh):
    result=[]
    for a in mesh.attributes:
        if a.name in SYSTEM:
            continue
        require(a.data_type in SUPPORTED,
                'Unsupported storage '+a.data_type+' for '+a.name+'; BYTE_COLOR allocation not admitted')
        require(a.domain in ('POINT','EDGE','FACE','CORNER'), 'Unsupported attribute domain')
        result.append({'name':a.name,'domain':a.domain,'type':a.data_type})
    return sorted(result,key=lambda a:(a['domain'],a['name']))


def mesh_corners(mesh):
    result=[]
    layers=[a for a in mesh.attributes if a.domain=='CORNER' and a.name not in SYSTEM]
    for f in mesh.polygons:
        points=list(f.vertices)
        for j,index in enumerate(f.loop_indices):
            values={a.name:scalar(getattr(a.data[index],PROPS[a.data_type])) for a in layers}
            flags={layer.name:[layer.data[index].pin_uv,layer.data[index].select,
                              layer.data[index].select_edge] for layer in mesh.uv_layers}
            result.append({'face':f.index,'point':points[j],'next':points[(j+1)%len(points)],
                           'values':values,'uv_flags':flags})
    return result


def bmesh_corners(bm, inventory):
    bm.verts.index_update();bm.edges.index_update();bm.faces.index_update()
    corner=[a for a in inventory if a['domain']=='CORNER']
    kind={'FLOAT':'float','INT':'int','BOOLEAN':'int','FLOAT_VECTOR':'float_vector',
          'FLOAT_COLOR':'float_color','FLOAT2':'uv'}
    layers={}
    for a in corner:
        collection=getattr(bm.loops.layers,kind[a['type']])
        layer=collection.get(a['name'])
        require(layer is not None,'Missing BMesh corner layer '+a['name'])
        layers[a['name']]=layer
    result=[]
    for f in bm.faces:
        for l in f.loops:
            values={};flags={}
            for a in corner:
                value=l[layers[a['name']]]
                if a['type']=='FLOAT2':
                    values[a['name']]=list(value.uv)
                    flags[a['name']]=[value.pin_uv,value.select,value.select_edge]
                elif a['type']=='BOOLEAN':values[a['name']]=bool(value)
                else:values[a['name']]=scalar(value)
            result.append({'face':f.index,'point':l.vert.index,'next':l.link_loop_next.vert.index,
                           'values':values,'uv_flags':flags})
    return result


def layout(bm):
    bm.verts.index_update();bm.edges.index_update();bm.faces.index_update()
    return {'points':[list(v.co) for v in bm.verts],
            'edges':[sorted(v.index for v in e.verts) for e in bm.edges],
            'faces':[[v.index for v in f.verts] for f in bm.faces],
            'corners':[[l.vert.index,l.edge.index] for f in bm.faces for l in f.loops]}


def compare_entry(mesh,bm):
    inventory=schema(mesh)
    state=mesh_state(mesh)
    require(layout(bm)=={k:state[k] for k in ('points','edges','faces','corners')},
            'Mesh→BMesh indexed topology transport mismatch')
    require(mesh_corners(mesh)==bmesh_corners(bm,inventory),
            'Mesh→BMesh directed corner transport mismatch')
    return {'schema':inventory,'state':state,'corners':mesh_corners(mesh)}


def compare_exit(mesh,pending):
    state=mesh_state(mesh)
    require(schema(mesh)==pending['schema'],'Exit field schema changed')
    require({k:state[k] for k in ('points','edges','faces','corners')}==pending['layout'],
            'BMesh→Mesh indexed topology transport mismatch')
    require(mesh_corners(mesh)==pending['corners'],'BMesh→Mesh corner values or order changed')
    return state


def snapshot_source_faces(ledger,faces,points,cells,inventory):
    """Actual source face traversal; no sorted/global face-order substitution."""
    storage={a['name']:'float32' for a in inventory if a['domain']=='CORNER'}
    require(all(a['type'] in ('FLOAT2','FLOAT_COLOR') for a in inventory if a['domain']=='CORNER'),
            'Source assign_ear_loop only admits UV and float-color in this selftest')
    for face in faces:
        require(face in cells,'Source FACE has no allocation identity')
        samples=[]
        for loop in face.loops:
            values={};flags={}
            for a in inventory:
                if a['domain']!='CORNER':continue
                value=loop[a['handle']]
                values[a['name']]=list(value.uv if a['type']=='FLOAT2' else value)
                if a['type']=='FLOAT2':flags[a['name']]=[value.pin_uv,value.select,value.select_edge]
            samples.append({'point':points[loop.vert],'values':values,'categorical':flags})
        ledger.capture(cells[face],[points[v] for v in face.verts],samples,storage)


def fullfield_roundtrip_probe(obj,bm):
    """Expose all BMesh fields through a disposable mesh; never mutate source."""
    import bpy
    from types import SimpleNamespace
    from construction_markers import fields
    before=layout(bm)
    probe=obj.data.copy()
    try:
        bm.to_mesh(probe);probe.update()
        result=fields(SimpleNamespace(data=probe,vertex_groups=obj.vertex_groups,
                                     matrix_world=obj.matrix_world))
        require(layout(bm)==before,'Field probe changed live BMesh topology')
        return result
    finally:bpy.data.meshes.remove(probe)
