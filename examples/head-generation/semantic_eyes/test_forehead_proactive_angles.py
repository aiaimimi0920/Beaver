"""Isolated proactive API integration tests. No optimizer or native work."""
import ast
from contextlib import ExitStack
from fractions import Fraction as F
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import numpy as np
from scipy.sparse import csr_matrix
from orbital_support_rule import orbital_quad_quality
from forehead_angle_rounding_targets import forehead_float32_angle_targets as target
from forehead_quantization_targets import forehead_make_quantization_margins as factory


class ProactiveAngles(unittest.TestCase):
    def fixture(self):
        p = np.array([[0.,0,0],[1.,0,0],[1.,1,0],[0.,1,0]]) + 1000.
        q = np.array([[0,1,2,3]])
        a = orbital_quad_quality(p,q)[0]
        b = orbital_quad_quality(p.astype(np.float32).astype(float),q)[0]
        return p,q,np.arange(1.,9.),np.arange(4),np.arange(4,8),a,b

    def test_exact_both_sides_far_from_boundary_and_idempotence(self):
        args = self.fixture()
        t,e = target(*args,variable_ids=np.arange(4))
        self.assertTrue((t>0).all())
        np.testing.assert_array_equal(t,target(*args,variable_ids=np.arange(4))[0])
        self.assertEqual(e,target(*args,variable_ids=np.arange(4))[1])
        for item in e:
            row = item['row']
            buffer = F.from_float(25.003)-F.from_float(25.) if row<4 else F.from_float(150.)-F.from_float(149.997)
            required = max(F(0),F.from_float(item['bound_degrees'])+F.from_float(.001)-buffer)
            self.assertGreaterEqual(F.from_float(t[row]),required/F.from_float(args[2][row]))
            self.assertTrue(item['current_candidate_only'])
            self.assertTrue(item['proactive_original_variable_corner'])

    def test_disabled_empty_and_three_vertex_ownership(self):
        args = self.fixture()
        for ids in (None,np.array([],dtype=int)):
            t,e = target(*args,variable_ids=ids)
            self.assertFalse(t.any());self.assertEqual(e,[])
        t,e = target(*args,variable_ids=np.array([3]))
        np.testing.assert_array_equal(np.flatnonzero(t),[0,2,3,4,6,7])

    def test_failures_preserved_and_stale_or_bad_ids_rejected(self):
        from test_forehead_angle_rounding_targets import target_fixture
        p,q,a,b = target_fixture()
        args = (p,q,np.ones(8),np.arange(4),np.arange(4,8),a,b)
        old,e = target(*args)
        fixed,fe = target(*args,variable_ids=np.array([],dtype=int))
        np.testing.assert_array_equal(old,fixed);self.assertEqual(e,fe)
        self.assertTrue(np.all(target(*args,variable_ids=np.arange(4))[0]>=old))
        args = list(self.fixture()); args[-1] = args[-1]+.1
        with self.assertRaisesRegex(ValueError,'original candidate'):
            target(*args,variable_ids=np.arange(4))
        for ids in (np.array([True]),np.array([1,1]),np.array([-1]),np.array([4]),np.array([1.])):
            with self.subTest(ids=ids),self.assertRaises(ValueError):
                target(*self.fixture(),variable_ids=ids)

    def test_factory_charge_disabled_zero_derivative_and_held_only(self):
        p,q,_,lo,hi,a,b = self.fixture()
        scales=np.ones(13);prior=np.zeros(13);prior[0]=1.;extra=np.zeros(13);extra[-1]=.7
        charges=[]
        stage=SimpleNamespace(charge=charges.append,quantization_targets=lambda *a:extra.copy())
        layout={'angle_lower':(0,4),'angle_upper':(4,8),'orientation':(8,12),'complete_guide_fairing':(12,13)}
        args=(lambda x:(p,a),lambda x:csr_matrix((13,2)),layout,scales,q,np.ones((1,4)),1.,np.ones(4))
        with ExitStack() as stack:
            stack.enter_context(patch('forehead_quantization_targets.forehead_quantization_margin_targets',return_value=(prior.copy(),{})))
            stack.enter_context(patch('forehead_quantization_targets.forehead_orientation_margin_targets',side_effect=lambda t,*a,**k:(t,[])))
            disabled,ev=factory(*args,variable_ids=np.array([0]))(np.zeros(2),np.zeros(13))
            np.testing.assert_array_equal(disabled,prior);self.assertEqual(charges,[])
            no_ids,ev=factory(*args,complete_stage=stage)(np.zeros(2),np.zeros(13))
            np.testing.assert_array_equal(no_ids,np.maximum(prior,extra));self.assertEqual(charges,[])
            callback=factory(*args,complete_stage=stage,variable_ids=np.array([0]))
            t,ev=callback(np.zeros(2),np.zeros(13))
            wanted,_=target(p,q,scales,lo,hi,a,b,variable_ids=np.array([0]))
            np.testing.assert_array_equal(t,np.maximum.reduce([prior,extra,wanted]))
            self.assertEqual(charges,['float32_angle_clearance_request'])
            self.assertGreater(t[1],0) # Remains requested despite zero local Jacobian.
            from forehead_seam_support import forehead_intersect_held_bounds
            lower,upper=forehead_intersect_held_bounds(np.zeros(2),np.ones(2),[0,1])
            np.testing.assert_array_equal(lower,upper)
            # Original IDs remain original even when every parameter is temporarily held.
            np.testing.assert_array_equal(callback(lower,np.zeros(13))[0],t)
            stage.charge=lambda kind:(_ for _ in ()).throw(ValueError('budget exhausted'))
            with self.assertRaisesRegex(ValueError,'budget exhausted'):
                callback(np.zeros(2),np.zeros(13))

    def test_cross_leg_max_without_running_optimizer(self):
        from forehead_seam_restore import forehead_restore_seam_stage
        calls=[];requests=[np.array([.7,.2]),np.array([.1,.5])]
        def callback(x,c):return requests[len(calls)].copy(),{}
        def fake_restore(start,radii,constraints,jacobian,admit,orientation_rows,domain,**kw):
            t,_=kw['margin_callback'](start,np.zeros(2));calls.append((t.copy(),list(kw['temporary_held_columns'])))
            return SimpleNamespace(x=start.copy(),success=False,message='diagnostic',nit=0),{'status':'diagnostic','steps':[]}
        with patch('forehead_seam_restore.forehead_restore_linearized',side_effect=fake_restore):
            _,meta=forehead_restore_seam_stage(np.zeros(2),np.ones(2),None,None,None,(0,1),None,{'held_columns':[0]},callback,np.ones(1),max_iterations=1,wall_seconds=1.)
        np.testing.assert_array_equal(calls[0][0],[.7,.2]);np.testing.assert_array_equal(calls[1][0],[.7,.5])
        self.assertEqual([c[1]for c in calls],[[0],[]]);self.assertTrue(meta['positive_working_targets_retained_across_legs'])

    def test_shared_fit_forwards_original_ids(self):
        tree=ast.parse((Path(__file__).parent/'forehead_shared_fit.py').read_text())
        calls=[n for n in ast.walk(tree)if isinstance(n,ast.Call)and isinstance(n.func,ast.Name)and n.func.id=='forehead_make_quantization_margins']
        self.assertEqual(len(calls),1)
        kw=next(k for k in calls[0].keywords if k.arg=='variable_ids')
        self.assertIsInstance(kw.value,ast.Name);self.assertEqual(kw.value.id,'ids')

    def test_generated_flattened_and_ordered_proactive_parity(self):
        from source_nasal_spacing import adapt
        from feature_flow_recipe import retained_sources
        retained=retained_sources()
        for name in ('forehead_angle_rounding_targets.py','forehead_quantization_targets.py','forehead_shared_fit.py'):
            self.assertEqual(retained[name],(Path(__file__).parent/name).read_text())
        pipeline='''def pipeline():
        report.update(refine_nasal_source(data))
        from shell import refine_chin_source
        report.update(refine_chin_source(data,report["chin_source_anchors"]))
'''
        flat=adapt({'pipeline.py':pipeline,'shell.py':''})['shell.py']
        namespace={};exec(compile(flat,'generated-proactive','exec'),namespace)
        args=self.fixture();kwargs={'variable_ids':np.array([0])}
        expected,evidence=target(*args,**kwargs)
        actual,ev=namespace['forehead_float32_angle_targets'](*args,**kwargs)
        np.testing.assert_array_equal(actual,expected);self.assertEqual(ev,evidence)
        sys.path.insert(0,str(Path(__file__).resolve().parent.parent))
        from source_layout.module_layout import MODULE_STAGES,module_layout,execute_ordered_module
        from source_layout.layout import reconstructed
        present={n.name for n in ast.parse(flat).body if isinstance(n,(ast.FunctionDef,ast.ClassDef))}
        earlier='pass\n'+''.join('def '+name+'():\n    pass\n'for _,name in MODULE_STAGES['shell.py'][1:]if name not in present)
        files,record=module_layout(earlier+flat,'shell.py');self.assertEqual(reconstructed(files,record),earlier+flat)
        with tempfile.TemporaryDirectory()as directory:
            root=Path(directory)
            for name,source in files.items():(root/name).write_text(source)
            replayed={};execute_ordered_module(root/'shell.py',record,replayed)
            actual,ev=replayed['forehead_float32_angle_targets'](*args,**kwargs)
        np.testing.assert_array_equal(actual,expected);self.assertEqual(ev,evidence)

if __name__=='__main__':unittest.main()
