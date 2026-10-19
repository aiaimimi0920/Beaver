import copy
import unittest
from verify_outline import verify_outline_guard


class OutlineReportsTests(unittest.TestCase):
    def test_required_guard_fields_and_limits(self):
        verify_outline_guard({}, {})
        s = {"family_templates": [{"id": "broad-blunt-chin-v3"}]}
        r = {
            key: {
                "outline_backtracking_factor": 0.5,
                "outline_fairing_status": "applied",
                "outline_turns_radians": [-0.1] * 4,
                "reference_turn_limits_radians": [0.0] * 4,
                "maximum_displacement_m": 0.001,
            }
            for key in ["jaw_boundary_fairing", "final_return_boundary_fairing"]
        }
        verify_outline_guard(s, r)
        for changes in [
            {"outline_backtracking_factor": True},
            {"outline_turns_radians": [0.1] * 4},
            {"maximum_displacement_m": 0.01},
            {"outline_fairing_status": "skipped_no_safe_shape_step"},
            {"reference_turn_limits_radians": [0.1] * 4},
        ]:
            bad = copy.deepcopy(r)
            bad["jaw_boundary_fairing"].update(changes)
            with self.assertRaises(ValueError):
                verify_outline_guard(s, bad)
        with self.assertRaises(ValueError):
            verify_outline_guard(s, {})
