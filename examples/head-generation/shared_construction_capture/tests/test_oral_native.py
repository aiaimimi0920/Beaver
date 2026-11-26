"""Source allocation state guards with RNA stand-ins, not actual native evidence."""
from types import SimpleNamespace
import unittest
from construction_oral_native import ORALS, begin_oral, bind_oral, oral_state
from test_native_lifecycle import native


def bound():
    obj = native(3, 4)
    journal = SimpleNamespace(namespace='skin', points=[{'point_id': 'p' + str(i)} for i in range(4)], seam={0, 3})
    handoff = begin_oral(journal, list(range(4)))
    bind_oral(obj, handoff, [list(range(6)), list(range(6, 12))],
              [list(v.co) for v in obj.data.vertices], [list(f.vertices) for f in obj.data.polygons])
    return obj


class OralNativeGuards(unittest.TestCase):
    def tearDown(self):
        ORALS.clear()

    def test_allocations_bound_to_actual_object_mesh_and_rings(self):
        obj = bound()
        state = oral_state(obj)
        self.assertEqual(len(state['packet']['point_ids']), 12)
        self.assertEqual(len(set(state['packet']['point_ids'])), 12)
        self.assertFalse(state['packet']['field_authority'])

    def test_tampered_interior_or_handoff_cannot_be_resealed_at_join(self):
        for key in ('point_ids', 'ring_slots', 'handoff'):
            ORALS.clear()
            obj = bound()
            packet = ORALS[obj.as_pointer()]['packet']
            if key == 'point_ids':
                packet[key][8] = 'borrowed-interior'
            elif key == 'ring_slots':
                packet[key][1][0] = 0
            else:
                packet[key]['half_point_ids'][1] = 'borrowed-skin'
            with self.assertRaisesRegex(ValueError, 'Changed original oral allocation'):
                oral_state(obj)

    def test_replaced_mesh_or_changed_topology_rejects(self):
        obj = bound()
        obj.data.polygons[0].vertices = [0, 1, 2]
        with self.assertRaisesRegex(ValueError, 'topology changed'):
            oral_state(obj)

    def test_slot_alias_and_changed_seams_reject(self):
        journal = SimpleNamespace(namespace='skin', points=[{'point_id': 'p' + str(i)} for i in range(4)], seam={0, 3})
        for slots in ([0, 1, 1, 3], [False, 1, 2, 3], [3, 2, 1, 0]):
            with self.subTest(slots=slots):
                if slots == [3, 2, 1, 0]:
                    journal.seam = {0, 2, 3}
                with self.assertRaises(ValueError):
                    begin_oral(journal, slots)


if __name__ == '__main__':
    unittest.main()
