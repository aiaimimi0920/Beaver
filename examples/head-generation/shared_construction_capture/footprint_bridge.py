"""Translate verified native storage to the reviewed semantic footprint protocol."""
from corner_ledger import corner_id
from corner_transport import SYSTEM,PROPS,scalar,schema
from joint_binding import digest,require,seal

TYPES={'FLOAT':'float','INT':'int','BOOLEAN':'bool','FLOAT_VECTOR':'float3',
       'FLOAT2':'float2','FLOAT_COLOR':'float4'}


def footprint(obj,point_ids,face_ids,invocation):
    mesh=obj.data
    inventory=schema(mesh)
    require(len(point_ids)==len(mesh.vertices) and len(face_ids)==len(mesh.polygons),
            'Verified storage identities missing')
    domains={'point':{'position':'float3','select':'bool','hide':'bool'},
             'face':{'material':'int','smooth':'bool','select':'bool','hide':'bool'},
             'edge':{'seam':'bool','sharp':'bool','select':'bool','hide':'bool'},
             'corner':{},'uv':{}}
    point_fields={pid:{'position':list(v.co),'select':bool(v.select),'hide':bool(v.hide)}
                  for pid,v in zip(point_ids,mesh.vertices)}
    for group in obj.vertex_groups:
        name='group:'+str(group.index)+':'+group.name;domains['point'][name]='float'
        for pid,vertex in zip(point_ids,mesh.vertices):
            point_fields[pid][name]=next((g.weight for g in vertex.groups if g.group==group.index),0.)
    face_values=[{'material':p.material_index,'smooth':bool(p.use_smooth),'select':bool(p.select),'hide':bool(p.hide)}
                 for p in mesh.polygons]
    edge_values=[{'seam':bool(e.use_seam),'sharp':bool(e.use_edge_sharp),'select':bool(e.select),'hide':bool(e.hide)}
                 for e in mesh.edges]
    corner_values=[{} for _ in mesh.loops];uv=[{} for _ in mesh.loops]
    for a in inventory:
        domain=a['domain'].lower()
        domain='uv' if domain=='corner' and a['type']=='FLOAT2' else domain
        require(a['name'] not in domains[domain],'Field name collides with explicit system field')
        domains[domain][a['name']]=TYPES[a['type']]
        values=[scalar(getattr(v,PROPS[a['type']])) for v in mesh.attributes[a['name']].data]
        target={'point':list(point_fields.values()),'face':face_values,'edge':edge_values,
                'corner':corner_values,'uv':uv}[domain]
        require(len(target)==len(values),'Attribute domain cardinality mismatch')
        for row,value in zip(target,values):row[a['name']]=value
    for layer in mesh.uv_layers:
        for flag in ('pin_uv','select','select_edge'):
            name=layer.name+':'+flag;domains['corner'][name]='bool'
            for row,value in zip(corner_values,layer.data):row[name]=bool(getattr(value,flag))
    faces=[]
    for p,fid in zip(mesh.polygons,face_ids):
        ps=[point_ids[i] for i in p.vertices]
        corners=[{'id':corner_id(fid,ps[j],ps[(j+1)%len(ps)]),'point':ps[j],
                  'uv':uv[index],'categorical':corner_values[index]} for j,index in enumerate(p.loop_indices)]
        faces.append({'id':fid,'points':ps,'corners':corners,'categorical':face_values[p.index]})
    field_schema={'schema':'complete-stage-fields-v1','stage_id':invocation['stage_id'],'domains':domains}
    invocation=dict(invocation,field_schema_sha256=digest(field_schema))
    ledger={'side':invocation['side'],'stage_id':invocation['stage_id'],'point_ids':point_ids,
            'point_fields':point_fields,'field_schema':field_schema,'faces':faces,
            'edges':[{'points':[point_ids[i] for i in e.vertices],'categorical':edge_values[e.index]}for e in mesh.edges]}
    descriptor,seal_sha=seal(invocation,ledger)
    return {'descriptor':descriptor,'sha256':seal_sha,'schema_observed_from_actual_stage':True,
            'baseline_fullfield_equality_required':True,'candidate_acceptance':False}
