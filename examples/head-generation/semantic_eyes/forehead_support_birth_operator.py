"""Prospective support-birth adapter; preserves all existing baselines and guides."""
from forehead_pattern_materialize import forehead_pattern_plain_records
from forehead_support_contract import forehead_support_admission,forehead_support_held_nodes,forehead_support_topology_contract
import copy
import numpy as np
from forehead_birth_geometry import forehead_birth_solve
from forehead_surface_batch import make_forehead_surface_batch
from forehead_spacing import forehead_frozen_field
from forehead_flow_rule import forehead_chart
from nasal_source_geometry import AuthorQuadSurface

def forehead_solve_support_births(r, base, p, f, *, wall_seconds=120., maximum_births=32, maximum_old_supports=64, regularization=.00002):
    r = forehead_pattern_plain_records(r)
    base = forehead_pattern_plain_records(base)
    p = forehead_pattern_plain_records(p)
    f = forehead_pattern_plain_records(f)
    if not p.get("support_operator_pending_birth_admission"):raise ValueError("Prospective birth transaction required")
    if not 0<wall_seconds<=120 or len(p["vertices"])-len(base["vertices"])>maximum_births:raise ValueError("Birth operator limits exceeded")
    forehead_support_topology_contract(base,p)
    old=np.asarray(r['vertices']);delta=np.asarray(r['open_delta']);v=np.asarray(p['vertices']);N=len(base['vertices']);chart=np.asarray(p['source_chart']);lin={int(k):z for k,z in p['lineage'].items()};params=np.vstack((chart,np.zeros((len(v)-len(old),3))))
    for i,z in lin.items():params[i]=np.asarray(z['weights'])@chart[z['donors']]
    neutral={tuple(e)for e in f['neutral_parent_edges']};fixed=set();held=forehead_support_held_nodes(p)
    for row in p['topology']['edge_lineage']:
     if row['parent'] is not None and tuple(sorted(row['parent'])) not in neutral:
      for i in row['edge']:
       if i>=N:fixed.add(i)
       elif i>=len(old):held.add(i)
    new=set(range(N,len(v)));near={i for q in p['faces']if new.intersection(q)for i in q}
    near|={i for q in p['faces']if near.intersection(q)for i in q}
    movable={i for i in near if i<N and i in p['support'] and i not in held}
    movable-={i for q in p['faces']if len(q)!=4 for i in q}
    guarded=sorted(set(p['affected_new'])|{j for j,q in enumerate(p['faces'])if (movable|new).intersection(q)})
    vb=make_forehead_surface_batch(chart,old,np.asarray(p['source_faces']));db=make_forehead_surface_batch(chart,delta,np.asarray(p['source_faces']));surface=AuthorQuadSurface(chart,p['source_faces'],[1]*len(p['source_faces']));surface.vertices=old.copy()
    def sample_row(x):
     try:d,w,_=surface.lineage(*x)
     except ValueError:return None
     return {'donors':list(d),'weights':list(w)}
    frozen=forehead_frozen_field(delta,N,{int(i):z for i,z in base['topology']['lineage'].items()})
    discrete=f['point_discrete'];reference={}
    for key,values in discrete.items():
     reference[key]=copy.deepcopy(values)
     for i in range(len(old),N):
      z=base['topology']['lineage'].get(i, base['topology']['lineage'].get(str(i)));active=[d for d,w in zip(z['donors'],z['weights'])if w>0];assert all(values[d]==values[active[0]]for d in active);reference[key].append(values[active[0]])
    if len(movable)>maximum_old_supports:raise ValueError('Birth support halo exceeds limit')
    result=forehead_birth_solve(vertices=v,faces=p['faces'],paths=p['paths'],parameters=params[:,[0,2]],existing_count=N,frozen_old_fields=frozen,current_fields=p['open_delta'],source_sampler=vb.sample,field_sampler=db.sample,row_sampler=sample_row,source_vertices=old,source_fields=delta,actual_chart=lambda points:forehead_chart(points)[:,[0,2]],movable_old=movable,held_nodes=held,free_births=new-fixed,fixed_births=fixed,fixed_birth_rows={i:lin[i]for i in fixed},guard_face_ids=guarded,source_discrete=discrete,discrete_reference=reference,quality_margin=.003,wall_seconds=wall_seconds,regularization=regularization)
    return result


def forehead_apply_support_births(source, base, prospective, result, field_context, extra_admission):
    source = forehead_pattern_plain_records(source)
    base = forehead_pattern_plain_records(base)
    prospective = forehead_pattern_plain_records(prospective)
    result = forehead_pattern_plain_records(result)
    field_context = forehead_pattern_plain_records(field_context)
    if not result.get('admitted'):
        raise ValueError('Rejected birth geometry has no applicable updates')
    p = copy.deepcopy(prospective)
    count = len(base['vertices'])
    for owner in (p, p['topology']):
        owner['lineage'] = {int(k): row for k, row in owner['lineage'].items()}
    for key, position in result['positions'].items():
        p['vertices'][int(key)] = position
    for key, value in result['current_field_updates'].items():
        p['open_delta'][int(key)] = np.asarray(value, np.float32).tolist()
    for key, row in result['old_current_row_updates'].items():
        p['lineage'][int(key)] = row
    for key, row in result['new_birth_rows'].items():
        index = int(key)
        if index < count:
            raise ValueError('Existing birth baseline cannot be replaced')
        p['lineage'][index] = copy.deepcopy(row)
        p['topology']['lineage'][index] = copy.deepcopy(row)
        p['topology']['vertices'][index] = p['vertices'][index]
        p['topology']['chart'][index] = (np.asarray(row['weights']) @
            np.asarray(p['source_chart'])[row['donors']]).tolist()
    for key, row in base['topology']['lineage'].items():
        if p['topology']['lineage'][int(key)] != row:
            raise ValueError('Existing frozen field row changed')
    guides = sorted({i for path in base['paths'].values() for i in path})
    if not np.array_equal(np.asarray(p['vertices'])[guides], np.asarray(base['vertices'])[guides]):
        raise ValueError('Support birth changed a guide')
    p.pop("support_operator_pending_birth_admission",None)
    neutral={tuple(sorted(e))for e in field_context['neutral_parent_edges']}
    fixed={i for row in prospective['topology']['edge_lineage']if row['parent']is not None and tuple(sorted(row['parent']))not in neutral for i in row['edge']if i>=count}
    prows={int(i):row for i,row in prospective['lineage'].items()}
    for i in fixed:
        if p['lineage'][i]!=prows[i]or p['vertices'][i]!=prospective['vertices'][i]:raise ValueError('Fixed nonneutral chain birth locus changed')
    audit=forehead_support_admission(source,base,p,field_context,extra_admission)
    return {"proposal":p,"local_admitted":True,"fit_complete":False,"audit":audit}
