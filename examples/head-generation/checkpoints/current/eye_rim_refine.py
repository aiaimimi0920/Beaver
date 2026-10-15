"""Bounded outer orbital contour calibration; accepted non-eye stays fixed."""
def smooth(t):
    t=max(0.0,min(1.0,t));return t*t*(3-2*t)

def eye_weight(x,y,z):
    wx=smooth((abs(x)-.052)/.010)*smooth((.09-abs(x))/.012)
    wz=smooth((z-1.600)/.015)*smooth((1.665-z)/.008)
    return wx*wz*smooth((-y-.035)/.015)*smooth((outer_limit(z)+.006-abs(x))/.006)

def outer_eye_delta(x,y,z):
    return .0025*eye_weight(x,y,z)

def refine_outer_eye(data):
    moves=[]
    for v in data.vertices:
        x,y,z=v.co;w=eye_weight(x,y,z)
        if w:
            v.co.x+=(1 if x>0 else -1)*.0015*w
            v.co.y+=.0025*w;moves.append(w)
    data.update()
    assert all(p.area>1e-12 for p in data.polygons)
    return {"moved_vertices":len(moves),"maximum_recession_m":.0025*max(moves,default=0),"maximum_outward_m":.0015*max(moves,default=0),"bounded_eye_zone":True}


def outer_limit(z):
    # Authored lower outer aperture, accounting for the -21mm head assembly.
    z+=.021
    def level(t):
        return 1.544+.236*(.44+.014*t-.088*max(0,1-t*t)**.53)
    if z>=level(1):return .236*(.222+.11)
    lo,hi=0.,1.
    for _ in range(40):
        mid=(lo+hi)/2
        if level(mid)<z:lo=mid
        else:hi=mid
    return .236*(.222+.11*(lo+hi)/2)
