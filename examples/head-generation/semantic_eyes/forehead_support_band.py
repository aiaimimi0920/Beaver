"""Prospective neutral edge trisection bands; unsupported patterns fail closed."""
from forehead_pattern_materialize import forehead_pattern_plain_records
import copy
import numpy as np
from nasal_source_geometry import AuthorQuadSurface
from forehead_star_topology import forehead_edge_counts,forehead_guide_corners

def forehead_prospect_trisection_band(proposal,source,selected_edges,neutral_parent_edges=(),inner_height=.5,allow_nonneutral_original_endpoint_chain=False,nonneutral_locus=None):
 proposal = forehead_pattern_plain_records(proposal)
 source = forehead_pattern_plain_records(source)
 selected_edges = forehead_pattern_plain_records(selected_edges)
 neutral_parent_edges = forehead_pattern_plain_records(neutral_parent_edges)
 if not 0<inner_height<1:raise ValueError('Interior transition height must lie strictly inside face')
 if not selected_edges or len(selected_edges)>8:raise ValueError('Trisection edge budget exceeded or empty')
 if nonneutral_locus not in (None,"original_author","accepted_current_chain"):raise ValueError("Unknown chain locus")
 p=copy.deepcopy(proposal);t=p['topology'];old=np.asarray(source['vertices']);od=np.asarray(source['open_delta']);sc=np.asarray(p['source_chart']);N=len(p['vertices']);F=len(p['faces']);faces=p['faces'];selected={tuple(sorted(e))for e in selected_edges};before=forehead_edge_counts(faces);parents={tuple(z['edge']):z['parent']for z in t['edge_lineage']};neutral={tuple(sorted(e))for e in neutral_parent_edges};guideedges={tuple(sorted(e))for path in p['paths'].values()for e in zip(path,path[1:])};patterns={}
 has_nonneutral=any(parents.get(e)is not None and tuple(sorted(parents[e]))not in neutral for e in selected)
 if has_nonneutral and nonneutral_locus is None:raise ValueError('Explicit nonneutral chain locus required')
 if nonneutral_locus is None:nonneutral_locus='accepted_current_chain'
 for e in selected:
  if before.get(e)!=2 or e in guideedges:raise ValueError('Only interior non-guide edges may be trisected')
  if parents[e]is not None and tuple(sorted(parents[e]))not in neutral and (not allow_nonneutral_original_endpoint_chain or tuple(sorted(parents[e]))!=e or max(e)>=len(old)):raise ValueError('Nonneutral edge requires explicit original-endpoint locus and fixed births')
 for fi,q in enumerate(faces):
  hits=[k for k in range(4)if tuple(sorted((q[k],q[(k+1)%4])))in selected]if len(q)==4 else[]
  if not hits:continue
  if len(hits)>2 or(len(hits)==2 and(hits[1]-hits[0])%4!=2):raise ValueError('Unsupported adjacent/multiple trisection pattern')
  patterns[fi]=hits
 if {e for fi in patterns for e in selected if e[0]in faces[fi]and e[1]in faces[fi]}!=selected:raise ValueError('Selected edge not covered by quadrilateral patterns')
 if len(patterns)>12:raise ValueError('Trisection affected-face budget exceeded')
 for o in(p,t):o['lineage']={int(i):z for i,z in o['lineage'].items()}
 params=np.vstack((sc,np.zeros((N-len(sc),3))))
 for i,z in p['lineage'].items():params[i]=np.asarray(z['weights'])@sc[z['donors']]
 surf=AuthorQuadSurface(sc,p['source_faces'],[1]*len(p['source_faces']));surf.vertices=old.copy();births=[]
 def add(xy,fixed_row=None):
  if fixed_row is None:don,w,pt=surf.lineage(xy[0],xy[2])
  else:don,w=fixed_row;pt=np.asarray(w)@old[list(don)]
  i=len(p['vertices']);pt=np.asarray(pt,np.float32).astype(float).tolist();row={'donors':list(don),'weights':list(w)};p['vertices'].append(pt);p['open_delta'].append(np.asarray(np.asarray(w)@od[list(don)],np.float32).astype(float).tolist());p['lineage'][i]=row;t['lineage'][i]=copy.deepcopy(row);t['vertices'].append(pt);t['chart'].append(xy.tolist());births.append(i);return i
 edgepoints={}
 for a,b in sorted(selected):
  par=parents[(a,b)]
  if par is not None and tuple(sorted(par))not in neutral and nonneutral_locus=="original_author":
   edgepoints[(a,b)]=(add((2*sc[a]+sc[b])/3,([a,b],[2/3,1/3])),add((sc[a]+2*sc[b])/3,([a,b],[1/3,2/3])))
  else:edgepoints[(a,b)]=(add((2*params[a]+params[b])/3),add((params[a]+2*params[b])/3))
 def pair(a,b):
  u,v=edgepoints[tuple(sorted((a,b)))];return(u,v)if a<b else(v,u)
 details=[]
 for fi,hits in patterns.items():
  q=faces[fi];k=hits[0];a,b,c,d=q[k:]+q[:k];u,v=pair(a,b)
  if len(hits)==2:
   r,s=pair(c,d);children=[[a,u,s,d],[u,v,r,s],[v,b,c,r]];kind='three_strip_quads'
  else:
   def bil(x,y):return np.array([(1-x)*(1-y),x*(1-y),x*y,(1-x)*y])@params[[a,b,c,d]]
   A=add(bil(1/3,inner_height));B=add(bil(2/3,inner_height));children=[[d,a,u,A],[v,b,c,B],[u,v,B,A],[B,c,d,A]];kind='four_quad_transition'
  faces[fi]=children[0];faces.extend(children[1:]);t['face_owners'].extend([t['face_owners'][fi]]*(len(children)-1));details.append({'face':fi,'kind':kind,'children':children})
  for child in children:
   for e in zip(child,child[1:]+child[:1]):parents.setdefault(tuple(sorted(e)),None)
 for e,(u,v)in edgepoints.items():
  par=parents.pop(e);a,b=e
  for aa,bb in[(a,u),(u,v),(v,b)]:parents[tuple(sorted((aa,bb)))]=par
 t['faces']=copy.deepcopy(faces);t['edge_lineage']=[{'edge':list(e),'parent':par}for e,par in parents.items()];p['affected_new']=sorted(set(p['affected_new'])|set(patterns)|set(range(F,len(faces))));p['affected_old']=sorted(set(p['affected_old'])|{t['face_owners'][fi]for fi in patterns});p['moving']=sorted(set(p['moving'])|set(births));p['support']=sorted(set(p['support'])|set(births));t['retiled_new_faces']=sorted(set(t['retiled_new_faces'])|set(patterns)|set(range(F,len(faces))));t['retiled_old_faces']=sorted(set(t['retiled_old_faces'])|{t['face_owners'][fi]for fi in patterns});after=forehead_edge_counts(faces)
 assert set(after)==set(parents)and max(after.values())<=2 and N-len(before)+F==len(p['vertices'])-len(after)+len(faces)
 assert {e for e,n in before.items()if n==1}=={e for e,n in after.items()if n==1}
 assert all(not forehead_guide_corners(faces,path)for path in p['paths'].values())
 t.setdefault('prospective_trisection_bands',[]).append({'nonneutral_locus':nonneutral_locus,'frozen_current_chain_endpoints':{str(i):params[i].tolist()for e in selected for i in e},'selected_edges':[list(e)for e in sorted(selected)],'edge_points':[{'edge':list(e),'points':list(z)}for e,z in edgepoints.items()],'births':births,'face_patterns':details})
 p['support_operator_pending_birth_admission']=True
 return p
