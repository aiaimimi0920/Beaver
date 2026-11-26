"""Real stage dispatch/state isolation; mocked math never certifies a mesh."""
import copy
import hashlib
import json
import sys
import types
import unittest
from unittest.mock import patch, Mock
import forehead_shared_consumer as consumer
import forehead_retile_mesh as stage
import nasal_spacing_joint as joint


class SharedConsumerTests(unittest.TestCase):
    def setUp(self):
        self.author={'vertices':[[0.,0.,0.]],'faces':[],'regions':[],
            'open_delta':[[0.,0.,0.]],'report':{},'ordinals':{}}
        self.proposal={'vertices':[[0.,0.,0.]],'faces':[], 'paths':{},
            'source_chart':[[0.,0.,0.]],'source_faces':[],
            'uniform_station_reference_length_mm':{'example':1.},
            'topology':{'vertices':[[0.,0.,0.]],'chart':[[0.,0.,0.]],'lineage':{}}}
        self.saved=[]
    def sink(self,value):
        self.saved.append(copy.deepcopy(value))
        digest=hashlib.sha256(consumer.forehead_consumer_checkpoint_bytes(value)).hexdigest()
        value['positive_proposal']['vertices'][0][0]=999 # cannot alias the caller
        return {'persisted':True,'sha256':digest}
    def restore(self,*args,**kwargs):
        self.assertEqual(kwargs['gap_reduction'],0.)
        self.assertEqual(kwargs['progress_names'],[])
        self.assertEqual(kwargs['objective_mode'],'restore')
        p=copy.deepcopy(args[2]);p['topology']['lineage']={int(k):v for k,v in p['topology']['lineage'].items()}
        p['joint_stage_caps']={'held':1.}
        p['matched_gap_caps_v2']={str(i):.3 for i in range(10)}
        return p
    def run_consumer(self,options,sink=None):
        with patch.object(consumer,'initialize_forehead_shared',return_value={
                'admitted':True,'status':'initial_atlas_ready','proposal':self.proposal}), \
             patch.object(consumer,'fit_forehead_shared_constraints',side_effect=self.restore), \
             patch.object(consumer,'measure_forehead_shared_acceptance',return_value={'passed':False}), \
             patch.object(consumer,'continue_forehead_flows',side_effect=lambda a,p,**kw:{'proposal':p,'stages':[]}):
            return consumer.prepare_forehead_shared_consumer(self.author,self.author,
                {'edge_fields':{(0,1):(0.,0,False,True)},'neutral_parent_edges':{(0,1)}},
                {'assembly_offset':-.021},options,self.sink if sink is None else sink)
    def test_fresh_owned_checkpoint_no_native_ready(self):
        old=copy.deepcopy(self.proposal)
        result=self.run_consumer({'mode':'fresh'})
        self.assertEqual(self.proposal,old)
        self.assertEqual(result['positive_proposal']['vertices'],old['vertices'])
        self.assertFalse(result['native_ready']);self.assertFalse(result['full_model_success'])
        self.assertTrue(self.saved)
    def test_resume_no_initializer_and_exact_source_binding(self):
        self.proposal['topology']['lineage']={0:{'donors':[0],'weights':[1.]}}
        first=json.loads(consumer.forehead_consumer_checkpoint_bytes(self.run_consumer({'mode':'fresh','flow_steps':0})))
        self.assertIn('0',first['positive_proposal']['topology']['lineage'])
        snapshot=copy.deepcopy(first)
        with patch.object(consumer,'initialize_forehead_shared',side_effect=AssertionError('must not initialize')), \
             patch.object(consumer,'fit_forehead_shared_constraints',side_effect=self.restore), \
             patch.object(consumer,'measure_forehead_shared_acceptance',return_value={'passed':True}):
            result=consumer.prepare_forehead_shared_consumer(self.author,self.author,
                {'edge_fields':{(0,1):(0.,0,False,True)},'neutral_parent_edges':{(0,1)}},
                {'assembly_offset':-.021},{'mode':'resume','resume':first,'flow_steps':0},self.sink)
        self.assertEqual(first,snapshot);self.assertFalse(result['native_ready'])
        first['source_binding']='wrong'
        with self.assertRaises(ValueError):self.run_consumer({'mode':'resume','resume':first})
    def test_zero_progress_resume_is_explicit_optimizing_restore(self):
        first=self.run_consumer({'mode':'fresh','flow_steps':0})
        def moving_restore(*args,**kwargs):
            proposal=self.restore(*args,**kwargs)
            proposal['vertices'][0][0]+=.0001
            return proposal
        with patch.object(consumer,'fit_forehead_shared_constraints',side_effect=moving_restore), \
             patch.object(consumer,'measure_forehead_shared_acceptance',return_value={'passed':True}):
            result=consumer.prepare_forehead_shared_consumer(self.author,self.author,
                {'edge_fields':{(0,1):(0.,0,False,True)},'neutral_parent_edges':{(0,1)}},
                {'assembly_offset':-.021},{'mode':'resume','resume':first,'flow_steps':0},self.sink)
        self.assertTrue(result['restoration']['geometry_changed'])
        self.assertEqual(result['restoration']['mode'],'optimizing_restore')
        self.assertTrue(result['prior_geometry_evidence_invalidated'])
        self.assertFalse(result['shape_checked'])

    def test_budget_adapter_reaches_bootstrap_and_all_three_drivers(self):
        calls=[]
        def fit(*args,**kwargs):
            self.assertEqual(kwargs['seam_wall_seconds'],240.)
            self.assertEqual(kwargs['seam_initial_held_seconds'],180.)
            self.assertIs(kwargs['paired_diagonal_guidance'],True)
            calls.append(kwargs)
            return self.restore(*args,**kwargs)
        def driver(*args,**kwargs):
            proposal=args[2] if len(args)==4 else args[1]
            candidate=kwargs['fitter'](self.author['vertices'],self.author['open_delta'],proposal,
                progress_names=[],gap_reduction=0.,objective_mode='restore')
            return {'proposal':candidate,'stages':[]}
        with patch.object(consumer,'initialize_forehead_shared',return_value={'admitted':True,'status':'initial_atlas_ready','proposal':self.proposal}), patch.object(consumer,'fit_forehead_shared_constraints',side_effect=fit), patch.object(consumer,'continue_forehead_flows',side_effect=driver), patch.object(consumer,'continue_forehead_fairing',side_effect=driver), patch.object(consumer,'continue_forehead_gaps',side_effect=driver), patch.object(consumer,'measure_forehead_shared_acceptance',return_value={'passed':False}):
            consumer.prepare_forehead_shared_consumer(self.author,self.author,{}, {},
                {'mode':'fresh','flow_steps':1,'fairing_stages':1,'gap_steps':1,
                 'seam_active_set_growth':True,'seam_wall_seconds':240.,'seam_initial_held_seconds':180.,
                 'paired_diagonal_guidance':True},self.sink)
        self.assertEqual(len(calls),4)
        with self.assertRaises(ValueError):
            consumer.forehead_consumer_options({'mode':'fresh','seam_wall_seconds':240.})

    def test_paired_guidance_default_and_strict_bool(self):
        self.assertIs(consumer.forehead_consumer_options({'mode':'fresh'})['paired_diagonal_guidance'],False)
        for value in (None,0,1,'true',[],{}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                consumer.forehead_consumer_options({'mode':'fresh','paired_diagonal_guidance':value})
        with patch.object(consumer,'initialize_forehead_shared',return_value={'admitted':True,'status':'initial_atlas_ready','proposal':self.proposal}), patch.object(consumer,'fit_forehead_shared_constraints',side_effect=self.restore) as fit, patch.object(consumer,'measure_forehead_shared_acceptance',return_value={'passed':False}):
            consumer.prepare_forehead_shared_consumer(self.author,self.author,{}, {},
                {'mode':'fresh','flow_steps':0},self.sink)
        self.assertIs(fit.call_args.kwargs['paired_diagonal_guidance'],False)

    def test_author_change_evidence_is_owned_and_persisted(self):
        evidence={'changed_points':[{'vertex':0,'original':[0.,0.,0.],'author':[0.,0.,.0001]}],
                  'pose_correspondence':{'passed':True}}
        self.author['author_shape_change']=evidence
        result=self.run_consumer({'mode':'fresh','flow_steps':0})
        self.assertEqual(result['author_shape_change'],evidence)
        self.assertEqual(self.saved[-1]['author_shape_change'],evidence)
        result['author_shape_change']['changed_points'].clear()
        self.assertEqual(len(evidence['changed_points']),1)

    def test_skin_receipt_serialized_resume_and_native_label_change(self):
        from test_forehead_skin_ownership import SkinOwnershipTests
        receipt = SkinOwnershipTests().receipt()
        self.author['regions'] = [71, 72, 94]
        self.author['report'] = {'ear_shell_membership': {'authored_skin_ownership': receipt}}
        first = json.loads(consumer.forehead_consumer_checkpoint_bytes(
            self.run_consumer({'mode': 'fresh', 'flow_steps': 0})))
        self.assertEqual(first['authored_skin_ownership']['source_face_ids'], [0, 1])
        resumed = self.run_consumer({'mode': 'resume', 'resume': first, 'flow_steps': 0})
        self.assertEqual(resumed['authored_skin_ownership'], first['authored_skin_ownership'])
        altered = copy.deepcopy(first)
        altered['authored_skin_ownership']['source_face_ids'] = [2]
        with self.assertRaisesRegex(ValueError, 'receipt mismatch'):
            self.run_consumer({'mode': 'resume', 'resume': altered, 'flow_steps': 0})
        self.author['regions'] = [94, 72, 71]
        with self.assertRaisesRegex(ValueError, 'binding mismatch'):
            self.run_consumer({'mode': 'resume', 'resume': first, 'flow_steps': 0})

    def test_missing_ownership_old_checkpoint_remains_legacy_compatible(self):
        first = self.run_consumer({'mode': 'fresh', 'flow_steps': 0})
        first.pop('authored_skin_ownership')
        result = self.run_consumer({'mode': 'resume', 'resume': first, 'flow_steps': 0})
        self.assertFalse(result['authored_skin_ownership']['available'])
        self.assertFalse(result['native_ready'])

    def test_uv_value_only_binding_change_rejects_resume(self):
        raw=copy.deepcopy(self.author)
        raw['stable_native_source_values']={'uv':{'UVMap':'value-hash-A'}}
        first=self.run_consumer({'mode':'fresh','flow_steps':0})
        first['source_binding']=consumer.forehead_consumer_binding(raw,self.author,
            {'edge_fields':{(0,1):(0.,0,False,True)},'neutral_parent_edges':{(0,1)}},
            {'assembly_offset':-.021})
        raw['stable_native_source_values']['uv']['UVMap']='value-hash-B'
        with self.assertRaisesRegex(ValueError,'binding mismatch'), patch.object(consumer,'fit_forehead_shared_constraints',side_effect=AssertionError('must not solve')):
            consumer.prepare_forehead_shared_consumer(raw,self.author,
                {'edge_fields':{(0,1):(0.,0,False,True)},'neutral_parent_edges':{(0,1)}},
                {'assembly_offset':-.021},{'mode':'resume','resume':first},self.sink)

    def test_json_lineage_key_normalization_preserves_values(self):
        before=copy.deepcopy(self.proposal);after=copy.deepcopy(self.proposal)
        before['topology']['lineage']={'0':{'donors':[0],'weights':[1.]}}
        after['topology']['lineage']={0:{'donors':[0],'weights':[1.]}}
        self.assertTrue(consumer.forehead_consumer_retains(before,after))
        duplicate=copy.deepcopy(after);duplicate['topology']['lineage']['0']={'donors':[0],'weights':[1.]}
        with self.assertRaises(ValueError):consumer.forehead_consumer_retains(before,duplicate)
        after['topology']['lineage'][0]['weights']=[.99]
        with self.assertRaises(ValueError):consumer.forehead_consumer_retains(before,after)

    def test_sink_required_and_failed_receipt_stops(self):
        with self.assertRaises(RuntimeError):self.run_consumer({'mode':'fresh'},sink=lambda p:None)
        with self.assertRaises(ValueError):
            consumer.prepare_forehead_shared_consumer({}, {}, {}, {}, {'mode':'fresh'})
    def test_mirrored_resume_rejected(self):
        first=self.run_consumer({'mode':'fresh','flow_steps':0})
        first['positive_proposal']['guide_paths']={}
        with self.assertRaises(ValueError):self.run_consumer({'mode':'resume','resume':first})


class NativeStageDispatchTests(unittest.TestCase):
    def setUp(self):
        self.data=types.SimpleNamespace(shape_keys=None,materials=[],vertices=[types.SimpleNamespace(co=[0.,0.,0.])],
            polygons=[],attributes={'NormalRegion':types.SimpleNamespace(domain='FACE',data_type='INT',data=[]),
            'MouthOpenDelta':types.SimpleNamespace(domain='POINT',data_type='FLOAT_VECTOR',
                data=[types.SimpleNamespace(vector=[0.,0.,0.])])})
        self.context={'calibration':{},'report':{},'shared_workflow':{'mode':'fresh'},'shared_checkpoint':lambda p:None}
        self.modules={n:types.SimpleNamespace(**{attr:lambda *a:None}) for n,attr in (
            ('ear_surface','polygon_diagonals'),('semantic_eye_runtime','closure_delta'),
            ('researched_mouth_runtime','motion_delta'))}
    def test_live_stage_dispatch_before_any_native_write(self):
        payload={'status':'partial','native_ready':False}
        from test_forehead_skin_ownership import SkinOwnershipTests
        self.context['report'] = SkinOwnershipTests().report(SkinOwnershipTests().receipt())
        self.data.attributes['NormalRegion'].data = [types.SimpleNamespace(value=r) for r in (71,72,94)]
        self.data.polygons = [types.SimpleNamespace(vertices=[0], material_index=0) for _ in range(3)]
        with patch.dict(sys.modules,self.modules), \
             patch.object(stage,'regularize_forehead_author_surface',side_effect=lambda p,*a:(p + .0001,{'changed_vertices':[0]})), \
             patch.object(stage,'guard_forehead_author_poses',return_value={}), \
             patch.object(stage,'nasal_mesh_snapshot',return_value={k:{} for k in ('counts','positions','edges','polygons','loops','face_smooth','edge_flags','deform','uv','active_uv','color_settings','attributes')}), \
             patch.object(stage,'capture_forehead_source_fields',return_value={'live':True}) as capture, \
             patch.object(stage,'prepare_forehead_shared_consumer',return_value=payload) as shared, \
             patch.object(stage,'apply_forehead_retile_data') as apply, \
             patch.object(stage,'prepare_forehead_source_retile') as legacy:
            with self.assertRaises(consumer.ForeheadSharedPending):stage.stage_forehead_retile(self.data,{},self.context,-.021)
        capture.assert_called_once_with(self.data);self.assertEqual(shared.call_args.args[2],{'live':True})
        self.assertIn('pose_correspondence',shared.call_args.args[0]['author_shape_change'])
        self.assertIn('changed_points',shared.call_args.args[0]['author_shape_change'])
        raw, author = shared.call_args.args[:2]
        self.assertEqual(raw['authored_skin_ownership'], author['authored_skin_ownership'])
        self.assertEqual(raw['authored_skin_ownership']['source_face_ids'], [0,1])
        self.assertEqual(raw['vertices'].tolist(), [[0.,0.,0.]])
        self.assertGreater(float(author['vertices'][0,0]), 0.)
        apply.assert_not_called();legacy.assert_not_called();self.assertEqual(self.data.vertices[0].co,[0.,0.,0.])
    def test_detached_transaction_unwinds_without_transfer(self):
        detached=types.SimpleNamespace(shape_keys=None);self.data.copy=lambda:detached
        remove=Mock();new=Mock(side_effect=AssertionError('no BMesh transfer'))
        with patch.dict(sys.modules,{'bpy':types.SimpleNamespace(data=types.SimpleNamespace(meshes=types.SimpleNamespace(remove=remove))),
                                     'bmesh':types.SimpleNamespace(new=new)}), \
             patch.object(joint,'nasal_mesh_snapshot',return_value={}), \
             patch.object(joint,'stage_forehead_retile',side_effect=consumer.ForeheadSharedPending({'status':'partial'})):
            with self.assertRaises(consumer.ForeheadSharedPending):
                joint.refine_nasal_bridge_spacing(self.data,{},None,None,assembly_offset=-.021,forehead_context=self.context)
        remove.assert_called_once_with(detached);new.assert_not_called()
    def test_legacy_default_dispatch_unchanged(self):
        context={'calibration':{},'report':{}}
        with patch.dict(sys.modules,self.modules), \
             patch.object(stage,'regularize_forehead_author_surface',side_effect=lambda p,*a:(p,{'changed_vertices':[]})), \
             patch.object(stage,'guard_forehead_author_poses',return_value={}), \
             patch.object(stage,'prepare_forehead_source_retile',side_effect=RuntimeError('legacy reached')) as legacy, \
             patch.object(stage,'capture_forehead_source_fields') as capture:
            with self.assertRaisesRegex(RuntimeError,'legacy reached'):stage.stage_forehead_retile(self.data,{},context,-.021)
        legacy.assert_called_once();capture.assert_not_called()

if __name__=='__main__':unittest.main()
