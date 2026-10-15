"""Ensure bridge coherence is imposed before the safety backtracking."""
import unittest
from pathlib import Path

class BridgePrecondition593(unittest.TestCase):
    def test_precondition_before_backtracking_without_removing_guards(self):
        source = (Path(__file__).parent / 'ear_boundary.py').read_text()
        function = source.split('def fair_ear_jaw_rim',1)[1].split('def blend_rim_displacements',1)[0]
        self.assertLess(function.index('coherent=coherent_bridge_targets'), function.index('opening_factors={'))
        self.assertIn('eligible-bridge_ids_initial', function)
        self.assertIn("assert len(initial_bridge_groups)==2", function)
        self.assertIn("'Ear-jaw rim fairing inverted a face'", function)
        self.assertIn('tuple(data.vertices[i].co)==p for i,p in enumerate(before) if i not in changed', function)
