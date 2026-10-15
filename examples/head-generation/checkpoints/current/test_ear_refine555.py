import unittest
from pathlib import Path
class EarRefinement(unittest.TestCase):
 def test_original_coordinates_are_verified_after_bmesh_rebuild(self):
  source=(Path(__file__).parent/'ear_boundary.py').read_text()
  self.assertIn('original=[v.co.copy() for v in bm.verts]',source)
  self.assertIn('tree.find(p)[2]<1e-9',source)
 def test_refinement_excludes_ear_and_boundary_edges(self):
  source=(Path(__file__).parent/'ear_boundary.py').read_text()
  self.assertIn('not e.is_boundary',source)
  self.assertIn('all(f[regions] in (1,2) for f in e.link_faces)',source)
