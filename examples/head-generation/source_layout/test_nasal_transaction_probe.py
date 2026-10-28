import copy
import sys
import types
import unittest
from unittest.mock import patch
from nasal_transaction_probe import install

class Vector:
    def __init__(self,x=0.,y=0.,z=0.):self.x,self.y,self.z=x,y,z
class Pointer:
    next_id=100
    def __init__(self):self.pointer=Pointer.next_id;Pointer.next_id+=1
    def as_pointer(self):return self.pointer
class Key(Pointer):
    def __init__(self):
        super().__init__();self.key_blocks=[types.SimpleNamespace(data=[types.SimpleNamespace(co=Vector())])]
class Mesh(Pointer):
    def __init__(self,collection):
        super().__init__();self.revision=0;self.shape_keys=Key();self.collection=collection;self.fail_copy=None;collection.append(self)
    def copy(self):
        if self.fail_copy is not None:raise self.fail_copy
        result=Mesh(self.collection);result.revision=self.revision
        result.shape_keys.key_blocks=copy.deepcopy(self.shape_keys.key_blocks)
        return result
    def update(self):pass

def snapshot(data):
    return {'revision':data.revision,'keys':[[[p.co.x,p.co.y,p.co.z] for p in b.data] for b in data.shape_keys.key_blocks]}
def equal(a,b,label='snapshot'):
    if a!=b:raise ValueError(label+' differs')
    return {'equal':True}

class NativeTests(unittest.TestCase):
    def environment(self,rollback_error=None):
        meshes=[];mesh=Mesh(meshes)
        bpy=types.ModuleType('bpy');bpy.data=types.SimpleNamespace(meshes=meshes)
        bmesh=types.ModuleType('bmesh');bmesh.original_target_writes=0
        class BM:
            def from_mesh(self,data):self.revision=data.revision
            def to_mesh(self,data):
                if data is mesh:
                    bmesh.original_target_writes+=1
                    if rollback_error is not None and bmesh.original_target_writes==2:raise rollback_error
                data.revision=self.revision
            def free(self):pass
        bmesh.new=lambda:BM()
        shell=types.ModuleType('shell')
        shell.nasal_mesh_snapshot=snapshot;shell.assert_nasal_mesh_snapshot_equal=equal
        shell.validate_nasal_joint_domain=lambda:True
        exec('''def refine_nasal_source(data, ordinals, authored):
    import bpy,bmesh
    staged=data.copy()
    try:
        staged.revision+=1
        validate_nasal_joint_domain()
        new=bmesh.new();new.from_mesh(staged)
        backup=bmesh.new();backup.from_mesh(data)
        try:
            new.to_mesh(data);data.update()
        except Exception:
            backup.to_mesh(data);data.update()
            raise
        return {'nasal_joint_transaction':{'committed':True}}
    finally:
        bpy.data.meshes.remove(staged)
''',shell.__dict__)
        return mesh,bpy,bmesh,shell
    def test_injections_then_normal_refinement_and_patches_restored(self):
        mesh,bpy,bmesh,shell=self.environment();new=bmesh.new;validator=shell.validate_nasal_joint_domain
        with patch.dict(sys.modules,{'bpy':bpy,'bmesh':bmesh}):
            install(shell);result=shell.refine_nasal_source(mesh,{},[])
        self.assertEqual(mesh.revision,1);self.assertEqual(len(bpy.data.meshes),1)
        self.assertIs(bmesh.new,new);self.assertIs(shell.validate_nasal_joint_domain,validator)
        self.assertEqual(result['nasal_transaction_fault_injection']['post_write_rollback']['original_target_writes'],2)
        self.assertEqual(bmesh.original_target_writes,3)
    def test_real_rollback_error_not_hidden_and_factory_restored(self):
        error=RuntimeError('Real native rollback failure')
        mesh,bpy,bmesh,shell=self.environment(error);new=bmesh.new;validator=shell.validate_nasal_joint_domain
        with patch.dict(sys.modules,{'bpy':bpy,'bmesh':bmesh}):
            install(shell)
            with self.assertRaises(RuntimeError) as caught:shell.refine_nasal_source(mesh,{},[])
        self.assertIs(caught.exception,error);self.assertIs(bmesh.new,new)
        self.assertIs(shell.validate_nasal_joint_domain,validator);self.assertEqual(len(bpy.data.meshes),1)
    def test_double_install_rejected(self):
        mesh,bpy,bmesh,shell=self.environment()
        with patch.dict(sys.modules,{'bpy':bpy,'bmesh':bmesh}):
            install(shell)
            with self.assertRaisesRegex(ValueError,'already installed'):install(shell)

if __name__=='__main__':unittest.main()
