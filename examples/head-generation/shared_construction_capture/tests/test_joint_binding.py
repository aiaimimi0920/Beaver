"""Synthetic producer ledgers only: no claims of native/full764 correspondence."""
import copy
import unittest
from joint_binding import bind, cycles, edge, operands, seal, invocation_from_observer, digest


def fixture(side=1):
    original=[f'producer:ear:{side}:point:{i}' for i in range(40)]
    generated={i:f'ear_patch:point:{i}' for i in (40,41,42)}
    skin={k:'skin:point:'+k for k in ('I','J','K','U','s','left','middle','right')}
    invocation={'schema':'upper-ear-source-v2','side':side,'stage_id':'verified:test-stage',
                'source_sha256':{'upper_cap':'a'*64,'skin_reconnect':'b'*64},
                'original':original,'generated':generated,'skin':skin,
                'reverse':{'skin':side>0,'cap':side<0}}
    roles=operands(original,generated,skin,side)
    faces=[]
    for slot,ps in cycles(roles,invocation['reverse']).items():
        faces.append({'id':'stage:face:'+slot,'points':ps,
                      'categorical':{'NormalRegion':1 if slot.startswith('skin') else 148},
                      'corners':[{'id':'stage:face:'+slot+':corner:'+str(i),'point':p,
                                  'uv':{'UVMap':[.2 if slot.startswith('skin') else .7,.3]},
                                  'categorical':{'pin_uv':False}}for i,p in enumerate(ps)]})
    # Include one genuine outside witness cell on the inner skin boundary.
    ps=([roles['I'],roles['J'],'outside:point'] if side>0 else
        [roles['J'],roles['I'],'outside:point'])
    faces.append({'id':'outside:face','points':ps,'categorical':{'NormalRegion':1},
                  'corners':[{'id':'outside:corner:'+str(i),'point':p,
                              'uv':{'UVMap':[.2,.3]},'categorical':{'pin_uv':False}}
                             for i,p in enumerate(ps)]})
    es={edge(a,b)for f in faces for a,b in zip(f['points'],f['points'][1:]+f['points'][:1])}
    ledger={'side':side,'stage_id':invocation['stage_id'],
            'point_ids':sorted(set(original)|set(roles.values())|{'outside:point'}),
            'faces':faces,'edges':[{'points':list(e),'categorical':{'crease':0,'group':0}}for e in sorted(es)]}
    schema={'schema':'complete-stage-fields-v1','stage_id':invocation['stage_id'],
            'domains':{'point':{'position':'float3'},'face':{'NormalRegion':'int'},
                       'edge':{'crease':'float','group':'int'},'corner':{'pin_uv':'bool'},
                       'uv':{'UVMap':'float2'}}}
    ledger['field_schema']=schema
    ledger['point_fields']={p:{'position':[0.,0.,0.]} for p in ledger['point_ids']}
    invocation['field_schema_sha256']=digest(schema)
    return invocation,ledger


class BindingTests(unittest.TestCase):
    def setUp(self):
        self.inv,self.ledger=fixture()
        self.descriptor,self.sha=seal(self.inv,self.ledger)

    def rejected(self, inv=None, ledger=None):
        with self.assertRaises(ValueError):
            bind(inv or self.inv,ledger or self.ledger,self.sha)

    def test_actual_callback_operand_adapter(self):
        inv=self.inv
        objects={p:object() for p in self.ledger['point_ids']}
        ledger={obj:p for p,obj in objects.items()}
        roles=operands(inv['original'],inv['generated'],inv['skin'],1)
        state={k:objects[v] for k,v in roles.items()}
        state['original']=[objects[p] for p in inv['original']]
        state['generated']=state['original']+[objects[inv['generated'][i]] for i in (40,41,42)]
        state['reverse']=True
        got=invocation_from_observer(state,ledger,1,inv['stage_id'],inv['source_sha256'],inv['field_schema_sha256'])
        self.assertEqual(got,inv)
        state['R']=state['L']
        with self.assertRaises(ValueError):
            invocation_from_observer(state,ledger,1,inv['stage_id'],inv['source_sha256'],inv['field_schema_sha256'])
        state['R']=object()
        with self.assertRaises(ValueError):
            invocation_from_observer(state,ledger,1,inv['stage_id'],inv['source_sha256'],inv['field_schema_sha256'])

    def test_generated_alias_outside_patch_rejected_before_seal(self):
        inv=copy.deepcopy(self.inv)
        inv['generated'][40]=inv['original'][2]
        with self.assertRaisesRegex(ValueError,'aliases'):
            operands(inv['original'],inv['generated'],inv['skin'],1)

    def test_external_wrong_winding_rejected_before_seal(self):
        ledger=copy.deepcopy(self.ledger);f=ledger['faces'][-1]
        f['points'].reverse();f['corners'].reverse()
        with self.assertRaisesRegex(ValueError,'winding'):
            seal(self.inv,ledger)

    def test_field_schema_missing_empty_and_wrong_type(self):
        for kind in ('uv','face','corner','point','edge'):
            ledger=copy.deepcopy(self.ledger)
            if kind=='uv': ledger['faces'][0]['corners'][0]['uv']={}
            if kind=='face': ledger['faces'][0]['categorical']={}
            if kind=='corner': ledger['faces'][0]['corners'][0]['categorical']['pin_uv']=1
            if kind=='point': ledger['point_fields'].pop(next(iter(ledger['point_fields'])))
            if kind=='edge': ledger['edges'][0]['categorical']['group']=False
            with self.assertRaises(ValueError): seal(self.inv,ledger)
        ledger=copy.deepcopy(self.ledger);ledger['field_schema']['domains']['uv']={}
        inv=copy.deepcopy(self.inv);inv['field_schema_sha256']=digest(ledger['field_schema'])
        with self.assertRaises(ValueError): seal(inv,ledger)

    def test_valid_both_sides_and_semantic_roles(self):
        for side in (-1,1):
            inv,ledger=fixture(side);d,h=seal(inv,ledger)
            self.assertEqual(bind(inv,ledger,h),d)
            self.assertEqual(len(d['cells']),9)
            shared=[e for e in d['edges'] if e['role']=='skin_ear_shared']
            self.assertEqual(len(shared),2)
            self.assertTrue(all(e['uv_seam'] for e in shared))
            self.assertTrue(any(e['outside']==['outside:face'] for e in d['edges']))

    def test_producer_local_reorder_rejected(self):
        inv=copy.deepcopy(self.inv)
        inv['original'][27],inv['original'][39]=inv['original'][39],inv['original'][27]
        self.rejected(inv=inv)

    def test_unrelated_local_reorder_rejected_by_seal(self):
        inv=copy.deepcopy(self.inv)
        inv['original'][2],inv['original'][3]=inv['original'][3],inv['original'][2]
        self.rejected(inv=inv)

    def test_wrong_side(self):
        ledger=copy.deepcopy(self.ledger);ledger['side']=-1
        self.rejected(ledger=ledger)

    def test_missing_face(self):
        ledger=copy.deepcopy(self.ledger);ledger['faces'].pop(0)
        self.rejected(ledger=ledger)

    def test_reversed_winding(self):
        ledger=copy.deepcopy(self.ledger);f=ledger['faces'][0]
        f['points'].reverse();f['corners'].reverse()
        self.rejected(ledger=ledger)

    def test_corner_reorder(self):
        ledger=copy.deepcopy(self.ledger);ledger['faces'][0]['corners'].reverse()
        self.rejected(ledger=ledger)

    def test_cyclic_corner_rotation_rejected_by_seal(self):
        ledger=copy.deepcopy(self.ledger);f=ledger['faces'][0]
        f['points']=f['points'][1:]+f['points'][:1]
        f['corners']=f['corners'][1:]+f['corners'][:1]
        self.rejected(ledger=ledger)

    def test_changed_uv_boundary(self):
        ledger=copy.deepcopy(self.ledger)
        ledger['faces'][6]['corners'][0]['uv']['UVMap'][0]=.2
        self.rejected(ledger=ledger)

    def test_changed_categorical_corner_face_edge(self):
        for domain in ('corner','face','edge'):
            ledger=copy.deepcopy(self.ledger)
            if domain=='corner': ledger['faces'][0]['corners'][0]['categorical']['pin_uv']=True
            if domain=='face': ledger['faces'][0]['categorical']['NormalRegion']=2
            if domain=='edge':
                key=self.descriptor['edges'][0]['points']
                next(e for e in ledger['edges'] if e['points']==key)['categorical']['crease']=1
            self.rejected(ledger=ledger)

    def test_external_witness_change(self):
        ledger=copy.deepcopy(self.ledger)
        ledger['faces'][-1]['categorical']['NormalRegion']=48
        self.rejected(ledger=ledger)

    def test_external_incidence_removed(self):
        ledger=copy.deepcopy(self.ledger);ledger['faces'].pop()
        self.rejected(ledger=ledger)

    def test_duplicate_corner_identity(self):
        ledger=copy.deepcopy(self.ledger)
        ledger['faces'][1]['corners'][0]['id']=ledger['faces'][0]['corners'][0]['id']
        self.rejected(ledger=ledger)

    def test_source_hash_or_stage_change(self):
        inv=copy.deepcopy(self.inv);inv['source_sha256']['upper_cap']='c'*64
        self.rejected(inv=inv)
        ledger=copy.deepcopy(self.ledger);ledger['stage_id']='unverified'
        self.rejected(ledger=ledger)

    def test_face_storage_reorder_is_not_identity(self):
        # Full global face order is intentionally not used; internal source order is.
        ledger=copy.deepcopy(self.ledger);ledger['faces'].reverse()
        # Edge witnesses are normalized by stable FACE ID, not BMesh enumeration.
        self.assertEqual(bind(self.inv,ledger,self.sha),self.descriptor)


if __name__=='__main__':
    unittest.main()
