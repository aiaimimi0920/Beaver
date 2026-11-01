"""Synthetic pure tests; runner supplies core-module import path externally."""
import copy
import unittest
import numpy as np
from forehead_support_band import forehead_prospect_trisection_band
from forehead_support_band_selection import forehead_band_angle_closure
from forehead_support_inset import forehead_prospect_face_inset
from forehead_support_contract import forehead_support_topology_contract
from forehead_operator_selected_shape_samples import ForeheadSelectedForwardSamples
from forehead_operator_reverse_shape_samples import ForeheadReverseShapeSamples

def forehead_test_grid():
    v=[[x*.01,-1.,z*.01]for z in range(2)for x in range(3)];faces=[[0,1,4,3],[1,2,5,4]];edges=sorted({tuple(sorted((a,b)))for q in faces for a,b in zip(q,q[1:]+q[:1])});chart=[[x,0.,z]for x,y,z in v]
    source={'vertices':v,'open_delta':np.zeros((6,3)).tolist(),'faces':faces};p={'vertices':copy.deepcopy(v),'open_delta':np.zeros((6,3)).tolist(),'faces':copy.deepcopy(faces),'source_chart':chart,'source_faces':copy.deepcopy(faces),'lineage':{},'paths':{'rail':[0,1,2]},'protected':[0,2,3,5],'support':[1,4],'moving':[1,4],'affected_old':[0,1],'affected_new':[0,1],'topology':{'vertices':copy.deepcopy(v),'chart':copy.deepcopy(chart),'faces':copy.deepcopy(faces),'lineage':{},'face_owners':[0,1],'edge_lineage':[{'edge':list(e),'parent':list(e)}for e in edges],'retiled_new_faces':[0,1],'retiled_old_faces':[0,1]}}
    return source,p,edges

class ForeheadSupportOperatorTests(unittest.TestCase):
    def test_trisection_winding_counts_and_immutability(self):
        source,p,edges=forehead_test_grid();prior=copy.deepcopy(p);q=forehead_prospect_trisection_band(p,source,[(1,4)],edges)
        self.assertEqual(p,prior);self.assertEqual(len(q['vertices'])-len(p['vertices']),6);self.assertEqual(len(q['faces']),8);self.assertTrue(q['support_operator_pending_birth_admission']);self.assertTrue(forehead_support_topology_contract(p,q));self.assertEqual(q['vertices'][:6],p['vertices']);self.assertEqual(q['topology']['lineage'].get(1),None)
    def test_inset_outer_edges_unchanged(self):
        source,p,edges=forehead_test_grid();q=forehead_prospect_face_inset(p,source,0)
        self.assertEqual(len(q['vertices']),10);self.assertEqual(len(q['faces']),6);self.assertTrue(forehead_support_topology_contract(p,q))
    def test_boundary_and_guide_cuts_rejected(self):
        source,p,edges=forehead_test_grid()
        for e in[(0,1),(1,2)]:
            with self.assertRaises(ValueError):forehead_prospect_trisection_band(p,source,[e],edges)
        selected,proof=forehead_band_angle_closure(p,[(1,4)])
        self.assertEqual(selected,[(1,4)]);self.assertEqual(proof['affected_faces'],[0,1])
    def test_nonneutral_requires_explicit_locus(self):
        source,p,edges=forehead_test_grid()
        with self.assertRaises(ValueError):forehead_prospect_trisection_band(p,source,[(1,4)])
        q=forehead_prospect_trisection_band(p,source,[(1,4)],allow_nonneutral_original_endpoint_chain=True,nonneutral_locus='accepted_current_chain')
        children=[z for z in q['topology']['edge_lineage']if z['parent']==[1,4]];self.assertEqual(len(children),3);self.assertTrue(forehead_support_topology_contract(p,q));self.assertEqual(q['topology']['prospective_trisection_bands'][-1]['nonneutral_locus'],'accepted_current_chain')
    def test_reversed_child_rejected(self):
        source,p,edges=forehead_test_grid();q=forehead_prospect_trisection_band(p,source,[(1,4)],edges);q['faces'][-1].reverse();q['topology']['faces']=copy.deepcopy(q['faces'])
        with self.assertRaises(ValueError):forehead_support_topology_contract(p,q)
    def test_reindexed_topology(self):
        source,p,edges=forehead_test_grid();perm=[3,5,1,4,0,2]
        for owner in (source,p):
            for key in ('vertices','open_delta'):
                old=copy.deepcopy(owner[key]);owner[key]=[old[perm.index(i)]for i in range(6)]
            owner['faces']=[[perm[i]for i in q]for q in owner['faces']]
        old=p['source_chart'];p['source_chart']=[old[perm.index(i)]for i in range(6)];p['source_faces']=copy.deepcopy(source['faces']);p['paths']={key:[perm[i]for i in path]for key,path in p['paths'].items()}
        for key in ('protected','support','moving'):p[key]=[perm[i]for i in p[key]]
        t=p['topology'];t['vertices']=copy.deepcopy(p['vertices']);t['chart']=copy.deepcopy(p['source_chart']);t['faces']=copy.deepcopy(p['faces']);t['edge_lineage']=[{'edge':sorted(perm[i]for i in z['edge']),'parent':sorted(perm[i]for i in z['parent'])}for z in t['edge_lineage']]
        neutral=[tuple(sorted(perm[i]for i in e))for e in edges];q=forehead_prospect_trisection_band(p,source,[tuple(sorted((perm[1],perm[4])))],neutral);self.assertTrue(forehead_support_topology_contract(p,q))
    def test_forward_reverse_analytic_world_columns(self):
        old=np.array([[0.,0.,0.],[.01,0.,0.],[.01,.01,0.],[0.,.01,0.]]);q=np.array([[0,1,2,3]]);v=old.copy();v[:,2]+=.0006;d=np.zeros_like(v);J=np.zeros((4,3,2));J[:,0,0]=1;J[:,2,1]=1;D=np.zeros_like(J)
        a=ForeheadSelectedForwardSamples(old,q,d,v,d,q,limit_mm=.975);b=ForeheadReverseShapeSamples(old,q,d,v,d,q,limit_mm=.975,dense_source_face_indices=[0])
        for obj in(a,b):
            jac=obj.jacobian(v,d,list(range(4)),J,D).toarray()
            for node,axis in[(0,2),(2,2),(1,0)]:
                h=1e-8;vp=v.copy();vm=v.copy();vp[node,axis]+=h;vm[node,axis]-=h;fd=(obj.constraints(vp,d)-obj.constraints(vm,d))/(2*h);col=2*node+(axis==2);self.assertLess(float(np.max(abs(fd-jac[:,col]))),.03)

if __name__=='__main__':unittest.main()
