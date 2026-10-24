"""Experimental output keeps failed gates visible; normal mode still raises."""
import ast,unittest
from pathlib import Path
from sampling_contract import validate_sampling,GAME_FACE_SAMPLING
from source_density_layout import adapt

class BudgetPolicyTests(unittest.TestCase):
    def test_controls_cannot_silently_skip_pipeline_adaptation(self):
        for values in ({'sampling_surface_subdivision':0},{'sampling_surface_subdivision':0,'sampling_surface_fairing':1},{'sampling_budget_trial':1}):
            with self.assertRaises(ValueError):validate_sampling(values)
        self.assertEqual(validate_sampling(GAME_FACE_SAMPLING)['sampling_budget_trial'],0)
    def fixture(self):
        root=Path(__file__).resolve().parents[1]/'checkpoints/current'
        sources={name:(root/name).read_text() for name in ('face.py','shell.py','pipeline.py')}
        sources['pipeline.py']+='\nif True:\n    if True:\n        report["eyelid_closure"] = build_blink_shapes(members[0], semantic_part_ordinals, C, ink_floor_closure=False)\n'
        sources['semantic_eye_runtime.py']='''def build_blink_shapes(obj, ordinals, calibration, ink_floor_closure=False):
    report = {}
    report["continuous_orientation"] = verify_pose_orientation(obj)
    return report
'''
        return adapt(sources)
    def test_blink_failure_is_not_reclassified_as_passed(self):
        source=self.fixture()['semantic_eye_runtime.py']
        def fail(_):raise AssertionError('injected pose fold')
        scope={'verify_pose_orientation':fail};exec(source,scope)
        with self.assertRaises(AssertionError):scope['build_blink_shapes'](None,None,None)
        result=scope['build_blink_shapes'](None,None,None,budget_trial=True)
        self.assertEqual(result['continuous_orientation']['status'],'failed')
        self.assertFalse(result['continuous_orientation']['acceptance'])
        pipeline=self.fixture()['pipeline.py']
        lines=pipeline.splitlines();index=next(i for i,l in enumerate(lines) if 'report["eyelid_closure"] = build_blink_shapes' in l)
        snippet='\n'.join(l[8:] for l in lines[index:index+3])
        report={};exec(snippet,{'report':report,'members':[None],'semantic_part_ordinals':None,'C':None,'STYLE':{'sampling_budget_trial':1},'build_blink_shapes':lambda *a,**k:result})
        self.assertTrue(report['budget_trial_quality_pending'])
    def test_ear_diagnostic_branch_keeps_default_failure(self):
        pipeline=self.fixture()['pipeline.py'];tree=ast.parse(pipeline)
        nodes=[n for n in ast.walk(tree) if isinstance(n,ast.Try) and any(isinstance(c,ast.Call) and isinstance(c.func,ast.Name) and c.func.id=='reflow_source_fans' for c in ast.walk(n))]
        self.assertEqual(len(nodes),1)
        def fail(*_):raise AssertionError('injected ear failure')
        for trial in (0,1):
            scope={'data':None,'report':{'ear_root_fit':[]},'STYLE':{'sampling_budget_trial':trial},'reflow_source_fans':fail}
            code=compile(ast.Module(body=nodes,type_ignores=[]),'ear-test','exec')
            if not trial:
                with self.assertRaises(AssertionError):exec(code,scope)
            else:
                exec(code,scope);self.assertTrue(scope['report']['budget_trial_quality_pending']);self.assertEqual(scope['report']['ear_source_fan_reflow']['status'],'failed')

if __name__=='__main__':unittest.main()
