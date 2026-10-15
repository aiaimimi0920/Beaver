"""Keep the frozen source contract strict across AST schema versions."""
import unittest
from ast_contract import canonical_function_dump as canonical


class AstContractTests(unittest.TestCase):
    def test_only_empty_type_parameter_metadata_is_version_neutral(self):
        old = "FunctionDef(name='f', body=[Return(value=Constant(value=1))], decorator_list=[])"
        new = old[:-1] + ", type_params=[])"
        self.assertEqual(canonical(old), canonical(new))

    def test_nonempty_type_parameters_remain_a_change(self):
        old = "FunctionDef(name='f', body=[], decorator_list=[])"
        changed = old[:-1] + ", type_params=[TypeVar(name='T')])"
        self.assertNotEqual(canonical(old), canonical(changed))

    def test_function_body_changes_remain_a_change(self):
        before = "FunctionDef(name='f', body=[Return(value=Constant(value=1))], type_params=[])"
        after = before.replace("value=1", "value=2")
        self.assertNotEqual(canonical(before), canonical(after))

    def test_similar_text_inside_a_string_is_not_removed(self):
        before = "FunctionDef(name='f', body=[Constant(value='type_params=[]')], type_params=[])"
        after = before.replace("value='type_params=[]'", "value='' ")
        self.assertNotEqual(canonical(before), canonical(after))


if __name__ == "__main__":
    unittest.main()
