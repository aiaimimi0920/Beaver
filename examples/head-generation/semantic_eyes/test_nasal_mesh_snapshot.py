import copy
import unittest
from types import SimpleNamespace as N
from nasal_mesh_snapshot import nasal_mesh_snapshot, assert_nasal_mesh_snapshot_equal

class Block(N):
    def as_pointer(self):
        return self.pointer
class Collection(list):
    active = None
    active_color_name = 'Paint'
    default_color_name = 'Paint'

def fixture():
    verts = [N(co=p, groups=[N(group=0, weight=w)]) for p,w in
             [((0.,0.,0.),.2),((1.,0.,0.),.4),((1.,0.,1.),.6),((0.,0.,1.),.8)]]
    basis = Block(pointer=20,name='Basis',value=0.,slider_min=0.,slider_max=1.,mute=False,
                  interpolation='KEY_LINEAR',vertex_group='',frame=0.,data=[N(co=v.co) for v in verts])
    smile = Block(pointer=21,name='Smile',value=.3,slider_min=0.,slider_max=1.,mute=False,
                  interpolation='KEY_LINEAR',vertex_group='',frame=1.,data=[N(co=(v.co[0],.01,v.co[2])) for v in verts])
    basis.relative_key = basis; smile.relative_key = basis
    keys = N(name='IgnoredKeyID',use_relative=True,eval_time=0.,reference_key=basis,key_blocks=[basis,smile])
    uv = N(name='UVMap',active_render=True,active_clone=False,
           data=[N(uv=p,pin_uv=False) for p in [(0.,0.),(1.,0.),(1.,1.),(0.,1.)]])
    uvs=Collection([uv]);uvs.active=uv
    return N(name='IgnoredMeshID',vertices=verts,
             edges=[N(vertices=e,use_seam=False,use_edge_sharp=False) for e in [(0,1),(1,2),(2,3),(3,0)]],
             polygons=[N(vertices=(0,1,2,3),material_index=0,use_smooth=True)],
             loops=[N(vertex_index=i,edge_index=i) for i in range(4)],
             materials=[Block(pointer=10,name='IgnoredMaterialName')],uv_layers=uvs,
             color_attributes=Collection(),
             attributes=[N(name='NormalRegion',domain='FACE',data_type='INT',data=[N(value=1)]),
                         N(name='Weight',domain='POINT',data_type='FLOAT',data=[N(value=.4)]*4)],
             shape_keys=keys)

class SnapshotTests(unittest.TestCase):
    def test_identity_and_renamed_datablocks(self):
        a=fixture();b=copy.deepcopy(a);b.name='Other';b.shape_keys.name='OtherKey';b.materials[0].name='OtherMat'
        # Shape key datablock pointers can differ; relative key relationships cannot.
        b.shape_keys.key_blocks[0].pointer=200;b.shape_keys.key_blocks[1].pointer=201
        assert_nasal_mesh_snapshot_equal(nasal_mesh_snapshot(a),nasal_mesh_snapshot(b))
    def test_each_relevant_payload_change_detected(self):
        edits=[lambda b:setattr(b.vertices[0],'co',(1.,0.,0.)),
               lambda b:setattr(b.vertices[0].groups[0],'weight',.9),
               lambda b:setattr(b.uv_layers[0].data[0],'uv',(.2,.3)),
               lambda b:setattr(b.uv_layers[0].data[0],'pin_uv',True),
               lambda b:setattr(b.polygons[0],'material_index',1),
               lambda b:setattr(b.materials[0],'pointer',99),
               lambda b:setattr(b.attributes[0].data[0],'value',2),
               lambda b:setattr(b.shape_keys.key_blocks[1].data[0],'co',(0.,.2,0.)),
               lambda b:setattr(b.shape_keys.key_blocks[1],'value',.5),
               lambda b:setattr(b.shape_keys.key_blocks[1],'relative_key',b.shape_keys.key_blocks[1])]
        for edit in edits:
            a=fixture();b=copy.deepcopy(a);edit(b)
            with self.subTest(edit=edit),self.assertRaises(ValueError):
                assert_nasal_mesh_snapshot_equal(nasal_mesh_snapshot(a),nasal_mesh_snapshot(b))
    def test_unsupported_and_nonfinite_fail_closed(self):
        a=fixture();a.attributes[0].data_type='UNKNOWN'
        with self.assertRaisesRegex(ValueError,'Unsupported'):nasal_mesh_snapshot(a)
        a=fixture();a.vertices[0].co=(float('nan'),0.,0.)
        with self.assertRaisesRegex(ValueError,'Nonfinite'):nasal_mesh_snapshot(a)
    def test_schema_loss_detected(self):
        a=fixture();b=copy.deepcopy(a);b.attributes.pop()
        with self.assertRaisesRegex(ValueError,'attributes'):
            assert_nasal_mesh_snapshot_equal(nasal_mesh_snapshot(a),nasal_mesh_snapshot(b))

if __name__=='__main__':unittest.main()
