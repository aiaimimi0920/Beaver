"""Pure discrete guide bending objective; not a geometric acceptance guard.
All supplied path vertices participate. Supply actual full joined mirrored paths
when the midline junction must participate; endpoints have no fabricated tangent.
"""
import numpy as np

def forehead_polyline_fairing(points, frozen_length, variation_weight=0.):
    """Return dimensionless energy, world-coordinate gradient, diagnostics.

    Raw bending B=sum 2*||t_next-t_prev||^2/(l_prev+l_next).
    E=L_frozen*B is scale invariant (equivalently express arc length in frozen
    path-length units). Optional V=L_frozen^3*sum||k_next-k_prev||^2/l_shared,
    k=2*(t_next-t_prev)/(l_prev+l_next). E+variation_weight*V is an objective,
    never a new style/shape threshold. L_frozen is constant, not differentiated.
    Coordinates and frozen_length use the SAME units; gradient is per that unit.
    Degenerate edges are rejected, not silently clipped or dropped.
    """
    p=np.asarray(points,float);L=float(frozen_length);w=float(variation_weight)
    if p.ndim!=2 or p.shape[1]!=3 or len(p)<2 or not np.isfinite(p).all():raise ValueError('Invalid actual guide polyline')
    if not np.isfinite(L) or L<=0 or not np.isfinite(w) or w<0:raise ValueError('Invalid frozen length or objective weight')
    e=np.diff(p,axis=0);l=np.linalg.norm(e,axis=1)
    if np.any(l<=0):raise ValueError('Zero-length actual guide edge')
    t=e/l[:,None];s=l[:-1]+l[1:];d=np.diff(t,axis=0);k=2*d/s[:,None];raw=float(np.sum(2*np.sum(d*d,axis=1)/s));gk=L*s[:,None]*k;gs=.5*L*np.sum(k*k,axis=1);gl=np.zeros_like(l);variation=0.
    if len(k)>1 and w:
        dk=np.diff(k,axis=0);h=l[1:-1];variation=float(L**3*np.sum(np.sum(dk*dk,axis=1)/h));z=2*w*L**3*dk/h[:,None];gk[:-1]-=z;gk[1:]+=z;gl[1:-1]-=w*L**3*np.sum(dk*dk,axis=1)/(h*h)
    gd=2*gk/s[:,None];gs-=np.sum(gk*k,axis=1)/s;gt=np.zeros_like(t);gt[:-1]-=gd;gt[1:]+=gd;gl[:-1]+=gs;gl[1:]+=gs
    ge=(gt-t*np.sum(gt*t,axis=1)[:,None])/l[:,None]+gl[:,None]*t;gradient=np.zeros_like(p);gradient[:-1]-=ge;gradient[1:]+=ge
    return L*raw+w*variation,gradient,{'raw_bending':raw,'dimensionless_bending':L*raw,'dimensionless_variation':variation,'frozen_length':L,'edge_count':len(l),'interior_vertex_count':len(k)}

def forehead_guides_fairing(points,paths,frozen_lengths,variation_weight=0.,role_weights=None):
    """Sum every supplied path, retaining all tails and actual endpoint positions."""
    p=np.asarray(points,float);gradient=np.zeros_like(p);energy=0.;details={};weights={}if role_weights is None else role_weights
    if set(paths)!=set(frozen_lengths):raise ValueError('Frozen lengths must cover exactly the supplied whole paths')
    for role,path in paths.items():
        ids=np.asarray(path,int)
        if len(set(ids.tolist()))!=len(ids):raise ValueError('Repeated vertex in guide path')
        weight=float(weights.get(role,1.))
        if not np.isfinite(weight)or weight<0:raise ValueError('Invalid role objective weight')
        value,g,details[role]=forehead_polyline_fairing(p[ids],frozen_lengths[role],variation_weight)
        energy+=weight*value;np.add.at(gradient,ids,weight*g)
    return energy,gradient,details

def forehead_fairing_parameter_gradient(world_gradient,variable_ids,world_differential,donor_boundary=None):
    """Chain world gradient through sampler Jacobian; flag boundary columns.

    world_differential has shape(n_variables,3,2), typically sample_differential
    J/1000 for world-metres vs solver-millimetres. At flagged donor boundaries
    caller must FD the unchanged complete objective; do not treat the selected
    one-sided donor derivative as an exact central derivative. Invalid sampler
    results must remain invalid; objective cannot waive feasibility guards.
    """
    ids=np.asarray(variable_ids,int);g=np.asarray(world_gradient,float)[ids];J=np.asarray(world_differential,float)
    if J.shape!=(len(ids),3,2):raise ValueError('Invalid source differential shape')
    flags=np.zeros(len(ids),bool)if donor_boundary is None else np.asarray(donor_boundary,bool)
    if flags.shape!=(len(ids),):raise ValueError('Invalid donor-boundary flags')
    return np.einsum('ni,nia->na',g,J).reshape(-1),np.repeat(flags,2)
