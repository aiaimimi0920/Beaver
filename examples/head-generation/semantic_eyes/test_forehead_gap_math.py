"""Shared domains cannot silently discard an ear span or re-admit a jaw tail."""
import copy
import unittest
import numpy as np
from forehead_author_atlas import ForeheadFrozenAuthorAtlas
from forehead_author_gap_samples import ForeheadFrozenGapResidual
from forehead_matched_gap_stage import ForeheadMatchedGapStage
from test_forehead_gap_fixture import make_forehead_gap_fixture


class ForeheadGapMathTests(unittest.TestCase):
    def setUp(self):
        self.author, self.proposal, self.references = make_forehead_gap_fixture()
        self.atlas = ForeheadFrozenAuthorAtlas(self.author, self.references)
        self.points = np.asarray(self.proposal['vertices'])
        self.params = np.asarray(self.proposal['source_chart'])

    def test_complete_lower_domains_follow_both_authored_endpoints(self):
        fast = ForeheadFrozenGapResidual(self.atlas, self.proposal, 33)
        lower = [(a, b) for a, b, kind, _, _ in fast.specs if kind == 'lower']
        self.assertEqual(lower, [('green', 'blue'), ('blue', 'purple')])
        result = fast.evaluate(self.points, self.params, details=True)
        self.assertTrue(result['valid'], result['reason'])
        slow = self.atlas.measure_all(self.proposal, 33)
        expected = np.concatenate([chart['gaps_mm'] for pair in slow['pairs']
                                   for chart in pair['charts']])
        np.testing.assert_array_equal(result['gaps_mm'], expected)
        green = next(p for p in slow['pairs'] if p['first'] == 'green')
        self.assertAlmostEqual(green['coverage']['green']['unmatched_arc_mm'], 0.)
        self.assertGreater(green['coverage']['blue']['unmatched_arc_mm'], 0.)

    def test_role_changes_are_rejected(self):
        moved = self.points.copy()
        moved[self.proposal['paths']['green'][-1], 2] += .001
        fast = ForeheadFrozenGapResidual(self.atlas, self.proposal)
        self.assertFalse(fast.evaluate(moved, self.params)['valid'])

    def test_explicit_vertex_membership_respects_returning_jaw_branch(self):
        stage = ForeheadMatchedGapStage(self.author, self.proposal, self.params,
                                        reduction=0., atlas=self.atlas)
        last = self.proposal['paths']['purple'][-1]
        self.assertLess(self.params[last, 0], self.atlas.entry)
        rows = [r for r in stage.vertex_records(self.points, self.params)
                if r['source_id'] == last and r['target'] == 'blue']
        self.assertEqual(len(rows), 3)
        self.assertTrue(all(not row['active'] for row in rows))
        values = stage.measure(self.points, self.params)
        self.assertEqual(len(values), stage.size)
        self.assertTrue(np.isfinite(values).all())

    def test_zero_reduction_preserves_owned_caps(self):
        stage = ForeheadMatchedGapStage(self.author, self.proposal, self.params,
                                        reduction=0., atlas=self.atlas)
        p = copy.deepcopy(self.proposal)
        p['matched_gap_caps_v2'] = dict(stage.caps)
        retained = ForeheadMatchedGapStage(self.author, p, self.params,
                    active_pair='green-blue:lower', reduction=0., atlas=self.atlas)
        self.assertEqual(retained.caps, stage.caps)
        self.assertEqual(retained.active, [])


if __name__ == '__main__':
    unittest.main()
