"""Failure gates and source-stage wiring; no historical model fixtures."""
import unittest
from unittest.mock import patch
import numpy as np
from nasal_source_binding import bind_patch, validate_patch
from nasal_source_geometry import AuthorQuadSurface
from nasal_source_core import capture_nasal_source_anchors, nasal_transverse_stations
from source_nasal_flow import adapt, MODULES


class NasalSourceBindingTests(unittest.TestCase):
    def setUp(self):
        self.vertices=np.array([(0,0,0),(.02,0,0),(.02,0,.02),(0,0,.02)])
        self.surface=AuthorQuadSurface(self.vertices,[(0,1,2,3)],[1])

    def test_trial_validates_both_diagonals_and_surface(self):
        report=validate_patch(self.surface,self.vertices,np.empty((0,3)),[(0,1,2,3)])
        self.assertEqual(report['samples'],28)
        self.assertLess(report['max_author_surface_error_mm'],1e-9)
        for quad in ((0,3,2,1),(0,1,3,2),(0,0,2,3)):
            with self.assertRaises(ValueError):
                validate_patch(self.surface,self.vertices,np.empty((0,3)),[quad])
        displaced=self.vertices.copy();displaced[:,1]=-.002
        with self.assertRaises(ValueError):
            validate_patch(self.surface,displaced,np.empty((0,3)),[(0,1,2,3)])

    def test_binding_retains_actual_boundary_and_rejects_duplicate_faces(self):
        quad=[['outer',i] for i in range(4)]
        template={'centers':[],'quads':[quad]}
        with patch('nasal_source_binding.TEMPLATE',template):
            points,quads,lineage=bind_patch(self.surface,self.vertices,np.empty((0,3)),[],[0,1,2,3])
            self.assertEqual(quads,[(0,1,2,3)])
            self.assertEqual(len(points),0);self.assertEqual(lineage,[])
        with patch('nasal_source_binding.TEMPLATE',dict(template,quads=[quad,quad])):
            with self.assertRaises(ValueError):
                bind_patch(self.surface,self.vertices,np.empty((0,3)),[],[0,1,2,3])

    def test_capture_records_current_not_historical_coordinates(self):
        heights=[1.614,1.608,1.6045,1.599,1.590]
        points=[(i,2*i,3*i) for i in range(5)]
        records=capture_nasal_source_anchors(points,heights,[.87],1)
        self.assertEqual([r['position'] for r in records],[list(p) for p in points])
        with self.assertRaises(ValueError):capture_nasal_source_anchors(points,heights,[.87,.87],2)

    def test_transverse_grade_preserves_nasal_and_outer_end_stations(self):
        original=np.linspace(1/16,1.,17)
        current=nasal_transverse_stations()
        np.testing.assert_array_equal(current[[0,13,14,15,16]],original[[0,13,14,15,16]])
        self.assertTrue(np.all(np.diff(current)>0))
        self.assertTrue(np.all(current[1:13]>original[1:13]))
        self.assertGreater(current[1]-current[0],original[1]-original[0])
        self.assertTrue(np.all(np.diff(np.diff(current[:14]))<0))

    def test_adapter_captures_before_compaction_and_runs_before_export(self):
        shell='''def build_face():
    mouth_patch_sampling_report = sample()
    flow_chains = [chin_columns, rear_ids, outline_trace["vertex_ids"]]
    compact()
    return {
        "whole_mouth_patch_sampling": mouth_patch_sampling_report,
    }
'''
        pipeline='''def run():
        report["feature_flow_ear_source"] = retopologize_flow_ears(data)
        # Automatic runtime conversion: no neutral-shape edits follow this step.
        export()
'''
        result=adapt({'shell.py':shell,'pipeline.py':pipeline})
        self.assertIn('def replace_nasal_source(',result['shell.py'])
        self.assertNotIn('from nasal_source_',result['shell.py'])
        self.assertLess(result['shell.py'].index('capture_nasal_source_anchors(verts'),
                        result['shell.py'].index('compact()'))
        self.assertLess(result['pipeline.py'].index('refine_nasal_source(data'),
                        result['pipeline.py'].index('# Automatic runtime'))
        with self.assertRaises(ValueError):adapt(result)


if __name__=='__main__':unittest.main()
