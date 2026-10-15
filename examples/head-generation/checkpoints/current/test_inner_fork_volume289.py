import unittest
from test_style_math import STYLE
from pathlib import Path
class InnerVolumeTests(unittest.TestCase):
    def test_explicit_fold_is_thin_and_parameterized(self):
        self.assertGreater(STYLE['accent_inner_fork_fold_m'], .0001)
        self.assertLess(STYLE['accent_inner_fork_fold_m'], .002)
        self.assertIn('STYLE["accent_inner_fork_fold_m"]', Path(__file__).with_name('eye_accents.py').read_text())
