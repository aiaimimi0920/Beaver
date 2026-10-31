"""Completed native journals can recover lost polling, without fake RPC wrappers."""
import json
from pathlib import Path
import tempfile
import unittest
if __package__:
    from .evidence import native_job
else:
    from evidence import native_job


class DurableJobTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name).resolve()
        self.path=self.root/'.beaver-context/external/run/request/job.json'
        self.path.parent.mkdir(parents=True)
        self.job={'jobId':'job','runId':'run','requestId':'request','status':'succeeded',
                  'process':{'workspace':str(self.root),'jobDirectory':str(self.path.parent)},
                  'result':{'success':True,'exitCode':0,'ownedTreeCleanup':'completed',
                            'termination':'exited','outputs':[{'path':'a','sha256':'abc'}]}}
        self.path.with_name('completed.json').write_text('{"completed":true}')

    def write(self):
        self.path.write_text(json.dumps(self.job))

    def test_original_completed_record_accepted_unchanged(self):
        self.write();before=self.path.read_bytes()
        job,hashes=native_job(self.path,self.root)
        self.assertEqual(job,self.job);self.assertEqual(hashes,{'a':'abc'})
        self.assertEqual(before,self.path.read_bytes())

    def test_relocated_or_linked_record_rejected(self):
        self.write();other=self.root/'copy.json';other.write_bytes(self.path.read_bytes())
        with self.assertRaises(ValueError):native_job(other,self.root)
        link=self.root/'link.json';link.symlink_to(self.path)
        with self.assertRaises(ValueError):native_job(link,self.root)

    def test_running_failed_and_incomplete_cleanup_rejected(self):
        for key,value in [('success',False),('exitCode',1),('ownedTreeCleanup','pending'),('termination','timeout')]:
            with self.subTest(key=key):
                old=self.job['result'][key];self.job['result'][key]=value;self.write()
                with self.assertRaises(ValueError):native_job(self.path,self.root)
                self.job['result'][key]=old
        self.job['status']='running';self.write()
        with self.assertRaises(ValueError):native_job(self.path,self.root)

    def test_completion_marker_and_workspace_required(self):
        self.write();self.path.with_name('completed.json').write_text('{"completed":false}')
        with self.assertRaises(ValueError):native_job(self.path,self.root)
        self.path.with_name('completed.json').write_text('{"completed":true}')
        self.job['process']['workspace']=str(self.root/'other');self.write()
        with self.assertRaises(ValueError):native_job(self.path,self.root)

    def test_sdf_binding_reads_same_original_journal(self):
        import importlib.util
        module_path=Path(__file__).parents[1]/'semantic_eyes/receipt_binding.py'
        spec=importlib.util.spec_from_file_location('durable_sdf_binding',module_path)
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        prefix='assets/aster/general_face_eyes_combo90/'
        current={prefix+name:'digest-'+name for name in
                 ('aster_head_runtime_morph.blend','aster_head.glb','semantic_parameters.json')}
        self.job['process']['scriptSha256']='script-digest'
        self.job['result']['outputs']=[{'path':k,'sha256':v} for k,v in current.items()]
        self.write()
        result=module.bind_generation_receipt(self.path,self.root,prefix,current)
        self.assertEqual(result['immutable_outputs'],current)
        self.assertEqual(result['job_id'],'job')
        self.path.with_name('completed.json').write_text('{"completed":false}')
        with self.assertRaises(ValueError):
            module.bind_generation_receipt(self.path,self.root,prefix,current)
        self.path.with_name('completed.json').write_text('{"completed":true}')
        for key in ('runId','requestId'):
            for value in (str(self.root/'unmanaged'), '../escape', 'nested/child', 'nested\\child'):
                original=self.job[key];self.job[key]=value;self.write()
                with self.subTest(key=key,value=value):
                    with self.assertRaises(ValueError):
                        module.bind_generation_receipt(self.path,self.root,prefix,current)
                    with self.assertRaises(ValueError):native_job(self.path,self.root)
                self.job[key]=original



if __name__=='__main__':unittest.main()
