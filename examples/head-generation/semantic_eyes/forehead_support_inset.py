from forehead_pattern_materialize import forehead_pattern_plain_records
import copy
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from forehead_star_topology import forehead_edge_counts,forehead_guide_corners

def forehead_prospect_face_inset(proposal,source,face_id,inset_fraction=.25):
 proposal = forehead_pattern_plain_records(proposal)
 source = forehead_pattern_plain_records(source)
 if not 0 < inset_fraction < .5:raise ValueError("Inset fraction must lie strictly between zero and one half")
 selected=[int(face_id)]
 if len(proposal["faces"][face_id])!=4:raise ValueError("Inset requires a quad")
 saved={"proposal":proposal}
 p=copy.deepcopy(saved['proposal']);t=p['topology'];old=np.asarray(source['vertices']);od=np.asarray(source['open_delta']);chart=np.asarray(p['source_chart']);current=np.vstack((chart,np.zeros((len(p['vertices'])-len(old),3))))
 for o in(p,t):o['lineage']={int(i):row for i,row in o['lineage'].items()}
 for i,row in p['lineage'].items():current[i]=np.asarray(row['weights'])@chart[row['donors']]
 surf=AuthorQuadSurface(chart,p['source_faces'],[1]*len(p['source_faces']));surf.vertices=old.copy();parents={tuple(row['edge']):row['parent']for row in t['edge_lineage']};initial_faces=len(p['faces']);newids=[]
 for fi in selected:
  q=p['faces'][fi];inner=[]
  for u,v in ((inset_fraction,inset_fraction),(1-inset_fraction,inset_fraction),(1-inset_fraction,1-inset_fraction),(inset_fraction,1-inset_fraction)):
   w=np.array([(1-u)*(1-v),u*(1-v),u*v,(1-u)*v]);xy=w@current[q];donors,weights,point=surf.lineage(xy[0],xy[2]);point=np.asarray(point,np.float32).astype(float).tolist();idx=len(p['vertices']);row={'donors':list(donors),'weights':list(weights)}
   p['vertices'].append(point);p['open_delta'].append((np.asarray(weights)@od[list(donors)]).tolist());p['lineage'][idx]=row;t['vertices'].append(point);t['chart'].append(xy.tolist());t['lineage'][idx]=copy.deepcopy(row);inner.append(idx);newids.append(idx)
  children=[inner]+[[q[k],q[(k+1)%4],inner[(k+1)%4],inner[k]]for k in range(4)]
  p['faces'][fi]=children[0];p['faces'].extend(children[1:]);t['face_owners'].extend([t['face_owners'][fi]]*4)
  for child in children:
   for e in zip(child,child[1:]+child[:1]):parents.setdefault(tuple(sorted(e)),None)
 t['faces']=copy.deepcopy(p['faces']);t['edge_lineage']=[{'edge':list(e),'parent':par}for e,par in parents.items()]
 p['affected_new']=sorted(set(p['affected_new'])|set(selected)|set(range(initial_faces,len(p['faces']))));p['affected_old']=sorted(set(p['affected_old'])|{t['face_owners'][fi]for fi in selected});p['moving']=sorted(set(p['moving'])|set(newids));p['support']=sorted(set(p['support'])|set(newids));t['retiled_old_faces']=sorted(set(t['retiled_old_faces'])|{t['face_owners'][fi]for fi in selected});t['retiled_new_faces']=[j for j,q in enumerate(p['faces'])if q!=source['faces'][t['face_owners'][j]]]
 before=forehead_edge_counts(saved['proposal']['faces']);after=forehead_edge_counts(p['faces']);assert set(after)==set(parents) and max(after.values())<=2
 assert {e for e,n in before.items()if n==1}=={e for e,n in after.items()if n==1}
 assert len(saved['proposal']['vertices'])-len(before)+len(saved['proposal']['faces'])==len(p['vertices'])-len(after)+len(p['faces'])
 assert all(not forehead_guide_corners(p['faces'],path)for path in p['paths'].values())
 t.setdefault('interior_support_insets',[]).append({'source_faces':selected,'new_supports':newids,'guide_station_counts_and_positions_unchanged':True})
 p['support_operator_pending_birth_admission']=True
 return p
