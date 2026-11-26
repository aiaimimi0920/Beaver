import copy
import json
from pathlib import Path
import sys
import tempfile
import types
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.layout import pipeline_layout, parameter_layout, reconstructed


class LayoutTests(unittest.TestCase):
    def test_pipeline_order_and_shared_frame(self):
        source = """from pathlib import Path
OUT = "assets/example/"
steps = ["geometry"]
parts = [1, 2]
for role in ["Face"]:
    steps.append(("assembly", role, sum(parts)))
editable = list(steps)
for role in ["Face"]:
    members = parts
    steps.append(("runtime", role, len(members)))
steps.append(("export", members))
"""
        files, record = pipeline_layout(source)
        self.assertEqual(reconstructed(files, record), source)
        original = {}
        exec(source, original)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            folder = root / "assets/example"
            folder.mkdir(parents=True)
            for name, content in files.items():
                (folder / name).write_text(content)
            actual = {"beaver_input": lambda value: str(root / value)}
            exec(files["pipeline.py"], actual)
        self.assertEqual(actual["steps"], original["steps"])
        self.assertEqual(actual["editable"], original["editable"])
        self.assertEqual(actual["members"], original["members"])

    def test_pipeline_rejects_unrecognized_boundaries(self):
        with self.assertRaises(ValueError):
            pipeline_layout('OUT = "assets/x/"\nprint("unstructured")\n')

    def test_source_tampering_is_rejected(self):
        source = 'OUT="assets/x/"\nfor role in ["Face"]:\n    x=1\nfor role in ["Face"]:\n    x+=1\nx+=1\n'
        files, record = pipeline_layout(source)
        changed = dict(files)
        changed["pipeline_runtime.py"] += "\nx=99\n"
        with self.assertRaises(ValueError):
            reconstructed(changed, record)

    def test_sampling_optional_contract_shares_helper_globals(self):
        source = ('def upper_shape(t,c):\n    return t\n'
                  'def upper_shape_derivative(t,c):\n    return 1\n'
                  'import math\nSAMPLING_LIMITS = {"flow": 0}\n'
                  'def validate_sampling(p):\n    validate_feature_flow_contract(p)\n    return p\n'
                  'def lid_station_plan(n):\n    return n\n'
                  'def validate_feature_flow_contract(p):\n    assert p["flow"] == 1\n'
                  'import copy\nLIMITS = copy.copy(SAMPLING_LIMITS)\n')
        files, record = parameter_layout(source)
        self.assertEqual(reconstructed(files, record), source)
        helper = types.ModuleType('semantic_parameter_curves')
        exec(files['semantic_parameter_curves.py'], helper.__dict__)
        previous = sys.modules.get(helper.__name__)
        sys.modules[helper.__name__] = helper
        try:
            scope = {}
            exec(files['semantic_parameters.py'], scope)
            self.assertEqual(scope['validate_sampling']({'flow': 1}), {'flow': 1})
            with self.assertRaises(AssertionError):
                scope['validate_sampling']({'flow': 0})
        finally:
            if previous is None:
                sys.modules.pop(helper.__name__, None)
            else:
                sys.modules[helper.__name__] = previous

    def test_parameters_preserve_imported_api(self):
        source = '''"""Original public documentation."""
def upper_shape(t, c):
    return t * c["scale"]
def upper_shape_derivative(t, c):
    return c["scale"]
import copy
def adapt_style(style, p):
    result = copy.deepcopy(style)
    result["x"] = upper_shape(p["t"], p)
    return result
'''
        files, record = parameter_layout(source)
        self.assertEqual(reconstructed(files, record), source)
        helper = types.ModuleType("semantic_parameter_curves")
        exec(files["semantic_parameter_curves.py"], helper.__dict__)
        previous = sys.modules.get(helper.__name__)
        sys.modules[helper.__name__] = helper
        try:
            actual = {}
            exec(files["semantic_parameters.py"], actual)
            self.assertEqual(actual["__doc__"], "Original public documentation.")
            self.assertEqual(
                actual["adapt_style"]({}, {"t": 0.5, "scale": 2}), {"x": 1}
            )
        finally:
            if previous is None:
                sys.modules.pop(helper.__name__, None)
            else:
                sys.modules[helper.__name__] = previous

    def test_sampling_api_extracts_without_changing_source_or_behavior(self):
        source = ('def upper_shape(t,c):\n    return t\n'
                  'def upper_shape_derivative(t,c):\n    return 1\n'
                  'import math\nSAMPLING_LIMITS = {"count": 9}\n'
                  'def validate_sampling(p):\n    return {**SAMPLING_LIMITS, **p}\n'
                  'def lid_station_plan(n):\n    return math.sqrt(n)\n'
                  'import copy\nLIMITS = copy.copy(SAMPLING_LIMITS)\n')
        files, record = parameter_layout(source)
        self.assertEqual(reconstructed(files, record), source)
        helper = types.ModuleType("semantic_parameter_curves")
        exec(files["semantic_parameter_curves.py"], helper.__dict__)
        previous = sys.modules.get(helper.__name__)
        sys.modules[helper.__name__] = helper
        try:
            actual = {}
            exec(files["semantic_parameters.py"], actual)
            self.assertEqual(actual["LIMITS"], {"count": 9})
            self.assertEqual(actual["validate_sampling"]({"count": 17}), {"count": 17})
            self.assertEqual(actual["lid_station_plan"](9), 3)
        finally:
            if previous is None:
                sys.modules.pop(helper.__name__, None)
            else:
                sys.modules[helper.__name__] = previous

    def test_parameter_import_boundary_tampering_is_rejected(self):
        source = "def upper_shape(t,c):\n    return t\ndef upper_shape_derivative(t,c):\n    return 1\nimport copy\n"
        files, record = parameter_layout(source)
        changed = dict(files)
        changed["semantic_parameters.py"] = (
            "# changed\n" + changed["semantic_parameters.py"]
        )
        with self.assertRaises(ValueError):
            reconstructed(changed, record)
