"""Coupled terminal-strip fairing on the immutable authored quad surface."""
import numpy as np
from scipy.optimize import minimize
from nasal_source_geometry import AuthorQuadSurface
from orbital_support_rule import orbital_turns, orbital_quad_quality
from forehead_flow_rule import (forehead_chart, forehead_convex_chart,
                                forehead_no_chart_crossings)


def refine_forehead_terminal(vertices, faces, paths, spec):
    """Resolve the green earward turn without squeezing its two upper rows.

    The three terminal strips are bound by semantic path positions, never saved
    mesh indices. Original boundary and ear vertices are not optimization DOFs.
    Density, curvature and cell guards are retained after float32 quantization.
    Native expression and frozen-surface envelope checks remain separate gates.
    """
    old = np.asarray(vertices, float)
    initial = np.asarray(spec['vertices'], float)
    paths = [list(map(int, p)) for p in paths]
    if len(paths) != 5 or any(len(p) < 15 for p in paths[:3]):
        raise ValueError('Missing terminal five-flow roles')
    # Keep upstream green stations fixed: their blue-green export chords
    # already satisfy the frozen-surface envelope and must not be stretched.
    ids = sorted(set(paths[2][12:14]) | {i for path in paths[:2] for i in path[10:14]})
    if len(ids) != 10 or not set(ids).issubset(spec['lineage']):
        raise ValueError('Terminal strip contains a fixed or overlapping role')
    quads = np.asarray([faces[i] for i in spec['affected_faces']], int)
    chart = forehead_chart(old)
    surface = AuthorQuadSurface(chart, quads, [1]*len(quads))
    surface.vertices = old.copy()
    original_cross = forehead_convex_chart(chart, quads)
    angles, ratios = orbital_quad_quality(old, quads)
    triples = [np.asarray(list(zip(p, p[1:], p[2:])), int) for p in paths]
    before = [orbital_turns(old, t) for t in triples]
    edges = np.asarray([(a,b) for p in paths[:3] for a,b in zip(p,p[1:])])
    lengths = np.linalg.norm(old[edges[:,1]]-old[edges[:,0]], axis=1)*1000
    start = forehead_chart(initial)[ids][:,[0,2]].ravel()*1000
    cache = {}

    def evaluate(values):
        key = values.tobytes()
        if key in cache:
            return cache[key]
        points = initial.copy()
        donors = {}
        try:
            for index, xy in zip(ids, values.reshape(-1,2)/1000):
                source, weights, point = surface.lineage(*xy)
                points[index] = point
                donors[index] = {'donors': list(source), 'weights': list(weights)}
        except ValueError:
            return None
        new_angles, new_ratios = orbital_quad_quality(points, quads)
        turns = [orbital_turns(points, t) for t in triples]
        q = forehead_chart(points)[quads][:,:,[0,2]]
        edge = np.roll(q,-1,axis=1)-q
        cross = edge[:,:,0]*np.roll(edge[:,:,1],-1,axis=1)
        cross -= edge[:,:,1]*np.roll(edge[:,:,0],-1,axis=1)
        result = points, donors, new_angles, new_ratios, turns, cross
        cache.clear()
        cache[key] = result
        return result

    def constraints(values):
        result = evaluate(values)
        if result is None:
            return np.full(len(quads)*7+5+len(edges)*2, -1e3)
        points, _, a, ratio, turns, cross = result
        distance = np.linalg.norm(points[edges[:,1]]-points[edges[:,0]], axis=1)*1000
        return np.r_[a.min(axis=1)-angles.min()-.002, angles.max()-a.max(axis=1)-.002,
                     ratios.max()-.001-ratio, cross.ravel()*np.sign(original_cross).ravel()*1e6,
                     [(np.mean(b*b)*1.01-np.mean(t*t)-.05)/100 for b,t in zip(before,turns)],
                     distance-lengths*.65-.001, lengths*1.35-distance-.001]

    def objective(values):
        result = evaluate(values)
        if result is None:
            return 1e8+np.sum((values-start)**2)
        turns = result[4]
        return (np.mean(turns[2][-8:]**2)+.05*np.sum((values-start)**2)
                +.01*sum(np.sum(t*t) for t in turns[:2]))

    solved = minimize(objective, start, method='SLSQP',
                      bounds=[(x-12,x+12) for x in start],
                      constraints=[{'type':'ineq','fun':constraints}],
                      options={'maxiter':200,'ftol':1e-7})
    if not solved.success or constraints(solved.x).min() < -1e-5:
        raise ValueError('No feasible coupled forehead terminal solution: '+str(solved.message))
    points, donors, *_ = evaluate(solved.x)
    points = points.astype(np.float32).astype(float)
    after_chart = forehead_chart(points)
    cross = forehead_convex_chart(after_chart, quads)
    if np.any(cross*original_cross <= 0):
        raise ValueError('Quantized terminal chart reversed')
    forehead_no_chart_crossings(after_chart, [list(q) for q in quads])
    a, ratio = orbital_quad_quality(points, quads)
    turns = [orbital_turns(points,t) for t in triples]
    distance = np.linalg.norm(points[edges[:,1]]-points[edges[:,0]], axis=1)*1000
    if (a.min() < angles.min()-1e-5 or a.max() > angles.max()+1e-5
            or ratio.max() > ratios.max()+1e-5 or max(turns[2]) > 45
            or np.any(distance < lengths*.65-1e-4)
            or np.any(distance > lengths*1.35+1e-4)
            or any(np.mean(t*t) > np.mean(b*b)*1.01+1e-3 for b,t in zip(before,turns))):
        raise ValueError('Quantized terminal guard failed: '+repr({
            'angles': [float(a.min()),float(a.max())], 'ratio':float(ratio.max()),
            'green':float(max(turns[2])),
            'length_residual':float(min(np.min(distance-lengths*.65),np.min(lengths*1.35-distance))),
            'energy_residual':float(min(np.mean(b*b)*1.01-np.mean(t*t) for b,t in zip(before,turns)))}))
    q = points[quads]
    for i,j,k,l in ((0,1,2,3),(1,2,3,0)):
        left = np.cross(q[:,j]-q[:,i],q[:,k]-q[:,i])
        right = np.cross(q[:,k]-q[:,i],q[:,l]-q[:,i])
        norm = np.linalg.norm(left,axis=1)*np.linalg.norm(right,axis=1)
        if np.any(norm <= 1e-18) or np.any(np.sum(left*right,axis=1)/norm <= .1):
            raise ValueError('Terminal automatic diagonal folds')
    spec['vertices'] = points
    spec['lineage'].update(donors)
    spec['metrics'].update(
        peak_turn_after=[float(max(t)) for t in turns],
        quality_after=[float(a.min()),float(a.max()),float(ratio.max())],
        maximum_move_mm=float(np.linalg.norm(points-old,axis=1).max()*1000),
        aspect_ratio_worsened_cells=int(np.sum(ratio>ratios+1e-5)),
        minimum_angle_worsened_over5deg=int(np.sum(a.min(axis=1)<angles.min(axis=1)-5)),
        cells_below20deg=int(np.sum(a.min(axis=1)<20)),
        terminal_solver={'iterations':int(solved.nit),'variables':len(start),
                         'green_maximum_turn_limit_degrees':45,
                         'edge_length_relative_bounds':[.65,1.35],
                         'pre_quantization_constraint_minimum':float(constraints(solved.x).min()),
                         'quantized_quality_and_spacing_pass':True,
                         'dense_shape_guard_required_separately':True})
    return spec
