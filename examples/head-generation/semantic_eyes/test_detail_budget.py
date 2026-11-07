"""Exercise the actual native ribbon algorithm without Blender execution."""
import ast
import json
import math
import unittest
from parameters import adapt_style
from bundle import RECIPE


class DetailBudgetTests(unittest.TestCase):
    def test_all_parent_sampling_counts_preserve_distinct_fork_stations(self):
        names = {"outline_sections", "signed_volume", "closed_ribbon_mesh"}
        tree = ast.parse((RECIPE / "thin_ribbon.py").read_text())
        functions = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
        base = json.loads((RECIPE / "face_style.json").read_text())
        for count in range(9, 22, 2):
            with self.subTest(main_stations=count):
                request = dict(sampling_density_layout=1, sampling_detail_budget=1,
                               sampling_main_stations=count, layered_lashes=1,
                               upper_lash_layer=1, short_upper_tuft=1,
                               lash_contour_follow=1, supplemental_lashes=0, lid_fold=0)
                style = adapt_style(base, request)
                scope = {"STYLE": style}
                exec(compile(ast.Module(body=functions, type_ignores=[]), "<native ribbon>", "exec"), scope)
                sections = scope["outline_sections"](style["inner_lash_fork_outline"])
                self.assertGreater(min(b[0] - a[0] for a, b in zip(sections, sections[1:])), 1e-8)
                for row in style["upper_lash_rails"]:
                    if .36 < row[0] < .76:
                        self.assertLess(min(abs(s[0] - row[0]) for s in sections), 1e-12)
                vertices, faces, _ = scope["closed_ribbon_mesh"](sections, lambda t, y, z: (t, y, z), .001, .001)
                for face in faces:
                    for i in range(1, len(face) - 1):
                        a, b, c = [vertices[j] for j in (face[0], face[i], face[i + 1])]
                        u = [b[k] - a[k] for k in range(3)]
                        v = [c[k] - a[k] for k in range(3)]
                        cross = [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]]
                        self.assertGreater(math.sqrt(sum(x*x for x in cross))/2, 1e-12)


if __name__ == "__main__":
    unittest.main()
