"""Geometry-error witnesses to bounded operator contexts; no acceptance or masking."""
def forehead_local_shape_witness(proposal,localization,stop_mm=.98,maximum_faces=2):
    """Select highest measured face and co-edge neighbor among other high errors.
    Caller must remeasure WHOLE patch after each admitted step; this never claims
    ignored faces/poses pass. Source owners plus explicit reverse worst are used.
    Synthetic/native IDs are input data, never anatomical constants.
    """
    if not 0<stop_mm<1 or not 1<=maximum_faces<=4:raise ValueError('Witness selection limit invalid')
    rows=sorted(localization['faces'],key=lambda z:-max(z['forward_mm'],z['reverse_mm']))
    if not rows or max(rows[0]['forward_mm'],rows[0]['reverse_mm'])<=stop_mm:return None
    primary=int(rows[0]['face']);selected=[primary];quad=proposal['faces'][primary];edges={tuple(sorted(e))for e in zip(quad,quad[1:]+quad[:1])}
    for row in rows[1:]:
        if len(selected)>=maximum_faces:break
        if max(row['forward_mm'],row['reverse_mm'])<=stop_mm:continue
        face=int(row['face']);q=proposal['faces'][face]
        if edges&{tuple(sorted(e))for e in zip(q,q[1:]+q[:1])}:selected.append(face)
    sources={proposal['topology']['face_owners'][i]for i in selected};reverse=localization.get('reverse_worst')
    if reverse and reverse['target_face']in selected:sources.add(int(reverse['source_face']))
    return {'target_face_ids':selected,'source_face_ids':sorted(sources),'evidence_pose':localization.get('pose'),'evidence_subdivisions':localization.get('subdivisions'),'scope':'Local solve target; global/full-pose rescan still mandatory'}
