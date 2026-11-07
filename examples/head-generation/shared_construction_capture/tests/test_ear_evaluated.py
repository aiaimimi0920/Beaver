"""CPU contract tests; stand-ins do not assert native Blender success."""
import sys
from pathlib import Path
from types import SimpleNamespace as NS
import unittest
from unittest.mock import patch
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import ear_evaluated as subject


def mirror(**changes):
    values = dict(type='MIRROR', name='Mirror', use_axis=[True,False,False],
                  use_bisect_axis=[False]*3, use_mirror_merge=True, mirror_object=None,
                  show_viewport=True, show_render=True, use_clip=True, merge_threshold=1e-6)
    values.update(changes)
    return NS(**values)


class ModifierTests(unittest.TestCase):
    def test_supported_zero_viewport_subsurf_preserves_render_setting(self):
        rows = subject.modifier_contract([mirror(),NS(type='SUBSURF',name='Sub',levels=0,render_levels=2)])
        self.assertEqual(rows[1]['render_levels'],2)

    def test_missing_duplicate_or_unsupported(self):
        for modifiers in [[],[mirror(),mirror()],[NS(type='BEVEL')],
                          [mirror(),NS(type='SUBSURF',levels=1)]]:
            with self.subTest(modifiers=modifiers), self.assertRaises(ValueError):
                subject.modifier_contract(modifiers)

    def test_wrong_axis_bisect_no_merge_external_origin_and_disabled(self):
        for change in [dict(use_axis=[False,True,False]),dict(use_bisect_axis=[True,False,False]),
                       dict(use_mirror_merge=False),dict(mirror_object=object()),dict(show_viewport=False)]:
            with self.subTest(change=change), self.assertRaises(ValueError):
                subject.modifier_contract([mirror(**change)])


class LifecycleTests(unittest.TestCase):
    def lease(self, attached=True):
        shell=NS(modifiers=[mirror()])
        return dict(shell=shell,state={},before=subject.digest({'field':1}),
                    modifiers=subject.modifier_contract(shell.modifiers),
                    evaluated=None,closed=False,attached=attached)

    def test_source_markers_not_owned_are_never_removed(self):
        lease=self.lease(False)
        with patch.object(subject,'clear_verified') as clear, patch.object(subject,'fields',return_value={'field':1}), patch.object(subject,'_verify_mesh_state'):
            subject.close_evaluation(lease)
            clear.assert_not_called()

    def test_owned_markers_cleanup_and_original_mesh_verified(self):
        lease=self.lease()
        with patch.object(subject,'clear_verified') as clear, patch.object(subject,'fields',return_value={'field':1}), patch.object(subject,'_verify_mesh_state') as verify:
            subject.close_evaluation(lease)
            clear.assert_called_once_with(lease['shell'])
            verify.assert_called_once()
            with self.assertRaises(ValueError): subject.close_evaluation(lease)

    def test_cleanup_runs_even_when_evaluated_release_fails(self):
        lease=self.lease()
        def fail(): raise RuntimeError('release failure')
        lease['evaluated']=NS(to_mesh_clear=fail)
        with patch.object(subject,'clear_verified') as clear, patch.object(subject,'fields',return_value={'field':1}), patch.object(subject,'_verify_mesh_state'):
            with self.assertRaisesRegex(RuntimeError,'release failure'): subject.close_evaluation(lease)
            clear.assert_called_once()

    def test_source_field_mutation_fails(self):
        with patch.object(subject,'clear_verified'), patch.object(subject,'fields',return_value={'field':2}):
            with self.assertRaisesRegex(ValueError,'mutated'): subject.close_evaluation(self.lease())

    def test_no_native_import_or_mutation_for_wrong_version(self):
        with patch.dict(sys.modules,{'bpy':NS(app=NS(version=(4,4,0)))}), patch.object(subject,'attach') as attach:
            with self.assertRaisesRegex(ValueError,'4.3.2'): subject.begin_evaluation(None)
            attach.assert_not_called()

    def test_retired_lease_cannot_bind(self):
        with patch.dict(sys.modules,{'ear_contract':NS(bind_ear_attachment=lambda *a,**kw:None)}):
            with self.assertRaisesRegex(ValueError,'retired'):
                subject.bind_created_ear(None,{'closed':True},1,[],[],[],0,[],0,[],[],[])

class SealTests(unittest.TestCase):
    def fixture(self):
        shell={'point_ids':['shell/0'],'faces':[[0]]}
        ear={'point_ids':['ear/0','ear/1','ear/2'],'faces':[[0,1,2]]}
        receipt={'evaluated_source':shell,'ear_source':ear}
        receipt['sha256']=subject.digest(receipt)
        evaluation={'source':shell,'state':{'source':{'source_namespace':'fixture'}},'mirror':{'point_map':{}}}
        return evaluation,ear,receipt

    def test_complete_binding_is_sealed(self):
        evaluation,ear,receipt=self.fixture()
        binding=subject.seal_evaluation_binding(evaluation,ear,receipt,[0,1,2])
        self.assertEqual(binding['sha256'],subject.digest({k:v for k,v in binding.items() if k!='sha256'}))
        self.assertEqual(binding['ear_receipt_sha256'],receipt['sha256'])
        self.assertEqual(binding['ordinal_to_semantic_sha256'],subject.digest(ear['point_ids']))

    def test_tampered_receipt_rejected(self):
        evaluation,ear,receipt=self.fixture()
        receipt['injected']=True
        with self.assertRaisesRegex(ValueError,'Unsealed'):
            subject.seal_evaluation_binding(evaluation,ear,receipt,[0,1,2])

    def test_ear_allocation_or_evaluated_source_mismatch_rejected(self):
        evaluation,ear,receipt=self.fixture()
        with self.assertRaisesRegex(ValueError,'differs'):
            subject.seal_evaluation_binding(evaluation,{'point_ids':['foreign'],'faces':[[0]]},receipt,[0,1,2])
        evaluation['source']={'point_ids':['foreign'],'faces':[[0]]}
        with self.assertRaisesRegex(ValueError,'differs'):
            subject.seal_evaluation_binding(evaluation,ear,receipt,[0,1,2])

    def test_original_bridge_order_is_retained(self):
        evaluation,ear,receipt=self.fixture()
        binding=subject.seal_evaluation_binding(evaluation,ear,receipt,[2,1,0])
        self.assertEqual(binding['lower_bridge_triangle'],[2,1,0])

    def test_bridge_nontriangle_and_unknown_allocation_rejected(self):
        evaluation,ear,receipt=self.fixture()
        for triangle in ([0,1,1],[0,1,3],[True,1,2],[0,1]):
            with self.subTest(triangle=triangle),self.assertRaises(ValueError):
                subject.seal_evaluation_binding(evaluation,ear,receipt,triangle)

if __name__=='__main__': unittest.main()
