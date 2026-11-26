"""Native selection sequence protocol regression without Blender launch."""
import unittest
from unittest.mock import patch
from construction_bmesh_state import verify_noop
from construction_bmesh_state import selection_rows


class NativeSelectionSequence:
    def __init__(self, rows):
        self.rows = rows

    def __len__(self):
        return len(self.rows)

    def __getitem__(self, index):
        return self.rows[index]

    def __iter__(self):
        raise TypeError("BMEditSelIter object is not iterable")


class SelectionSequenceTests(unittest.TestCase):
    def test_indexed_native_history_preserves_objects_and_order(self):
        values = [object(), object(), object()]
        result = selection_rows(NativeSelectionSequence(values))
        self.assertEqual(result, values)
        self.assertTrue(all(a is b for a, b in zip(result, values)))

    def test_changed_noop_state_is_rejected_with_exact_evidence(self):
        before, after = {"field": 1}, {"field": 2}
        with patch("construction_bmesh_state.snapshot", return_value=(after, {})):
            with self.assertRaisesRegex(ValueError, "without a lineage receipt") as error:
                verify_noop(object(), before, {})
        self.assertEqual(error.exception.native_diagnostic, {"phase": "dissolve_noop", "before": before, "after": after})

    def test_empty_native_history_is_not_omitted(self):
        self.assertEqual(selection_rows(NativeSelectionSequence([])), [])


class DissolveScratchTests(unittest.TestCase):
    def state(self, tag=False):
        return {"domains": {"CORNER": [{"identity": 0, "tag": tag, "point": 0, "custom": []}]},
                "geometry": [1.0, 2.0], "editor": {"selected": []}}

    def verify(self, before, after, version=(4, 3, 2)):
        from construction_bmesh_state import verify_dissolve
        with patch("construction_bmesh_state.snapshot", return_value=(after, {})):
            return verify_dissolve(object(), before, {}, native_version=version)

    def test_explicit_exact_tag_transition_retains_all_evidence(self):
        before, after = self.state(False), self.state(True)
        receipt = self.verify(before, after)
        self.assertEqual(receipt['before'], before)
        self.assertEqual(receipt['after'], after)
        self.assertEqual(receipt['corner_scratch_transitions'], [{'corner_identity': 0, 'before': False, 'after': True}])
        self.assertFalse(receipt['model_field_waiver'])

    def test_already_true_tag_is_recorded_not_omitted(self):
        receipt = self.verify(self.state(True), self.state(True))
        self.assertTrue(receipt['corner_scratch_transitions'][0]['before'])

    def test_no_general_field_or_tag_waiver(self):
        from copy import deepcopy
        valid = self.state(True)
        mutations = [lambda s: s['geometry'].__setitem__(0, 1.001),
                     lambda s: s['domains']['CORNER'][0].__setitem__('tag', False),
                     lambda s: s['domains']['CORNER'][0].__setitem__('tag', 1),
                     lambda s: s['domains']['CORNER'][0].__setitem__('point', 1),
                     lambda s: s['domains']['CORNER'][0]['custom'].append(['weight', .5]),
                     lambda s: s['editor']['selected'].append(0)]
        for mutate in mutations:
            actual = deepcopy(valid)
            mutate(actual)
            with self.assertRaisesRegex(ValueError, 'beyond the explicit'):
                self.verify(self.state(False), actual)

    def test_changed_native_version_or_unknown_tag_fails_closed(self):
        with self.assertRaisesRegex(ValueError, '4.3.2'):
            self.verify(self.state(False), self.state(True), (4, 4, 0))
        with self.assertRaisesRegex(ValueError, 'boolean'):
            self.verify(self.state(None), self.state(True))


if __name__ == "__main__":
    unittest.main()
