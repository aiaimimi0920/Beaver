import unittest
import sys
from pathlib import Path
if not __package__:
    sys.path.insert(0,str(Path(__file__).resolve().parent.parent))
from source_layout.surface_layout import surface_layout
from source_layout.layout import reconstructed
from source_layout.evidence import validate_layout_set, HELPERS
SOURCE='"""Example."""\n\ndef keep_before():\n    return 1\n\n'+''.join('def '+n+'():\n    return 2\n\n' for n in ('polygon_diagonals','triangulate_valid_skin','verify_skin_triangles','sample_source_cage'))+'def keep_after():\n    return 3\n'
class SurfaceLayoutTests(unittest.TestCase):
    def test_exact_reconstruction(self):
        files,r=surface_layout(SOURCE);e=files['ear_surface.py'].encode()
        self.assertEqual(reconstructed(files,r),SOURCE)
    def test_rejects_changed_reconstruction_offset(self):
        files,r=surface_layout(SOURCE);r['entry_cut_bytes']+=1
        with self.assertRaises(ValueError):reconstructed(files,r)
    def test_rejects_changed_original_identity(self):
        files,r=surface_layout(SOURCE);r['original_sha256']='bad'
        with self.assertRaises(ValueError):reconstructed(files,r)
    def test_preserves_outer_functions(self):
        files,_=surface_layout(SOURCE)
        self.assertIn('def keep_before',files['ear_surface.py']);self.assertIn('def keep_after',files['ear_surface.py'])
        self.assertNotIn('def sample_source_cage',files['ear_surface.py'])
    def test_rejects_missing_rule(self):
        with self.assertRaises(ValueError):surface_layout(SOURCE.replace('def polygon_diagonals','def wrong'))
    def test_rejects_global_state_inside_cut(self):
        with self.assertRaises(ValueError):surface_layout(SOURCE.replace('def sample_source_cage','STATE=1\ndef sample_source_cage'))
    def test_rejects_cross_boundary_global(self):
        with self.assertRaises(ValueError):surface_layout('X=3\n'+SOURCE.replace('return 2','return X'))
    def test_rejects_future_semantics(self):
        with self.assertRaises(ValueError):surface_layout('from __future__ import annotations\n'+SOURCE)
    def test_rejects_decorator(self):
        with self.assertRaises(ValueError):surface_layout(SOURCE.replace('def polygon_diagonals','@staticmethod\ndef polygon_diagonals'))
    def test_rejects_dynamic_default(self):
        with self.assertRaises(ValueError):surface_layout(SOURCE.replace('def polygon_diagonals():','def polygon_diagonals(x=object()):'))
    def test_rejects_module_identity(self):
        with self.assertRaises(ValueError):surface_layout(SOURCE.replace('return 2','return __name__'))
    def test_rejects_shadowed_builtin(self):
        with self.assertRaises(ValueError):surface_layout('len=lambda x:99\n'+SOURCE.replace('return 2','return len([])'))
    def test_rejects_dynamic_globals(self):
        with self.assertRaises(ValueError):surface_layout('X=3\n'+SOURCE.replace('return 2',"return globals()['X']"))
    def test_legacy_set(self):
        validate_layout_set({'records':[{'path':'pipeline.py'},{'path':'semantic_parameters.py'}],'helper_sha256':{'x/'+n:'a' for n in HELPERS}},'x/')
    def fixture(self):
        _,r=surface_layout(SOURCE);return {'records':[{'path':'pipeline.py'},{'path':'semantic_parameters.py'},{'path':'ear_surface.py',**r}],'helper_sha256':{**{'x/'+n:'a' for n in HELPERS},'x/source_mesh_rules.py':r['helper_sha256']}}
    def test_extended_set(self):validate_layout_set(self.fixture(),'x/')
    def test_rejects_unbound_helper(self):
        r=self.fixture();r['helper_sha256']['x/source_mesh_rules.py']='wrong'
        with self.assertRaises(ValueError):validate_layout_set(r,'x/')
    def test_rejects_extra_helper(self):
        r=self.fixture();r['helper_sha256']['x/unknown.py']='a'
        with self.assertRaises(ValueError):validate_layout_set(r,'x/')
    def test_rejects_duplicate_entry(self):
        r=self.fixture();r['records'].append(r['records'][0])
        with self.assertRaises(ValueError):validate_layout_set(r,'x/')
if __name__=='__main__':unittest.main()
