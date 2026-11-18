"""Measured float32 arc margins; unchanged final stage gates remain authoritative."""
import numpy as np


def forehead_arc_quantization_targets(points,station_stage,row_offset,scales,
        ratio_safety=1e-7):
    """Return positive SCALED targets for saved-failing arc rows only.

    Own rows from each station-stage group LAST row. References are the stage's
    immutable millimetre lengths. No proposal/reference/cap mutation. Caller
    retains requests componentwise and still runs original saved admission.
    """
    xyz=np.asarray(points,float);scale=np.asarray(scales,float)
    if xyz.ndim!=2 or xyz.shape[1]!=3 or not np.isfinite(xyz).all():raise ValueError('Invalid world points')
    if scale.ndim!=1 or not len(scale) or not np.isfinite(scale).all()or np.any(scale<=0):raise ValueError('Invalid original row scales')
    if type(row_offset)is not int or row_offset<0:raise ValueError('Invalid station row offset')
    if not np.isfinite(ratio_safety) or ratio_safety<=0:raise ValueError('Invalid positive ratio safety')
    groups=list(station_stage.groups);paths=station_stage.paths
    if len(groups)!=len(paths):raise ValueError('Station group ownership mismatch')
    targets=np.zeros_like(scale);records=[];seen=set();previous=0
    saved=xyz.astype(np.float32).astype(float)
    if not np.isfinite(saved).all():raise ValueError('Nonfinite saved geometry')
    for (name,path),(first,last,nodes) in zip(paths.items(),groups):
        array=np.asarray(path)
        if array.ndim!=1 or len(array)<2 or array.dtype.kind not in 'iu' or len(set(array.tolist()))!=len(array) or np.any(array<0) or np.any(array>=len(xyz)):raise ValueError('Invalid guide path')
        if type(first)is not int or type(last)is not int or first!=previous or last-first!=2*(len(path)-1)+1 or list(nodes)!=list(path):raise ValueError('Invalid station group layout')
        previous=last;row=row_offset+last-1
        if row>=len(scale) or row in seen:raise ValueError('Arc row out of range or duplicate')
        seen.add(row);reference=float(station_stage.reference[name])
        if not np.isfinite(reference) or reference<=0:raise ValueError('Invalid immutable arc reference')
        smooth=float(np.linalg.norm(np.diff(xyz[array],axis=0),axis=1).sum()*1000/reference)
        rounded=float(np.linalg.norm(np.diff(saved[array],axis=0),axis=1).sum()*1000/reference)
        # Matches the stage's existing immutable 1.05 limit, not a new cap.
        if rounded>1.05:
            raw=max(rounded-smooth,0.)+ratio_safety;targets[row]=raw/scale[row]
            records.append({'role':name,'row':row,'smooth_ratio':smooth,'saved_ratio':rounded,'raw_ratio_target':raw,'ratio_safety':float(ratio_safety),'reference_mm':reference})
    if previous!=station_stage.count:raise ValueError('Station total row count mismatch')
    return targets,records
