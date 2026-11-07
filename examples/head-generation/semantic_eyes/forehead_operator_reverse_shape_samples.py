"""Reverse fixed-author sample distances, with current-target triangle Jacobians."""
import numpy as np
from scipy.sparse import coo_matrix
from forehead_exact_distance import forehead_exact_distance
from source_polygon_validity import polygon_diagonals

def make_forehead_reverse_shape_samples(*args, **kwargs):
    return ForeheadReverseShapeSamples(*args, **kwargs)

class ForeheadReverseShapeSamples:
    def __init__(self,old,source_quads,old_open,initial,initial_open,target_quads,limit_mm=.8,dense_source_face_indices=()):
        self.limit_mm=float(limit_mm);self.solve_limit_mm=self.limit_mm-.001;self.triangles=[];initial=np.asarray(initial);initial_open=np.asarray(initial_open)
        for quad in target_quads:
            q=list(map(int,quad));options=polygon_diagonals(initial,q,initial+initial_open)
            if not options:raise ValueError('No paired diagonal for reverse fit witness')
            diag=max(options,key=options.get);local=((0,1,2),(0,2,3))if diag==0 else((0,1,3),(1,2,3))
            self.triangles.extend([q[k]for k in row]for row in local)
        self.triangles=np.asarray(self.triangles,int);self.target_quads=[list(map(int,q))for q in target_quads];self.source=[];self.memo={}
        weights=[]
        for u in (0.,.25,.5,.75,1.):
            for v in (0.,.25,.5,.75,1.):weights.append([(1-u)*(1-v),u*(1-v),u*v,(1-u)*v])
        weights=np.asarray(weights);old=np.asarray(old,np.float32);old_open=np.asarray(old_open,np.float32)
        for points in(old,(old+old_open)):
            rows=np.einsum('sj,fjk->fsk',weights,points[np.asarray(source_quads,int)]).reshape(-1,3)
            if dense_source_face_indices:
                dense=[]
                for u in np.linspace(0,1,33):
                    for v in np.linspace(0,1,33):dense.append([(1-u)*(1-v),u*(1-v),u*v,(1-u)*v])
                extra=np.einsum('sj,fjk->fsk',np.asarray(dense),points[np.asarray(source_quads,int)[list(dense_source_face_indices)]]).reshape(-1,3)
                rows=np.vstack((rows,extra))
            self.source.append(np.unique(rows,axis=0))
        self.offsets=np.r_[0,np.cumsum([len(q)for q in self.source])]
        initial_values=self.values(initial,initial_open)[0]
        self.active=np.flatnonzero(initial_values<self.limit_mm**2-.35**2);self.count=len(self.active)
    def values(self,points,opened_delta):
        key=(np.asarray(points).tobytes(),np.asarray(opened_delta).tobytes())
        if key in self.memo:return self.memo[key]
        values=[];records=[]
        for source,pose in zip(self.source,(points,points+opened_delta)):
            distance,ids,hits=forehead_exact_distance(source,pose,self.triangles,details=True)
            tri=pose[self.triangles[ids]];a=tri[:,1]-tri[:,0];b=tri[:,2]-tri[:,0];h=hits-tri[:,0]
            aa=np.einsum('ij,ij->i',a,a);ab=np.einsum('ij,ij->i',a,b);bb=np.einsum('ij,ij->i',b,b);ah=np.einsum('ij,ij->i',a,h);bh=np.einsum('ij,ij->i',b,h);den=aa*bb-ab*ab
            if np.any(den<=1e-30):raise ValueError('Degenerate reverse-fit triangle')
            u=(ah*bb-bh*ab)/den;v=(bh*aa-ah*ab)/den;weights=np.column_stack((1-u-v,u,v))
            values.append(self.solve_limit_mm**2-(distance*1000)**2);records.append((self.triangles[ids],weights,2e6*(source-hits)))
        out=(np.concatenate(values),records);self.memo[key]=out
        if len(self.memo)>3:self.memo.pop(next(iter(self.memo)))
        return out
    def constraints(self,points,opened_delta):return self.values(points,opened_delta)[0][self.active]
    def full_constraints(self,points,opened_delta):
        # Candidate selection uses the actual current automatic export diagonal.
        points=np.asarray(points,np.float32);opened=np.asarray(opened_delta,np.float32)
        posed=(points+opened).astype(float);points=points.astype(float);triangles=[]
        for q in self.target_quads:
            options=polygon_diagonals(points,q,posed)
            if not options:return np.array([-1000.])
            diag=max(options,key=options.get);local=((0,1,2),(0,2,3))if diag==0 else((0,1,3),(1,2,3))
            triangles.extend([q[k]for k in row]for row in local)
        return np.concatenate([self.limit_mm**2-(forehead_exact_distance(source,pose,triangles)*1000)**2 for source,pose in zip(self.source,(points,posed))])
    def jacobian(self,points,opened_delta,ids,world_jac,open_jac):
        _,records=self.values(points,opened_delta);index={v:k for k,v in enumerate(ids)};rr=[];cc=[];dd=[]
        for pose,(nodes,weights,gradient)in enumerate(records):
            derivatives=world_jac if pose==0 else world_jac+open_jac
            for corner in range(3):
                active=np.array([i in index for i in nodes[:,corner]])
                rows=np.flatnonzero(active);local=np.array([index[i]for i in nodes[active,corner]],int)
                g=np.einsum('ij,ijk->ik',gradient[active],derivatives[local])*weights[active,corner,None]
                for axis in range(2):rr.extend(rows+self.offsets[pose]);cc.extend(2*local+axis);dd.extend(g[:,axis])
        return coo_matrix((dd,(rr,cc)),shape=(self.offsets[-1],2*len(ids))).tocsr()[self.active]
