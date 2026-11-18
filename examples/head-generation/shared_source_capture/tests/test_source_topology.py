"""Native source-corner and explicit topology-channel regression boundaries."""

from copy import deepcopy
import unittest
from fixtures import layout, triangulate
from shared_capture_contract import freeze_source, complete_capture


def topology_attributes(mesh):
    return {".edge_verts": {"domain": "EDGE", "type": "INT32_2D",
                            "values": [row["vertices"][:] for row in mesh["edges"]]},
            ".corner_vert": {"domain": "CORNER", "type": "INT",
                             "values": [row["vertex"] for row in mesh["corners"]]},
            ".corner_edge": {"domain": "CORNER", "type": "INT",
                             "values": [row["edge"] for row in mesh["corners"]]}}


class SourceTopologyTests(unittest.TestCase):
    def test_topology_attributes_are_verified_not_ignored(self):
        for name in (".edge_verts", ".corner_vert", ".corner_edge", "actual_corner_edge"):
            source, ownership, recipe = layout(3, 4, .002)
            source["attributes"].update(topology_attributes(source))
            frozen = freeze_source(source, ownership, recipe)
            runtime, ids, shapes, normals = triangulate(source)
            runtime["attributes"].update(topology_attributes(runtime))
            complete_capture(frozen, runtime, ids, shapes, normals)
            if name == "actual_corner_edge":
                runtime["corners"][0]["edge"] = len(runtime["edges"])
            elif name == ".edge_verts":
                runtime["attributes"][name]["values"][0][0] += 1
            else:
                runtime["attributes"][name]["values"][0] += 1
            with self.assertRaises(ValueError):
                complete_capture(frozen, runtime, ids, shapes, normals)

    def test_original_corner_and_triangle_occurrence_normals_remain_distinct(self):
        source, ownership, recipe = layout(3, 4, .002)
        source["native_corner_normals"] = [[i * .01, 0., 1.] for i in range(len(source["corners"]))]
        source["attributes"]["SourceNPRNormal"] = {
            "domain": "CORNER", "type": "FLOAT_VECTOR", "values": deepcopy(source["native_corner_normals"])}
        frozen = freeze_source(source, ownership, recipe)
        runtime, ids, shapes, _ = triangulate(source)
        actual = [[0., i * .01, 1.] for i in range(len(runtime["corners"]))]
        runtime["native_corner_normals"] = actual
        packet = complete_capture(frozen, runtime, ids, shapes, actual)
        self.assertEqual(packet["source"]["mesh"]["native_corner_normals"], source["native_corner_normals"])
        self.assertEqual(packet["actual_native_corner_normals"], actual)
        self.assertEqual(packet["runtime"]["attributes"]["SourceNPRNormal"]["values"],
                         [source["native_corner_normals"][i] for i in ids["CORNER"]])
        repeated = next(i for i in ids["CORNER"] if ids["CORNER"].count(i) > 1)
        occurrences = [actual[j] for j, i in enumerate(ids["CORNER"]) if i == repeated]
        self.assertGreater(len(occurrences), 1)
        self.assertNotEqual(occurrences[0], occurrences[1])


if __name__ == "__main__":
    unittest.main()
