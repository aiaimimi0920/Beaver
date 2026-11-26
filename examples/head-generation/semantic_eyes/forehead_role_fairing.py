"""Explicit role-max/B/V fairing mode; not pointwise-v1 acceptance."""
import numpy as np
from forehead_fairing_stage import ForeheadFairingStage
from forehead_guide_fairing import forehead_polyline_fairing
from orbital_support_rule import orbital_turns


def make_forehead_role_fairing_stage(*args, **kwargs):
    return ForeheadRoleFairingStage(*args, **kwargs)


class ForeheadRoleFairingStage(ForeheadFairingStage):
    def __init__(self, points, paths, lengths_mm, reference, reduction, weights=None,
                 turn_reduction=0., turn_goals=None, turn_roles=None):
        super().__init__(points, paths, lengths_mm, reference, reduction, weights,
                         turn_reduction, turn_goals, turn_roles)
        roles=list(paths);metrics=self.role_metrics(points);goals=turn_goals or {r:0. for r in roles}
        self.bv_caps=np.array([[metrics[r]['bending'],metrics[r]['variation']]for r in roles]).ravel()
        self.bv_norm=np.array([[self.reference['terms'][r]['bending'],self.reference['terms'][r]['variation']]for r in roles]).ravel()
        self.role_caps={r:metrics[r]['local_peak_degrees'] for r in roles}
        for r in self.turn_roles:
            self.role_caps[r]=min(self.role_caps[r],max(goals[r],self.role_caps[r]-turn_reduction))
        self.set_role_caps(self.role_caps)
        self.count=len(self.triples)+len(self.bv_caps)+1

    def set_role_caps(self,caps):
        self.role_caps=dict(caps)
        self.turn_caps=np.concatenate([np.full(len(p)-2,caps[r])for r,p in self.paths.items()])

    def bv_world(self,points):
        points=np.asarray(points,float);values=[];gradients=[]
        for role,path in self.paths.items():
            L=self.reference['terms'][role]['length_m']
            B,gB,_=forehead_polyline_fairing(points[path],L,0.)
            total,gT,detail=forehead_polyline_fairing(points[path],L,1.)
            for value,gradient in ((B,gB),(detail['dimensionless_variation'],gT-gB)):
                full=np.zeros_like(points);np.add.at(full,path,gradient)
                values.append(value);gradients.append(full)
        return np.array(values),np.array(gradients)

    def constraints(self,points):
        values,_=self.bv_world(points)
        return np.r_[self.turn_caps-orbital_turns(np.asarray(points),self.triples),
                     (self.bv_caps-values)/self.bv_norm,self.target_energy-self.energy_gradient(points)[0]]

    def bv_parameter_jacobian(self,x,evaluate,batch,ids):
        state=evaluate(x);d=batch.sample_differential(x.reshape(-1,2)/1000)
        if state is None or d is None or not np.array_equal(state[0][ids],d[0]):
            raise ValueError('V2 B/V derivative primal mismatch')
        _,g=self.bv_world(state[0]);rows=-np.einsum('kni,nia->kna',g[:,ids],d[1]/1000).reshape(len(g),-1)/self.bv_norm[:,None]
        flags=np.repeat(d[2]&np.any(np.any(g[:,ids]!=0,axis=0),axis=1),2)
        for col in np.flatnonzero(flags):
            a,b=x.copy(),x.copy();a[col]+=1e-4;b[col]-=1e-4
            first,last=evaluate(a),evaluate(b)
            if first is None or last is None:raise ValueError('No valid central V2 B/V boundary derivative')
            rows[:,col]=-(self.bv_world(first[0])[0]-self.bv_world(last[0])[0])/(2e-4*self.bv_norm)
        return rows

    def quantization_targets(self,points,offset,scales):
        smooth=self.constraints(points);saved=self.constraints(np.asarray(points).astype(np.float32).astype(float))
        failed=saved<0;safety=np.r_[np.full(len(self.triples),.001),np.full(len(self.bv_caps),1e-7),max(self.initial_energy*1e-7,1e-12)]
        targets=np.zeros(len(scales));targets[offset:offset+self.count][failed]=(np.maximum(smooth[failed]-saved[failed],0)+safety[failed])/scales[offset:offset+self.count][failed]
        return targets

    def evidence(self,points):
        out=super().evidence(points)
        out.update(guard_semantics='v2-role-max-and-separate-B-V',pointwise_v1_admission_claimed=False,
                   role_max_caps_degrees=self.role_caps,bending_variation_caps=self.bv_caps.tolist())
        return out
