"""Fail-closed numerical contracts for the bounded posterior flow correction."""
from pathlib import Path
import sys
import unittest
import numpy as np
sys.path.insert(0, str(Path(__file__).parent))
from nasal_rear_rule import nasal_rear_turns, propose_nasal_rear_spacing


class NasalRearRuleTests(unittest.TestCase):
    def test_turns_and_collapsed_segment(self):
        self.assertAlmostEqual(float(nasal_rear_turns(np.array([[0,0,0],[1,0,0],[1,1,0]]))[0]),90.)
        with self.assertRaises(ValueError): nasal_rear_turns(np.zeros((3,3)))
    def test_movement_bounds_and_nonfinite_inputs(self):
        vertices=np.zeros((30,3));paths=np.arange(30).reshape(5,6)
        for maximum in (0,-.001,.0021,float('nan'),float('inf')):
            with self.subTest(maximum=maximum), self.assertRaises(ValueError):
                propose_nasal_rear_spacing(vertices,[],[],paths,np.arange(30),
                    ear_regions={47,48},protected_vertices=set(),max_move_m=maximum)
        vertices[0,0]=float('nan')
        with self.assertRaises(ValueError):
            propose_nasal_rear_spacing(vertices,[],[],paths,np.arange(30),
                ear_regions={47,48},protected_vertices=set())
    def test_five_paths_and_turn_reserve_required(self):
        for paths,reserve in ((np.zeros((4,6),int),.005),(np.zeros((5,5),int),.005),
                              (np.zeros((5,6),int),0),(np.zeros((5,6),int),.03)):
            with self.assertRaises(ValueError):
                propose_nasal_rear_spacing(np.zeros((30,3)),[],[],paths,np.arange(30),
                    ear_regions={47,48},protected_vertices=set(),reserve_deg=reserve)

if __name__ == '__main__': unittest.main()
