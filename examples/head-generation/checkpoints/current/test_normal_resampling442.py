import ast,unittest
from pathlib import Path
class SectionNormalResampling(unittest.TestCase):
 def test_resampling_only_intermediate_front_lanes(self):
  t=(Path(__file__).parent/'pipeline.py').read_text();ast.parse(t)
  self.assertIn('((1, 2, (0, 2)), (3, 3, (2, 4)))',t)
  self.assertIn('assert len(ordered) == 5',t)
  self.assertIn('normal = endpoints[0].lerp(endpoints[1], fraction).normalized()',t)
