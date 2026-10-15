import unittest
from pathlib import Path
class EarBridgeNormalTests(unittest.TestCase):
    def test_bridge_endpoint_uses_existing_shared_normal_pass(self):
        root=Path(__file__).parent
        ears=(root/'ears.py').read_text(); pipeline=(root/'pipeline.py').read_text()
        self.assertIn('"lower_bridge_root_position": list(lower)', ears)
        self.assertIn('seam_points = ear_report["root_positions"] + [ear_report["lower_bridge_root_position"]]', pipeline)
        self.assertIn('for point in seam_points:', pipeline)
        self.assertIn('if index in ear_root_ids:', pipeline)
        self.assertIn('normal.lerp(ear_skin_normals[index], C["ear_root_normal_blend"])',pipeline)

        self.assertIn("ear_skin_normals[index] += poly.normal * poly.area", pipeline)
        self.assertIn("assert set(ear_skin_normals)==ear_root_ids", pipeline)
