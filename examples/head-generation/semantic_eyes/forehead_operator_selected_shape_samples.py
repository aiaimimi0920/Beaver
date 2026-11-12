"""Selected paired-export triangle samples; dense final guard stays independent."""
import numpy as np
from forehead_operator_shape_analytic_samples import ForeheadAnalyticShapeSamples
from forehead_operator_shape_fit_samples import ForeheadExactDistanceCache
from chin_source_shape import ChinFrozenDistance
from source_polygon_validity import polygon_diagonals


def forehead_paired_shape_weights(points,opened,quads):
    rows=[];faces=[];diagonals=[]
    for f,q in enumerate(quads):
        options=polygon_diagonals(points,list(q),points+opened)
        if not options:raise ValueError('No current paired-export diagonal for forward sample')
        diag=max(options,key=options.get);diagonals.append(diag)
        tris=((0,1,2),(0,2,3))if diag==0 else((0,1,3),(1,2,3));local=[]
        for tri in tris:
            for i in range(5):
                for j in range(5-i):
                    w=np.zeros(4);w[list(tri)]=[i/4,j/4,1-(i+j)/4]
                    if not any(np.array_equal(w,z)for z in local):local.append(w)
        rows.extend(local);faces.extend([f]*len(local))
    return np.asarray(faces,int),np.asarray(rows),diagonals

class ForeheadSelectedForwardSamples(ForeheadAnalyticShapeSamples):
    def __init__(self,old,source_faces,old_open,initial,initial_open,quads,limit_mm=.8):
        self.limit_mm=float(limit_mm);self.solve_limit_mm=self.limit_mm-.001;self.quads=np.asarray(quads,int);self.memo={}
        old=np.asarray(old,np.float32);old_open=np.asarray(old_open,np.float32)
        self.fields=[ForeheadExactDistanceCache(ChinFrozenDistance(v,source_faces,32))for v in(old.astype(float),(old+old_open).astype(float))]
        initial=np.asarray(initial);initial_open=np.asarray(initial_open)
        self.face_ids,self.weights,self.diagonals=forehead_paired_shape_weights(initial,initial_open,self.quads)
        self.count=2*len(self.weights);value=self.values(initial,initial_open)[0]
        self.active=np.flatnonzero(value<self.limit_mm**2-.35**2);self.count=len(self.active)
    def constraints(self,points,opened_delta):return self.values(points,opened_delta)[0][self.active]
    def jacobian(self,*args):return super().jacobian(*args)[self.active]
    def full_constraints(self,points,opened_delta):
        points=np.asarray(points,np.float32);opened=np.asarray(opened_delta,np.float32)
        posed=(points+opened).astype(float);points=points.astype(float)
        ids,weights,_=forehead_paired_shape_weights(points,posed-points,self.quads)
        result=[]
        for cached,pose in zip(self.fields,(points,posed)):
            query=np.einsum('ij,ijk->ik',weights,pose[self.quads[ids]])
            result.append(self.limit_mm**2-(cached.field.distance(query)*1000)**2)
        return np.concatenate(result)
