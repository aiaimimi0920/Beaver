"""Semantic ownership gates for the chin normal join."""
import unittest
from types import SimpleNamespace as S
from source_chin_normal_join import mark_chin_normal_join, shared_chin_normal_vertices


class Attributes(dict):
    def new(self, name, kind, domain):
        if name in self:
            raise ValueError('Duplicate attribute')
        self[name] = S(data_type=kind, domain=domain,
                       data=[S(value=0) for _ in range(5)])
        return self[name]


class ChinNormalJoinTests(unittest.TestCase):
    def fixture(self):
        data = S(vertices=list(range(5)), attributes=Attributes(), polygons=[
            S(index=0, vertices=(0, 1, 2)), S(index=1, vertices=(0, 1, 3)),
            S(index=2, vertices=(2, 3, 4))])
        regions = S(data=[S(value=1), S(value=2), S(value=9)])
        return data, regions

    def test_lateral_endpoint_not_marked(self):
        d, _ = self.fixture()
        mark_chin_normal_join(d, [0, 1, 2], [])
        self.assertEqual([x.value for x in d.attributes['SmoothChinJoin'].data], [1, 1, 0, 0, 0])

    def test_posterior_source_exclusion(self):
        d, _ = self.fixture()
        mark_chin_normal_join(d, [0, 1, 2], [1])
        self.assertEqual(d.attributes['SmoothChinJoin'].data[1].value, 0)

    def test_only_shared_skin_regions_eligible(self):
        d, r = self.fixture()
        mark_chin_normal_join(d, [0, 1, 2, 3, 4], [])
        self.assertEqual(shared_chin_normal_vertices(d, r, []), {0, 1})

    def test_actual_ear_roots_excluded(self):
        d, r = self.fixture()
        mark_chin_normal_join(d, [0, 1, 2], [])
        self.assertEqual(shared_chin_normal_vertices(d, r, [0]), {1})

    def test_missing_ownership_rejected(self):
        d, r = self.fixture()
        with self.assertRaises(ValueError):
            shared_chin_normal_vertices(d, r, [])

    def test_mismatched_vertex_field_rejected(self):
        d, r = self.fixture()
        mark_chin_normal_join(d, [0, 1, 2], [])
        d.attributes['SmoothChinJoin'].data.pop()
        with self.assertRaises(ValueError):
            shared_chin_normal_vertices(d, r, [])

    def test_lost_tag_values_are_not_a_silent_success(self):
        d, r = self.fixture()
        mark_chin_normal_join(d, [0, 1, 2], [])
        for item in d.attributes['SmoothChinJoin'].data:
            item.value = 0
        with self.assertRaises(ValueError):
            shared_chin_normal_vertices(d, r, [])

    def test_invalid_semantic_chain_rejected(self):
        for chain in ([0, 1], [0, 0, 1], [0, 8, 2]):
            d, _ = self.fixture()
            with self.assertRaises(ValueError):
                mark_chin_normal_join(d, chain, [])


if __name__ == '__main__':
    unittest.main()
