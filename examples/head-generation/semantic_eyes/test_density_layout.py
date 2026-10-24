"""Focused pure-source checks for shared density layout."""
import math
from pathlib import Path
import unittest
from source_density_layout import density_rows, chin_column_indices, chin_strip_faces, subnasal_band_faces, adapt
from sampling_contract import validate_sampling

class DensityLayoutTests(unittest.TestCase):
    def test_source_rows_preserve_feature_anchors(self):
        rows=[1.544,1.551,1.560,1.570,1.580,1.590,1.599,1.6045,1.608,1.614,1.623,1.633,1.644,1.655,1.664,1.673,1.685,1.701,1.720,1.740,1.760,1.780]
        out=density_rows(rows)
        self.assertEqual(out, rows)
        self.assertEqual(out[:5], rows[:5])
        with self.assertRaises(ValueError): density_rows([y for y in rows if y != 1.551])
        with self.assertRaises(ValueError): density_rows([1.552 if y == 1.551 else y for y in rows])
        self.assertTrue(set((1.570,1.599,1.614,1.673)).issubset(out))
        self.assertGreaterEqual(min(b-a for a,b in zip(out,out[1:])),.00349)
    def test_coarse_chin_has_one_boundary_and_no_hanging_edges(self):
        from collections import Counter
        for columns in (9,13):
            faces=sum((chin_strip_faces(row,columns) for row in range(3)), [])
            uses=Counter(tuple(sorted(e)) for f in faces for e in zip(f,f[1:]+f[:1]))
            self.assertLessEqual(max(uses.values()),2)
            boundary={e for e,n in uses.items() if n==1}
            indices=chin_column_indices(columns)
            expected={tuple(sorted(e)) for e in zip(indices,indices[1:])}
            self.assertEqual(indices[-3:], [columns-3,columns-2,columns-1])
            expected|={(3*columns+i,3*columns+i+1) for i in range(columns-1)}
            expected|={(r*columns,(r+1)*columns) for r in range(3)}
            expected|={((r+1)*columns-1,(r+2)*columns-1) for r in range(3)}
            self.assertEqual(boundary,expected)
            vertices={v for f in faces for v in f}
            self.assertEqual(len(vertices)-len(uses)+len(faces),1)
    def test_cheek_patch_preserves_full_outer_and_nasal_boundaries(self):
        from collections import Counter
        for columns,nose in ((9,2),(13,3)):
            faces=subnasal_band_faces(6,columns,nose)
            uses=Counter(tuple(sorted(e)) for f in faces for e in zip(f,f[1:]+f[:1]))
            self.assertLessEqual(max(uses.values()),2)
            boundary={e for e,n in uses.items() if n==1}
            expected={(row*columns+i,row*columns+i+1) for row in (6,8) for i in range(columns-1)}
            expected|={(row*columns,(row+1)*columns) for row in (6,7)}
            expected|={((row+1)*columns-1,(row+2)*columns-1) for row in (6,7)}
            self.assertEqual(boundary,expected)
            used={v for f in faces for v in f}
            self.assertEqual(len(used)-len(uses)+len(faces),1)
            self.assertTrue({7*columns+i for i in range(nose+1)} <= used)
            self.assertTrue({7*columns+i for i in range(columns-3,columns)} <= used)
    def test_source_is_not_post_export_decimation(self):
        shell=(Path(__file__).parents[1]/'checkpoints/current/shell.py').read_text()
        face=(Path(__file__).parents[1]/'checkpoints/current/face.py').read_text()
        result=adapt({'shell.py':shell, 'face.py':face})
        out=result['shell.py']
        compile(result['face.py'],'face.py','exec')
        self.assertIn('def chin_strip_faces', result['face.py'])
        self.assertNotIn('def chin_strip_faces', out)
        compile(out,'shell.py','exec')
        self.assertIn('levels = density_rows(levels)',out)
        self.assertIn('oral_ring_fractions(kind, STYLE)',out)
        self.assertIn('def oral_ring_fractions',result['face.py'])
        self.assertNotIn('dissolve',out)
        self.assertEqual(shell.count('else [0, 0.30, 0.65]'),1)
    def test_control_rejects_fraction(self):
        self.assertEqual(validate_sampling({'sampling_density_layout':1})['sampling_density_layout'],1)
        with self.assertRaises(ValueError): validate_sampling({'sampling_density_layout':.5})

if __name__=='__main__': unittest.main()
