"""Corpus-free boundary regressions; no geometry/global optimization."""
import copy
import unittest
from unittest.mock import patch
import numpy as np
from forehead_pattern_materialize import forehead_pattern_plain_records as plain
from forehead_support_band_selection import forehead_band_angle_closure
from forehead_support_band import forehead_prospect_trisection_band
from forehead_support_inset import forehead_prospect_face_inset
from forehead_support_contract import forehead_support_topology_contract, forehead_support_admission
from forehead_support_birth_operator import forehead_solve_support_births, forehead_apply_support_births


def arrays(value):
    if isinstance(value, dict):
        return {k: arrays(v) for k,v in value.items()}
    if isinstance(value, (list,tuple)):
        try:
            candidate=np.asarray(value)
            if candidate.dtype.kind in 'biuf' and candidate.size:
                return candidate.copy()
        except ValueError:
            pass
        return [arrays(v) for v in value]
    return value


def fixture():
    vertices=[[x/64,-1.,z/64] for z in range(2) for x in range(3)]
    faces=[[0,1,4,3],[1,2,5,4]]
    edges=sorted({tuple(sorted(e)) for q in faces for e in zip(q,q[1:]+q[:1])})
    source=dict(vertices=vertices,open_delta=np.zeros((6,3)).tolist(),faces=faces)
    chart=[[x,0.,z] for x,y,z in vertices]
    p=dict(vertices=copy.deepcopy(vertices),open_delta=copy.deepcopy(source['open_delta']),
           faces=copy.deepcopy(faces),source_chart=chart,source_faces=copy.deepcopy(faces),
           lineage={},paths={'rail':[0,1,2]},protected=[0,2,3,5],support=[1,4],moving=[1,4],
           affected_old=[0,1],affected_new=[0,1],retained_caps={'rail':[1.,2.]},
           metadata={(1,4): {'structured':[True,3]}},
           topology=dict(vertices=copy.deepcopy(vertices),chart=copy.deepcopy(chart),
                         faces=copy.deepcopy(faces),lineage={},face_owners=[0,1],
                         edge_lineage=[dict(edge=list(e),parent=list(e)) for e in edges],
                         retiled_new_faces=[0,1],retiled_old_faces=[0,1]))
    fields=dict(neutral_parent_edges=edges,point_discrete={'membership':[[1,2]]*6},
                edge_fields=[dict(edge=list(e),values=[0.,0,False,True]) for e in edges])
    return source,p,edges,fields


class SupportNumpyTests(unittest.TestCase):
    def test_construction_equivalence_and_no_alias(self):
        source,p,edges,fields=fixture()
        for operator in (
            lambda p,s: forehead_band_angle_closure(p,np.array([[1,4]])),
            lambda p,s: forehead_prospect_trisection_band(p,s,np.array([[1,4]]),np.array(edges)),
            lambda p,s: forehead_prospect_face_inset(p,s,0)):
            a,b=arrays(p),arrays(source);saved_a,saved_b=plain(a),plain(b)
            actual=operator(a,b);expected=operator(p,source)
            self.assertEqual(plain(actual),plain(expected))
            self.assertEqual(plain(a),saved_a);self.assertEqual(plain(b),saved_b)
            if isinstance(actual,dict):
                self.assertIn((1,4),actual['metadata'])
                actual['vertices'][0][0]=999
                self.assertEqual(plain(a),saved_a)

    def test_contract_equivalence_and_invalid_winding_rejection(self):
        s,p,e,f=fixture();q=forehead_prospect_trisection_band(p,s,[(1,4)],e)
        self.assertTrue(forehead_support_topology_contract(arrays(p),arrays(q)))
        q['faces'][-1].reverse();q['topology']['faces']=copy.deepcopy(q['faces'])
        for before,after in ((p,q),(arrays(p),arrays(q))):
            with self.assertRaises(ValueError):forehead_support_topology_contract(before,after)

    def test_real_noop_admission_and_rejection_equivalence(self):
        s,p,e,f=fixture()
        expected=forehead_support_admission(s,p,p,f,lambda q: True)
        a,b,c=arrays(s),arrays(p),arrays(f);saved=plain([a,b,c])
        actual=forehead_support_admission(a,b,b,c,lambda q: True)
        self.assertEqual(actual,expected);self.assertEqual(plain([a,b,c]),saved)
        with self.assertRaisesRegex(ValueError,'additional|extra admission'):
            forehead_support_admission(a,b,b,c,lambda q: False)
        bad=copy.deepcopy(b);bad['topology']['chart'][0,0]+=.001
        with self.assertRaisesRegex(ValueError,'baseline'):
            forehead_support_admission(a,b,bad,c,lambda q: True)

    def test_birth_setup_equivalent_without_solver(self):
        s,p,e,f=fixture();q=forehead_prospect_trisection_band(p,s,[(1,4)],e)
        # Observe setup only. Deliberately stop before optimization/admission.
        captured=[]
        class SetupComplete(Exception):pass
        def stop(**kwargs):
            captured.append({k:plain(v) for k,v in kwargs.items() if not callable(v)})
            raise SetupComplete()
        inputs=[arrays(x) for x in (s,p,q,f)];saved=plain(inputs)
        with patch('forehead_support_birth_operator.forehead_birth_solve',stop):
            for args in ((s,p,q,f),inputs):
                with self.assertRaises(SetupComplete):forehead_solve_support_births(*args)
        self.assertEqual(captured[0],captured[1]);self.assertEqual(plain(inputs),saved)

    def test_apply_noop_with_real_admission(self):
        s,p,e,f=fixture()
        # No births or updates: the actual admission function still checks every
        # scalar row, quad, frozen baseline, guide and supplied callback.
        result=dict(admitted=True,positions={},current_field_updates={},
                    old_current_row_updates={},new_birth_rows={})
        expected=forehead_apply_support_births(s,p,p,result,f,lambda q: True)
        inputs=[arrays(x) for x in (s,p,p,result,f)];saved=plain(inputs)
        actual=forehead_apply_support_births(*inputs,lambda q: True)
        self.assertEqual(actual,expected);self.assertEqual(plain(inputs),saved)
        with self.assertRaisesRegex(ValueError,'extra admission'):
            forehead_apply_support_births(*inputs,lambda q: False)

    def test_apply_rejected_birth_never_admitted(self):
        s,p,e,f=fixture()
        for convert in (lambda x:x,arrays):
            with self.assertRaisesRegex(ValueError,'Rejected birth'):
                forehead_apply_support_births(convert(s),convert(p),convert(p),
                                              {'admitted':False},convert(f),lambda q:True)

if __name__=='__main__':unittest.main()
