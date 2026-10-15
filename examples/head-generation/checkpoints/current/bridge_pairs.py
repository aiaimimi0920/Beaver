"""Select a semantic left/right bridge pair independently of which side backtracks first."""
def paired_bridge_indices(points, candidates, initial, tolerance=0.00002):
    assert len(initial) in (1,2), ('Expected one or two limiting bridges before symmetry pairing', initial, candidates)
    result=list(initial)
    if len(result)==1:
        seed=candidates[result[0]]
        mirrored=sorted([(-points[i][0],points[i][1],points[i][2]) for i in seed])
        matches=[]
        for index,ids in candidates.items():
            if index==result[0]:continue
            current=sorted([tuple(points[i]) for i in ids])
            error=max(sum((a-b)**2 for a,b in zip(p,q))**.5 for p,q in zip(mirrored,current))
            if error<tolerance:matches.append(index)
        assert len(matches)==1, ('Missing or ambiguous mirrored bridge',matches)
        result+=matches
    assert len(set(result))==2
    a,b=[candidates[i] for i in result]
    mirrored=sorted([(-points[i][0],points[i][1],points[i][2]) for i in a])
    other=sorted([tuple(points[i]) for i in b])
    assert max(sum((p-q)**2 for p,q in zip(u,v))**.5 for u,v in zip(mirrored,other))<tolerance
    return result
