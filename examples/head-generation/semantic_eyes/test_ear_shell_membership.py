"""Pure topology and adapter contracts; native donor writes need native audits."""
import ast
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).parent))
from source_ear_weld_ownership import validate_ear_shell_port_chain, adapt


class EarShellMembershipTests(unittest.TestCase):
    def test_open_chain_and_permuted_indices(self):
        validate_ear_shell_port_chain([2, 9, 5, 1], {(2, 9), (5, 9), (1, 5)})
    def test_stray_branch_cycle_and_disconnected_ports_rejected(self):
        for vertices, edges in [([1, 2], [(1, 2)]),
                                ([1, 2, 3, 4], [(1, 2), (2, 3), (2, 4)]),
                                ([1, 2, 3], [(1, 2), (2, 3), (3, 1)]),
                                ([1, 2, 3, 4, 5, 6], [(1, 2), (3, 4), (4, 5), (5, 3)]),
                                ([1, 2, 3], [(1, 2), (2, 4)])]:
            with self.subTest(vertices=vertices, edges=edges), self.assertRaises(ValueError):
                validate_ear_shell_port_chain(vertices, edges)
    def test_adapter_places_capture_and_restore_next_to_existing_weld(self):
        source = {'face.py': '', 'pipeline.py': '''if True:
    _oral_skin_donors = capture_oral_skin_donors(members,members[0])
    if True:
        report["oral_skin_point_ownership"] = restore_oral_skin_donors(members[0],_oral_skin_donors)
'''}
        result = adapt(source)
        ast.parse(result['face.py']); ast.parse(result['pipeline.py'])
        self.assertLess(result['pipeline.py'].index('_ear_shell_membership ='),
                        result['pipeline.py'].index('_oral_skin_donors ='))
        self.assertIn('report["ear_shell_membership"]', result['pipeline.py'])
        self.assertNotIn('ear_shell_membership', source['pipeline.py'])
        with self.assertRaises(ValueError): adapt({'face.py': '', 'pipeline.py': ''})


if __name__ == '__main__': unittest.main()
