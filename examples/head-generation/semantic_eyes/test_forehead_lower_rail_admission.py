"""Corpus-free hard-bound/ownership tests; no mesh file or original IDs."""
import copy, unittest
from unittest.mock import patch
import numpy as np
from forehead_flow_rule import forehead_chart
from forehead_lower_rail_recovery import forehead_lower_rail_assert_strict_quality, forehead_lower_rail_commit, forehead_recover_lower_rail

class ForeheadLowerRailAdmissionTests(unittest.TestCase):

    def test_150_004_near_miss_is_rejected(self):
        with self.assertRaises(ValueError):
            forehead_lower_rail_assert_strict_quality([25.03156, 150.0041103191354], [6.969416],
                [0.498996156])
        forehead_lower_rail_assert_strict_quality([25.0, 150.0], [7.0], [0.5])
        for angles, ratio, field in [([24.999, 149.0], [6.0], [0.4]), ([26.0, 149.0], [7.00001], [0.4]),
            ([26.0, 149.0], [6.0], [0.500001]), ([np.nan], [6.0], [0.4])]:
            with self.assertRaises(ValueError):
                forehead_lower_rail_assert_strict_quality(angles, ratio, field)

    def case(self):
        v = [[0.1, -1.0, 2.0], [0.2, -1.0, 2.0], [0.2, -1.0, 2.1], [0.1, -1.0, 2.1]]
        faces = [[0, 1, 2, 3]]
        chart = forehead_chart(np.array(v)).tolist()
        fields = [[0.0, 0.0, 0.0]] * 4
        edges = {(0, 1), (1, 2), (2, 3), (0, 3)}
        edgefields = {e: (0.0, 0, False, True) for e in edges}
        top = {'vertices': copy.deepcopy(v), 'chart': copy.deepcopy(chart), 'lineage': {}, 'faces': faces,
            'face_owners': [0], 'edge_lineage': [{'edge': list(e), 'parent': list(e)} for e in edges]}
        p = {'vertices': copy.deepcopy(v), 'faces': faces, 'source_faces': faces, 'source_chart': chart,
            'topology': top, 'lineage': {}, 'open_delta': copy.deepcopy(fields), 'paths': {'rail': [0, 1, 2]},
            'protected': list(range(4)), 'moving': [], 'support': [], 'affected_new': [0], 'affected_old': [0],
            'joint_stage_caps': {'station': {'rail': 0.1}}, 'matched_gap_caps_v2': {'kept': 0.2},
            'uniform_station_reference_length_mm': {'rail': 100.0},
            'initial_old_field_baseline': copy.deepcopy(fields)}
        admission = {'admitted': True, 'positions': {}, 'old_current_row_updates': {}, 'new_birth_rows': {},
            'current_field_updates': {}}
        config = {'held_nodes': list(range(4)), 'guard_faces': [0], 'dependent_births': {}, 'movable_old': []}
        return (p, admission, config, np.array(v), np.array(fields), edgefields)

    def test_caps_and_old_fields_cannot_be_reset(self):
        base, admission, config, v, fields, edgefields = self.case()
        before = copy.deepcopy(base)
        out, audit = forehead_lower_rail_commit(base, copy.deepcopy(base), admission, config, v, fields,
            edgefields, 'rail', source_faces=base['source_faces'])
        self.assertEqual(base, before)
        self.assertTrue(audit['owned_caps_references_exact'])
        for key in ['joint_stage_caps', 'matched_gap_caps_v2', 'uniform_station_reference_length_mm',
            'initial_old_field_baseline']:
            bad = copy.deepcopy(base)
            bad[key] = {'reset': True}
            with self.assertRaises(ValueError):
                forehead_lower_rail_commit(base, bad, admission, config, v, fields, edgefields, 'rail', source_faces=base['source_faces'])
        bad = copy.deepcopy(base)
        bad['open_delta'][0] = [0.001, 0, 0]
        with self.assertRaises(ValueError):
            forehead_lower_rail_commit(base, bad, admission, config, v, fields, edgefields, 'rail', source_faces=base['source_faces'])

    def test_other_guide_and_old_birth_ownership_rejected(self):
        base, admission, config, vertices, fields, edgefields = self.case()
        base["paths"]["other"] = [3, 2]
        config["held_nodes"] = []
        for mutation in ("path", "position", "frozen_position", "frozen_row"):
            candidate = copy.deepcopy(base)
            if mutation == "path":
                candidate["paths"]["other"] = [2, 3]
            elif mutation == "position":
                candidate["vertices"][3][0] += 0.0001
            elif mutation == "frozen_position":
                candidate["topology"]["vertices"][3][0] += 0.0001
            else:
                base["topology"]["lineage"][3] = {"donors": [3], "weights": [1.0]}
                candidate["topology"]["lineage"][3] = {"donors": [2], "weights": [1.0]}
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                forehead_lower_rail_commit(
                    base, candidate, admission, config, vertices, fields, edgefields, "rail", source_faces=base["source_faces"]
                )

    def test_numpy_noop_commit_preserves_frozen_values_and_input(self):
        from forehead_pattern_materialize import forehead_pattern_plain_records
        base, admission, config, vertices, fields, edgefields = self.case()
        for owner, keys in ((base, ("vertices", "open_delta", "source_chart", "initial_old_field_baseline")),
                            (base["topology"], ("vertices", "chart", "face_owners"))):
            for key in keys:
                owner[key] = np.asarray(owner[key])
        base["faces"] = np.asarray(base["faces"])
        base["paths"]["rail"] = np.asarray(base["paths"]["rail"])
        before = forehead_pattern_plain_records(base)
        result, audit = forehead_lower_rail_commit(
            base, base, admission, config, vertices, fields, edgefields, "rail",
            source_faces=np.asarray(base["source_faces"]),
        )
        self.assertEqual(forehead_pattern_plain_records(base), before)
        self.assertEqual(result["vertices"], before["vertices"])
        self.assertEqual(result["topology"], before["topology"])
        self.assertTrue(audit["owned_caps_references_exact"])

    def test_native_adapter_preserves_ownership_and_structured_capture(self):
        p, admission, config, v, fields, edgefields = self.case()
        author = {'vertices': v.tolist(), 'faces': p['faces'], 'open_delta': fields.tolist()}
        capture = {'original_vertex_count': 4, 'original_face_count': 1, 'frozen_before_allocation': True,
            'native_source_mesh_unchanged': True, 'edge_fields': [{'edge': list(e),
            'values': list(vals)} for e, vals in edgefields.items()],
            'neutral_parent_edges': [list(e) for e in edgefields], 'point_discrete': {'memberships': [[[0,
            1.0]]] * 4}}
        before = copy.deepcopy((author, capture, p))
        with patch('forehead_lower_rail_recovery.forehead_lower_rail_recover',
            return_value={'admitted': False, 'status': 'TEST ONLY'}) as inner:
            forehead_recover_lower_rail(author, capture, p, 'rail', [0], target_feedback_policy='diagnose')
            self.assertEqual(inner.call_args.args[3]['memberships'], [((0, 1.0),)] * 4)
        self.assertEqual((author, capture, p), before)
        bad = copy.deepcopy(author)
        bad['open_delta'][0] = [0.001, 0, 0]
        with self.assertRaisesRegex(ValueError, 'frozen field ownership'):
            forehead_recover_lower_rail(bad, capture, p, 'rail', [0])
if __name__ == '__main__':
    unittest.main()
