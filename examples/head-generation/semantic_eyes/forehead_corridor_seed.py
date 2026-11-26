"""Semantic pre-boundary-reroute seed. No saved IDs, files, or case coordinates."""
import copy
import numpy as np
from forehead_flow_bind import bind_forehead_paths
from forehead_retile_roles import forehead_interior_ear_route
from forehead_ear_route import extend_to_fitted_ear
from forehead_star_topology import ForeheadStarTopology, forehead_guide_corners


def forehead_corridor_boundary_targets(builder, functional_protected, vertical_direction=(0,0,1), lateral_direction=(1,0,0)):
    points=np.asarray(builder.points,float);paths=builder.paths
    red,orange,green=[paths[k][-1]for k in ('red','orange','green')]
    boundary={i for e,n in builder.original_edges.items()if n==1 for i in e}
    desired=np.array([np.linalg.norm(points[paths[a][0]]-points[paths[b][0]])for a,b in [('red','orange'),('orange','green')]])
    if desired.min()<=1e-9:raise ValueError('Collapsed semantic seed spacing')
    def cost(a,b):
        gaps=np.array([np.linalg.norm(points[a]-points[b]),np.linalg.norm(points[b]-points[green])]);movement=np.array([np.linalg.norm(points[a]-points[red]),np.linalg.norm(points[b]-points[orange])])
        return float(np.sum((gaps/desired-1)**2)+.01*np.sum((movement/desired)**2))
    z=points@np.asarray(vertical_direction);x=points@np.asarray(lateral_direction)
    candidates=[i for i in boundary if x[i]>0 and z[i]>z[green] and i not in functional_protected]
    ranked=sorted((cost(a,b),a,b)for a in candidates for b in candidates if z[a]>z[b])
    if not ranked:raise ValueError('No semantic boundary targets')
    if cost(red,orange)<=.05 or ranked[0][0]>=cost(red,orange):return {'red':red,'orange':orange}
    if len(ranked)>1 and abs(ranked[0][0]-ranked[1][0])<1e-10:raise ValueError('Ambiguous semantic boundary targets')
    return dict(zip(('red','orange'),ranked[0][1:]))


def forehead_corridor_prepare_seed(vertices,faces,regions,report,features):
    faces=[list(map(int,q))for q in faces];binding=bind_forehead_paths(vertices,faces,regions,report)
    paths={k:list(v)for k,v in binding['positive_paths'].items()};paths['green']=forehead_interior_ear_route(vertices,faces,regions,binding)
    corners={k:forehead_guide_corners(faces,p)for k,p in paths.items()}
    if [len(corners[k])for k in ('red','orange','green','blue','purple')]!=[0,0,1,1,2]:raise ValueError('Unsupported semantic corner recipe')
    upper,lower=corners['purple'];blue=corners['blue'][0];green=corners['green'][0]
    pivots=[upper['vertex'],blue['vertex'],green['vertex'],lower['opposite']]
    if set(pivots)&set(features['protected']):raise ValueError('Protected seed pivot')
    builder=ForeheadStarTopology(vertices,faces,regions,paths)
    centers=builder.refine_star(upper['vertex']);builder.refine_star(centers[upper['face']])
    for pivot in pivots[1:]:builder.refine_star(pivot)
    builder.paths['green']=extend_to_fitted_ear(builder.points,builder.faces,[regions[i]for i in builder.owners],report,builder.paths['green'])
    targets=forehead_corridor_boundary_targets(builder,features['protected'])
    ear=set(binding['ear_root_star']);used={i for p in builder.paths.values()for i in p};matches=[]
    for fi,q in enumerate(faces):
        shared=set(q)&ear
        if targets['orange']not in q or len(shared)!=2 or not(set(q)&set(builder.paths['green'])):continue
        choices=shared-{targets['orange']}-used
        if len(choices)==1:matches.append((fi,next(iter(choices))))
    if len(matches)!=1:raise ValueError('Ambiguous semantic ear bridge: '+str(matches))
    bridge,pivot=matches[0]
    if pivot in features['protected']:raise ValueError('Protected ear bridge pivot')
    # Source-face identity remains stable before this star only if unchanged.
    owners=[i for i,o in enumerate(builder.owners)if o==bridge]
    if len(owners)!=1 or builder.faces[owners[0]]!=faces[bridge]:raise ValueError('Ear bridge already refined')
    centers=builder.refine_star(pivot);builder.refine_star(centers[owners[0]])
    out=builder.finish();boundary={i for e,n in builder.original_edges.items()if n==1 for i in e}
    out['protected']=sorted(set(features['protected'])|boundary|ear|{i for p in out['paths'].values()for i in (p[0],p[-1])})
    out['roles']={'original_binding':binding,'original_corner_roles':corners,'boundary_target_requests':targets,'ear_bridge_source_face':bridge,'ear_bridge_pivot':pivot,'semantic_recipe':'Five corner stars plus unique semantic ear bridge two stars; no legacy boundary reroute'}
    return out


def forehead_corridor_request_terminals(proposal,selection,terminal_requests):
    """Construct explicit prospective route requests, NOT an asserted current rail.
    Only active suffixes of requested roles are replaced. All old mesh vertices,
    edges, lineage, and non-requested guide tails remain untouched. Caller must
    disable unchanged fallback and admit only actual solver/materializer output.
    """
    out=copy.deepcopy(proposal);selection=copy.deepcopy(selection)
    used={v for fi in selection['selected_base_faces']for v in proposal['faces'][fi]}
    for role,target in terminal_requests.items():
        path=list(out['paths'][role]);start=selection['guide_segments'][role][0]
        if target not in used:raise ValueError('Requested terminal outside selected corridor')
        if target in path:
            if path.index(target)<=path.index(start):raise ValueError('Terminal precedes active start')
            path=path[:path.index(target)+1]
        else:path.append(target)
        out['paths'][role]=path;selection['guide_segments'][role]=(start,target)
        d=selection['anchor_details'][role];d['fixed_tail']=[target];d['end_index']=len(path)-1
    out['topology']['paths']=copy.deepcopy(out['paths'])
    out['prospective_terminal_requests']=dict(terminal_requests)
    out['prospective_terminal_previous_endpoints']={role:proposal['paths'][role][-1]for role in terminal_requests}
    return out,selection
