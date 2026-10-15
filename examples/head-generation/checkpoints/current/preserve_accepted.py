"""Preserve approved own-head geometry while regenerating eye accents."""

def is_eye_edit_zone(p):
    x,y,z=p
    return .018<abs(x)<.09 and 1.600<z<1.665 and -y>.035

def preserve_non_eye(data,root,report):
    import bpy,json,hashlib
    from mathutils.kdtree import KDTree
    from pathlib import Path
    base=root/'assets/aster/head_recovery_617'
    path=base/'aster_head_runtime_morph.blend'
    with bpy.data.libraries.load(str(path),link=False) as (source,target):
        assert 'Face' in source.objects
        target.objects=['Face']
    obj=target.objects[0]
    assert obj and obj.type=='MESH'
    original=[v.co.copy() for v in data.vertices]
    tree=KDTree(len(obj.data.vertices))
    for v in obj.data.vertices:tree.insert(v.co,v.index)
    tree.balance()
    selected=[v.index for v in data.vertices if not is_eye_edit_zone(v.co)]
    matches=[]
    for i in selected:
        p,j,d=tree.find(data.vertices[i].co)
        assert d<.001,('Approved-base correspondence missing',i,d)
        matches.append((i,p.copy(),d))
    for i,p,d in matches:data.vertices[i].co=p
    data.update()
    assert all(p.area>1e-12 for p in data.polygons),'Preservation collapsed a face'
    assert all((v.co-original[v.index]).length==0 for v in data.vertices if v.index not in selected)
    previous=json.loads((base/'generation_report.json').read_text())
    report['ear_root_fit']=previous['ear_root_fit']
    result={'source':'own accepted 617','source_blend_sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'preserved_vertices':len(selected),'maximum_removed_regeneration_drift_m':max(d for _,_,d in matches),'eye_edit_zone_untouched':True,'note':'Approved geometry checkpoint; no reference-model vertices copied.'}
    bpy.data.objects.remove(obj,do_unlink=True)
    return result
