"""Source boundary sampling; retains every existing feature vertex and face."""
from math import sqrt


def refined_outline(vertices, faces, chain, maximum_deviation=.002, protected=()):
    from math import isfinite, sqrt
    if not isfinite(maximum_deviation) or maximum_deviation < 0: raise ValueError("Invalid outline displacement budget")
    if any(len(p)!=3 or any(not isfinite(c) for c in p) for p in vertices): raise ValueError("Nonfinite source outline")
    def add(a,b):return tuple(x+y for x,y in zip(a,b))
    def sub(a,b):return tuple(x-y for x,y in zip(a,b))
    def mul(a,s):return tuple(x*s for x in a)
    def distance(a,b):return sqrt(sum((x-y)**2 for x,y in zip(a,b)))
    def lerp(a,b,t):return add(mul(a,1-t),mul(b,t))
    def cubic(p0,p1,p2,p3):
        times=[0.]
        for a,b in zip((p0,p1,p2),(p1,p2,p3)):
            times.append(times[-1]+sqrt(max(distance(a,b),1e-12)))
        a,b,c,d=times;t=(b+c)/2
        q0=lerp(p0,p1,(t-a)/(b-a));q1=lerp(p1,p2,(t-b)/(c-b));q2=lerp(p2,p3,(t-c)/(d-c))
        r0=lerp(q0,q1,(t-a)/(c-a));r1=lerp(q1,q2,(t-b)/(d-b))
        return lerp(r0,r1,(t-b)/(c-b))
    if len(chain)<3 or len(set(chain))!=len(chain):raise ValueError('Invalid semantic outline chain')
    counts={}
    for face in faces:
        for a,b in zip(face,face[1:]+face[:1]):
            edge=tuple(sorted((a,b)));counts[edge]=counts.get(edge,0)+1
    result=list(vertices);mids={};maximum=0.
    for i,(a,b) in enumerate(zip(chain,chain[1:])):
        edge=tuple(sorted((a,b)))
        if counts.get(edge)!=1:raise ValueError('Outline sampling requires an actual boundary edge')
        if a in protected or b in protected:continue
        p1,p2=vertices[a],vertices[b]
        p0=vertices[chain[i-1]] if i else sub(mul(p1,2),p2)
        p3=vertices[chain[i+2]] if i+2<len(chain) else sub(mul(p2,2),p1)
        midpoint=lerp(p1,p2,.5);target=cubic(p0,p1,p2,p3)
        deviation=distance(midpoint,target)
        if deviation>maximum_deviation:target=lerp(midpoint,target,maximum_deviation/deviation)
        maximum=max(maximum,distance(midpoint,target))
        mids[edge]=len(result);result.append(target)
    output=[]
    for face in faces:
        new=[]
        for a,b in zip(face,face[1:]+face[:1]):
            new.append(a);index=mids.get(tuple(sorted((a,b))))
            if index is not None:new.append(index)
        output.append(tuple(new))
    return result,output,{'stage':'source outline sampling','new_vertices':len(mids),'source_faces_unchanged':len(output)==len(faces),'existing_vertices_unchanged':result[:len(vertices)]==list(vertices),'maximum_curve_deviation_m':maximum}
