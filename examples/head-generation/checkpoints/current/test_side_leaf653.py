import unittest
from side_leaf import leaf_sections
class SparseLeaf(unittest.TestCase):
 def test_five_semantic_stations(self):
  rows=leaf_sections(.52,.65,.76,.0176,.0025)
  self.assertEqual(len(rows),5)
  self.assertEqual([r[0] for r in rows],[.52,(.52+.65)/2,.65,(.65+.76)/2,.76])
