"""CPU contract checks only; native field roundtrips are deliberately unclaimed."""
import ast
import copy
from pathlib import Path
import sys
from types import SimpleNamespace as NS
import unittest

HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from corner_transport import schema,compare_exit,bmesh_corners,mesh_corners
from joint_source_fixture import source_pair


class TransportTests(unittest.TestCase):
    def test_exact_original_source_reversal(self):
        original,observed=source_pair()
        marker='    _observe_joint(locals())\n'
        self.assertEqual(observed.count(marker),1)
        restored=observed.replace(marker,'',1)
        restored=restored.replace("        _observe_cap('before',locals())\n",'',1)
        restored=restored.replace('        _observe_cap("allocated",locals())\n','',1)
        self.assertEqual(restored,original)
        compile(original,'baseline','exec');compile(observed,'observed','exec')

    def test_original_numeric_functions_not_wrapped(self):
        a,b=source_pair()
        aa=ast.parse(a);bb=ast.parse(b)
        for first,second in zip(aa.body,bb.body):
            if not isinstance(first,ast.FunctionDef):continue
            if first.name not in ('flow_reconnect_skin','retopologize_source_return_caps'):
                self.assertEqual(ast.dump(first),ast.dump(second))
        self.assertEqual(a.count('    bm.faces.remove(lower); bm.faces.remove(upper)'),1)

    def test_storage_refusal_is_preoperation(self):
        for kind in ('BYTE_COLOR','STRING','FLOAT4X4','UNKNOWN'):
            mesh=NS(attributes=[NS(name='test',data_type=kind,domain='CORNER')])
            before=copy.deepcopy(mesh.__dict__)
            with self.assertRaises(ValueError):schema(mesh)
            self.assertEqual(mesh.__dict__,before)

    def test_complete_supported_inventory(self):
        attrs=[NS(name=n,data_type=t,domain=d) for n,t,d in
               [('p','FLOAT_VECTOR','POINT'),('f','INT','FACE'),('e','FLOAT','EDGE'),
                ('uv','FLOAT2','CORNER'),('c','FLOAT_COLOR','CORNER')]]
        result=schema(NS(attributes=attrs))
        self.assertEqual({a['name']for a in result},{a.name for a in attrs})
        with self.assertRaises(ValueError):schema(NS(attributes=[NS(name='bad',data_type='FLOAT',domain='CURVE')]))

    def test_dormant_utilities_compile_without_execution(self):
        for name in ('joint_binding','field_schema','corner_ledger','corner_transport',
                     'footprint_bridge','cap_fields','cap_observer'):
            path=HERE.parent/(name+'.py')
            compile(path.read_text(),str(path),'exec')

    def test_no_automatic_runtime_activation(self):
        from construction_adapter import RUNTIME_MODULES, SOURCE_MODULES
        names={'joint_binding','field_schema','corner_ledger','corner_transport',
               'footprint_bridge','cap_fields','cap_observer'}
        self.assertFalse(names & set(RUNTIME_MODULES))
        self.assertFalse(names & set(SOURCE_MODULES))


if __name__=='__main__':unittest.main()
