"""Permissive continuation caps must not become final model acceptance."""
import copy
import unittest
import numpy as np
from forehead_author_atlas import ForeheadFrozenAuthorAtlas
from forehead_shared_acceptance import measure_forehead_shared_acceptance
from forehead_shared_acceptance import validate_forehead_shared_acceptance
from test_forehead_gap_fixture import make_forehead_gap_fixture


class ForeheadSharedAcceptanceTests(unittest.TestCase):
    def setUp(self):
        self.author, self.proposal, refs = make_forehead_gap_fixture()
        self.atlas = ForeheadFrozenAuthorAtlas(self.author, refs)
        points = np.asarray(self.proposal['vertices'])
        self.proposal['uniform_station_reference_length_mm'] = {
            name: float(np.linalg.norm(np.diff(points[path], axis=0), axis=1).sum()*1000)
            for name, path in self.proposal['paths'].items()}

    def test_nonuniform_fixture_fails_even_with_permissive_working_caps(self):
        first = measure_forehead_shared_acceptance(self.author, self.proposal, self.atlas)
        self.assertFalse(first['passed'])
        self.assertTrue(any(name.startswith('station:') for name in first['failures']))
        self.proposal['matched_gap_caps_v2'] = {
            domain: 100. for domain in first['shared_gap_maximum_relative_deviation']}
        second = measure_forehead_shared_acceptance(self.author, self.proposal, self.atlas)
        self.assertEqual(first['failures'], second['failures'])
        with self.assertRaisesRegex(ValueError, 'Final shared guide goals failed'):
            validate_forehead_shared_acceptance(self.author, self.proposal, self.atlas)

    def test_missing_reference_cannot_be_reinitialized_at_acceptance(self):
        del self.proposal['uniform_station_reference_length_mm']
        with self.assertRaisesRegex(ValueError, 'Frozen guide length references'):
            measure_forehead_shared_acceptance(self.author, self.proposal, self.atlas)

    def test_invalid_reference_is_rejected(self):
        self.proposal['uniform_station_reference_length_mm']['red'] = 0.
        with self.assertRaisesRegex(ValueError, 'Invalid frozen guide length'):
            measure_forehead_shared_acceptance(self.author, self.proposal, self.atlas)

    def test_acceptance_does_not_mutate_baselines_or_working_caps(self):
        before = copy.deepcopy(self.proposal)
        measure_forehead_shared_acceptance(self.author, self.proposal, self.atlas)
        self.assertEqual(before, self.proposal)


if __name__ == '__main__':
    unittest.main()
