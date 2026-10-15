"""Bounded seating of original short-leaf roots on their own main band."""
def seat_tuft_roots_on_parent(parts):
    from mathutils import Vector
    from mathutils.bvhtree import BVHTree
    report={}
    for side in (-1,1):
        main=next(o for o in parts if o.name=="Upper lash "+str(side))
        leaf=next(o for o in parts if o.name=="Upper swept lash tuft "+str(side))
        tree=BVHTree.FromPolygons([v.co.copy() for v in main.data.vertices],[tuple(p.vertices) for p in main.data.polygons])
        groups={}
        for v in leaf.data.vertices:groups.setdefault((round(abs(v.co.x),8),round(v.co.z,8)),[]).append(v)
        lo=min(abs(v.co.x) for v in leaf.data.vertices);hi=max(abs(v.co.x) for v in leaf.data.vertices)
        shifts=[]
        for group in groups.values():
            front=min(group,key=lambda v:v.co.y)
            p=(abs(front.co.x)-lo)/max(1e-12,hi-lo)
            u=max(0.0,min(1.0,(p-.25)/.2));weight=1-u*u*(3-2*u)
            if weight<=0:continue
            hit,_,_,_=tree.ray_cast(Vector((front.co.x,-.5,front.co.z)),Vector((0,1,0)),1.0)
            if hit is None:continue
            delta=hit.y+.00015-front.co.y
            broad=sum(abs(abs(v.co.x)-lo)<1e-7 for v in leaf.data.vertices)>1
            # The broad attached edge needs two-way seating. Behind it the
            # intentional rear scoop stays hidden under its parent sheet.
            root_weight=max(0.0,1-p/.1)**2 if broad else 0.0
            shift=max(0.0,delta)*weight+min(0.0,delta)*root_weight
            assert abs(shift)<.003,f"Short leaf parent seating exceeds bound: {shift}, p={p}, height={front.co.z}"
            for v in group:v.co.y+=shift
            shifts.append(shift)
        leaf.data.update();report[str(side)]={"maximum_rear_shift_m":max(shifts,default=0),"columns":len(shifts)}
    return report
