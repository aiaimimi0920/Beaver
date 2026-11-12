"""Private native integration prototype; not installed into production pipeline."""
from copy import deepcopy
from construction_topology import digest, cycle
from construction_markers import fields
from construction_native import _state_for
from construction_oral_native import join_with_oral_capture
from construction_oral_fields import expected_transfer
from ear_contract import bind_ear_attachment, compose_attachment_targets
from ear_evaluated import EARS


def _sealed(value):
    if value.get('sha256') != digest({k:v for k,v in value.items() if k!='sha256'}):
        raise ValueError('Changed sealed attachment receipt')


def join_with_ear_capture(members):
    captured=[]
    shells=[(obj,_state_for(obj)) for obj in members if _state_for(obj) is not None]
    if len(shells)!=1 or shells[0][1]['phase']!='mirrored':
        raise ValueError('Actual verified mirrored constructor shell required')
    shell,state=shells[0]; shell_ids=state['mirrored_point_ids']
    shell_cycles=sorted(cycle([shell_ids[i] for i in f.vertices]) for f in shell.data.polygons)
    for ordinal,obj in enumerate(members):
        key=obj.as_pointer(); record=EARS.get(key)
        if record is None:continue
        if (record['object'] is not obj or record['mesh'] is not obj.data or record['name']!=obj.name
                or record['join_bound']):raise ValueError('Ear producer object identity retired/replaced')
        receipt=record['receipt'];_sealed(receipt)
        if record['source']!=receipt['ear_source'] or record['ordinal_to_semantic_id']!=receipt['ear_source']['point_ids']:
            raise ValueError('Ear registry changed independently of allocation receipt')
        binding=record['evaluation_binding'];_sealed(binding)
        expected_binding={'lower_bridge_triangle':binding['lower_bridge_triangle'],'schema':'private_ear_evaluation_binding_v1',
            'shell_namespace':state['source']['source_namespace'],
            'evaluation_mirror_sha256':digest(record['evaluation_mirror']),
            'evaluated_source_sha256':digest(receipt['evaluated_source']),
            'ear_source_sha256':digest(record['source']),
            'ordinal_to_semantic_sha256':digest(record['ordinal_to_semantic_id']),
            'ear_receipt_sha256':receipt['sha256']}
        triangle=binding['lower_bridge_triangle']
        lower=receipt['lower_bridge']['ear_index']
        if (len(triangle)!=3 or len(set(triangle))!=3 or lower not in triangle
                or sum(len(face)==3 and set(face)==set(triangle) for face in receipt['ear_source']['faces'])!=1):
            raise ValueError('Actual lower bridge producer tuple changed')
        if {k:v for k,v in binding.items() if k!='sha256'}!=expected_binding:
            raise ValueError('Ear evaluation provenance binding changed')
        if [list(f.vertices) for f in obj.data.polygons]!=record['source']['faces']:
            raise ValueError('Ear producer topology changed without receipt')
        if len(obj.data.vertices)!=len(record['source']['point_ids']):
            raise ValueError('Ear producer point domain changed')
        evaluated=receipt['evaluated_source']
        if set(evaluated['point_ids'])!=set(shell_ids) or sorted(cycle([evaluated['point_ids'][i] for i in f]) for f in evaluated['faces'])!=shell_cycles:
            raise ValueError('Actual destructive Mirror differs from ear evaluated source')
        payload=fields(obj)
        if payload['matrix_world']!=fields(shell)['matrix_world']:
            raise ValueError('Prototype ear join requires exact shared object frame')
        captured.append({'ordinal':ordinal,'registry_key':key,'record':record,'fields':payload})
    if len(captured)!=2 or {r['record']['receipt']['side'] for r in captured}!={-1,1}:
        raise ValueError('Actual bilateral ear producers required')
    packets=join_with_oral_capture(members)
    if len(packets)!=1:raise ValueError('Unique captured shell packet required')
    packet=packets[0];source=packet['native_join']['source'];receipts=[];origins=[]
    for row in captured:
        record=row['record'];old=record['receipt'];part=source['parts'][row['ordinal']]
        count=part['counts'][0];start=part['point_offset'];new_ids=source['points'][start:start+count]
        if len(new_ids)!=len(record['ordinal_to_semantic_id']):raise ValueError('Ear join allocation count changed')
        translated={'point_ids':new_ids,'faces':record['source']['faces']}
        receipt=bind_ear_attachment(old['evaluated_source'],translated,
            **{k:old[k] for k in ('side','root_source_indices','ear_rows','lower_bridge','upper_bridge')})
        receipts.append(receipt)
        origins.append({'semantic_ids':record['ordinal_to_semantic_id'],'joined_ids':new_ids,
                        'allocation_receipt':old,'evaluation_binding':record['evaluation_binding'],'prejoin_fields':row['fields'],'part_ordinal':row['ordinal']})
        record['join_bound']=True
        EARS.pop(row['registry_key'])
    value={'ears':receipts,'origins':origins,'join_source_sha256':packet['native_join']['source_sha256'],
           'current_role_admission':False,'solver_ready':False}
    value['sha256']=digest(value);packet['ear_join']=value
    return packets


def combined_targets(packet):
    for name in ('ear_join','oral_join','native_weld'):_sealed(packet[name])
    join=packet['native_join']
    if any(packet[name]['join_source_sha256']!=join['source_sha256'] for name in ('ear_join','oral_join')):
        raise ValueError('Attachment receipts belong to different native joins')
    return compose_attachment_targets(packet['oral_join']['boundary'],packet['ear_join']['ears'],join,packet['native_weld'])


def verify_combined_oral_transfer(obj,packets,snapshot,restore):
    if len(packets)!=1:raise ValueError('Unique combined shell packet required')
    packet=packets[0];targets=combined_targets(packet);captured=packet['oral_join']
    by_skin={r['skin_point_id']:r for r in targets['rows'] if r['component']=='oral'}
    oral_targets={'rows':[{**r,'target_index':by_skin[r['skin_point_id']]['target_index']} for r in captured['boundary']['pairs']]}
    before=fields(obj)
    if digest(before)!=digest(packet['native_weld']['after_native_fields']):
        raise ValueError('Unreceipted mutation after native weld')
    expected,edges=expected_transfer(before,captured,oral_targets)
    report=restore(obj,snapshot);after=fields(obj)
    if digest(after)!=digest(expected):raise ValueError('Original oral effect differs from identity-derived expectation')
    value={'targets':targets,'before':before,'after':after,'edges':edges,
           'current_role_admission':False,'solver_ready':False}
    value['sha256']=digest(value);packet['combined_oral_transfer']=value
    return report


def expected_membership(before,skin_fields,skin_ids,targets):
    group='Continuous facial shell';source_groups={name:i for i,name in skin_fields['groups']}
    target_groups={name:i for i,name in before['groups']}
    if group not in source_groups or group not in target_groups:raise ValueError('Missing explicit skin selection group')
    index={pid:i for i,pid in enumerate(skin_ids)};expected=deepcopy(before)
    for row in targets['rows']:
        if not row['component'].startswith('ear:'):continue
        if row['skin_point_id'] not in index:raise ValueError('Ear donor is not authored shell')
        weights=dict(skin_fields['vertices'][index[row['skin_point_id']]][3])
        if weights.get(source_groups[group])!=1.:raise ValueError('Authored ear boundary membership is not unit weight')
        target=expected['vertices'][row['target_index']][3];gid=target_groups[group]
        found=False
        for entry in target:
            if entry[0]==gid:entry[1]=1.;found=True
        if not found:target.append([gid,1.]);target.sort(key=lambda entry:entry[0])
    return expected


def verify_ear_membership(obj,packets,snapshot,restore):
    if len(packets)!=1:raise ValueError('Unique shell packet required')
    packet=packets[0];previous=packet['combined_oral_transfer'];_sealed(previous)
    targets=combined_targets(packet);before=fields(obj)
    if digest(before)!=digest(previous['after']):raise ValueError('Unreceipted change before ear ownership')
    oral=packet['oral_join'];expected=expected_membership(before,oral['skin_fields'],oral['skin_point_ids'],targets)
    report=restore(obj,snapshot);after=fields(obj)
    if digest(after)!=digest(expected):raise ValueError('Original ear ownership differs from identity-derived expectation')
    value={'before':before,'after':after,'targets':targets,'current_role_admission':False,'solver_ready':False}
    value['sha256']=digest(value);packet['ear_membership']=value
    return report
