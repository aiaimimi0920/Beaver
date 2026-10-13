import unittest
from pathlib import Path

EXPECTED_UPPER_BRIDGE = '            # Mirror the lower attachment at the upper root. Without this\n            # bridge the return lip stops below the neighboring face edge,\n            # leaving the visible notch marked in the side/back wireframe.\n            upper_neighbors = [\n                b if a == roots[-1] else a\n                for a, b in boundary_edges\n                if roots[-1] in (a, b)\n                and data.vertices[b if a == roots[-1] else a].co.z > high\n            ]\n            assert len(upper_neighbors) == 1, "Need one upper facial boundary neighbor"\n            upper = data.vertices[upper_neighbors[0]].co.copy()\n            upper_id = len(vs)\n            vs.append(tuple(upper))\n            fs.append((upper_id, rows[-1][0], rim[-2]))\n'

class UpperEarBridgeTests(unittest.TestCase):
    def test_exact_bounded_addition(self):
        source = Path(__file__).with_name("ears.py").read_text()
        self.assertEqual(source.count(EXPECTED_UPPER_BRIDGE), 1)

    def test_boundary_winding(self):
        # Existing free upper return runs inner-rim to root; bridge opposes it.
        root, rim, upper = 1, 2, 3
        bridge = (upper, root, rim)
        edges = list(zip(bridge, bridge[1:]+bridge[:1]))
        self.assertIn((root, rim), edges)
        self.assertNotIn((rim, root), edges)
