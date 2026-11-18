"""Synthetic RPC fixtures ONLY: source transport tests, never native evidence."""
import copy
import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.seed_chunks import chunk_seed_jobs, assembly_code
from source_layout.source_seed import seed_retained_bundle, readable_source_literal
from source_layout.seed_evidence import verify_seed, finalize_seed_plan
from source_layout.seed_chunk_evidence import seed_receipt_sidecars
from source_layout.transport_sources import validate_native_frame
from source_layout.plan_files import write_plan_files
from source_layout.package import packaging_request
from source_layout.evidence import verify_packaging
from source_layout import test_package as fixtures
PREFIX = fixtures.PREFIX


class ChunkSeedTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve();(self.root/'asset').mkdir()
        self.text = '# ordinary readable source\n'*12000 + 'value = "中🙂"\n'
        self.runtime = "# both delimiters ''' \"\"\" and slash \\\nvalue = 2\r\n"
        self.jobs, b = chunk_seed_jobs([('asset/source.py',self.text),('asset/runtime.py',self.runtime)],readable_source_literal)
        self.binding = dict(b,path='asset/source.py',sha256=hashlib.sha256(self.text.encode()).hexdigest(),
                            runtime_path='asset/runtime.py',runtime_sha256=hashlib.sha256(self.runtime.encode()).hexdigest(),
                            native_seed_script_sha256=b['jobs'][0]['native_seed_script_sha256'])
        self.scope = {'beaver_input':lambda p:str(self.root/p),'beaver_output':lambda p:str(self.root/p)}
        self.receipts = []
        for i,request in enumerate(self.jobs):
            exec(request['code'],self.scope)
            record=b['jobs'][i];job={'jobId':f'SYNTHETIC-only-{i}','status':'succeeded',
                'process':{'workspace':str(self.root),'scriptSha256':record['native_seed_script_sha256']},
                'result':{'success':True,'exitCode':0,'outputs':[{'path':record['path'],'sha256':record['sha256']}]}}
            receipt=self.root/f'SYNTHETIC-receipt-{i}.json'
            receipt.write_bytes((json.dumps({'response':{'result':{'result':job}},'test_note':'中文'},ensure_ascii=False,indent=2)+'\r\n').encode())
            self.receipts.append(receipt)

    def test_utf8_exact_ordered_assembly_and_finalize(self):
        self.assertEqual((self.root/'asset/source.py').read_bytes(),self.text.encode())
        self.assertEqual((self.root/'asset/runtime.py').read_bytes(),self.runtime.encode())
        for job in self.jobs:self.assertLessEqual(validate_native_frame(job),1048576)
        bound=verify_seed(self.binding,self.receipts,self.root)
        self.assertEqual(len(bound['job_receipts']),len(self.jobs))
        code=f'report["authored_source_seed"] = {self.binding!r}\n'
        plan={'source_seed_binding':self.binding,'generation':{'code':code,'outputs':[]},'source_packaging':{}}
        finalized=finalize_seed_plan(plan,self.receipts,self.root)
        self.assertEqual(finalized['source_seed_binding'],bound)
        self.assertEqual(finalized['source_packaging']['source_seed_receipt_paths'],[str(p)for p in self.receipts])
        self.assertLessEqual(validate_native_frame(finalized['generation']),1048576)
        with self.assertRaisesRegex(ValueError,'already finalized'):finalize_seed_plan(finalized,self.receipts,self.root)

    def test_missing_reordered_duplicate_and_changed_receipts_reject(self):
        for rows in (self.receipts[:-1],self.receipts[::-1],[self.receipts[0]]*len(self.receipts)):
            with self.assertRaises(ValueError):verify_seed(self.binding,rows,self.root)
        bound=verify_seed(self.binding,self.receipts,self.root)
        original=self.receipts[1].read_bytes();self.receipts[1].write_bytes(original+b' ')
        with self.assertRaises(ValueError):verify_seed(bound,self.receipts,self.root)
        self.receipts[1].write_bytes(original)
        data=json.loads(original);data['response']['result']['result']['jobId']='SYNTHETIC-only-0';self.receipts[1].write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError,'Distinct'):verify_seed(self.binding,self.receipts,self.root)
        self.receipts[1].write_text(json.dumps(data['response']['result']['result']))
        with self.assertRaisesRegex(ValueError,'Original'):verify_seed(self.binding,self.receipts,self.root)

    def test_chunks_size_hash_order_duplicate_and_final_module_reject(self):
        module=self.binding['modules'][0];first=self.root/module['chunks'][0]['path'];raw=first.read_bytes();assembly=self.jobs[module['assembly_job_index']]['code']
        for damaged in (raw+b'x',b'x'+raw[1:]):
            first.write_bytes(damaged)
            with self.assertRaises(ValueError):exec(assembly,self.scope)
            with self.assertRaises(ValueError):verify_seed(self.binding,self.receipts,self.root)
        first.write_bytes(raw)
        for mutation in ('order','duplicate','hash','size'):
            bad=copy.deepcopy(module)
            if mutation=='order':bad['chunks'].reverse()
            if mutation=='duplicate':bad['chunks'][1]=bad['chunks'][0]
            if mutation=='hash':bad['sha256']='0'*64
            if mutation=='size':bad['byte_length']+=1
            with self.assertRaises(ValueError):exec(assembly_code(bad),self.scope)
        first.unlink()
        with self.assertRaises(FileNotFoundError):exec(assembly,self.scope)
        first.write_bytes(raw);(self.root/module['path']).write_bytes(b'changed = True\n')
        with self.assertRaises(ValueError):verify_seed(self.binding,self.receipts,self.root)

    def test_manifest_tampering_and_job_additions_reject(self):
        for change in ('chunks','jobs','assembly','finalhash','firstscript'):
            b=copy.deepcopy(self.binding)
            if change=='chunks':b['modules'][0]['chunks'].reverse()
            if change=='jobs':b['jobs'].append(b['jobs'][0])
            if change=='assembly':b['modules'][0]['assembly_job_index']-=1
            if change=='finalhash':b['sha256']='0'*64
            if change=='firstscript':b['native_seed_script_sha256']='0'*64
            with self.assertRaises(ValueError):verify_seed(b,self.receipts,self.root)

    def test_ordered_plan_artifact_is_not_just_first_job(self):
        plan={'source_seed':self.jobs[0],'source_seed_jobs':self.jobs,'source_seed_binding':self.binding}
        paths=write_plan_files(plan,self.root/'request.json')
        self.assertEqual(json.loads(paths['source_seed_jobs'].read_text()),self.jobs)
        with self.assertRaises(ValueError):write_plan_files(plan,self.root/'request.json')

    def test_genuine_over_limit_source_falls_back_without_runtime(self):
        text='RETAINED_UPPER_SHAPE_SOURCES = {}\nRETAINED_BROW_SOURCES = {}\n'+('# 中🙂 both quotes '+"'''"+' """\n')*60000
        code='from pathlib import Path\nimport json\nreport={}\n'+f'Path(beaver_output("asset/semantic_bundle.py")).write_text({text!r})\n'
        generation,jobs,binding=seed_retained_bundle(code,'asset/')
        self.assertEqual(binding['schema_version'],4);self.assertGreater(len(jobs),2)
        for job in jobs:
            self.assertLessEqual(validate_native_frame(job),1048576);exec(job['code'],self.scope)
        exec(generation,self.scope)
        self.assertEqual((self.root/'asset/semantic_bundle.py').read_bytes(),text.encode())
        (self.root/binding['path']).write_text(text+'# tamper\n')
        with self.assertRaisesRegex(ValueError,'seed changed'):exec(generation,self.scope)

    def test_oversized_runtime_reassembled_and_hash_checked(self):
        retained='RETAINED_UPPER_SHAPE_SOURCES = {}\nRETAINED_BROW_SOURCES = {}\n'
        runtime='# readable runtime source\n'*50000+'runtime_result = 17\n'
        code='from pathlib import Path\nimport json\nreport={}\n'+f'Path(beaver_output("asset/semantic_bundle.py")).write_text({retained!r})\n'+f'exec(compile({runtime!r},"runtime","exec"),globals())\n'
        generation,jobs,binding=seed_retained_bundle(code,'asset/')
        self.assertEqual(binding['schema_version'],4);self.assertEqual(len(binding['modules']),2)
        for job in jobs:validate_native_frame(job);exec(job['code'],self.scope)
        exec(generation,self.scope);self.assertEqual(self.scope['runtime_result'],17)
        self.assertEqual((self.root/binding['runtime_path']).read_bytes(),runtime.encode())
        (self.root/binding['runtime_path']).write_text(runtime+'# tamper\n')
        with self.assertRaisesRegex(ValueError,'Runtime source seed changed'):exec(generation,self.scope)

    def test_full_frame_includes_unicode_escaping_and_envelope(self):
        from source_layout.seed_chunks import chunk_seed_jobs
        text='# '+('🙂\\\"'*90000)+'\n'
        jobs,_=chunk_seed_jobs([('asset/unicode.py',text)],readable_source_literal)
        for job in jobs:self.assertLessEqual(validate_native_frame(job),1048576)
        bad=copy.deepcopy(jobs[0]);bad['code']+='\n#'+'🙂'*400000
        with self.assertRaisesRegex(ValueError,'frame limit'):validate_native_frame(bad)

    def test_all_original_receipts_survive_packaging_and_verify(self):
        fixture=fixtures.PackagingTests();fixture.setUp();self.addCleanup(fixture.doCleanups)
        root=fixture.root;semantic=PREFIX+'source_seed_semantic_bundle.py';runtime=PREFIX+'source_seed_runtime.py'
        jobs,b=chunk_seed_jobs([(semantic,'retained = 1\n'),(runtime,'runtime = 2\n')],readable_source_literal)
        binding=dict(b,path=semantic,sha256=b['modules'][0]['sha256'],runtime_path=runtime,runtime_sha256=b['modules'][1]['sha256'],native_seed_script_sha256=b['jobs'][0]['native_seed_script_sha256']);receipts=[]
        for i,request in enumerate(jobs):
            exec(request['code'],{'beaver_input':lambda p:str(root/p),'beaver_output':lambda p:str(root/p)})
            receipt=root/f'SYNTHETIC-chunk-{i}.json';fixture.write_receipt(receipt,f'seed-{i}',[b['jobs'][i]['path']],b['jobs'][i]['native_seed_script_sha256']);receipt.write_bytes(receipt.read_bytes()+b'\r\n');receipts.append(receipt)
        bound=verify_seed(binding,receipts,root);packing=fixture.plan['source_packaging'];packing['source_seed_binding']=bound;packing['source_seed_receipt_paths']=[str(p)for p in receipts]
        retained=PREFIX+'semantic_bundle.py';(root/retained).write_bytes((root/semantic).read_bytes());packing['generation_outputs'].append(retained);fixture.report['authored_source_seed']=bound;(root/PREFIX/'generation_report.json').write_text(json.dumps(fixture.report));fixture.write_receipt(fixture.gen,'gen',packing['generation_outputs'],packing['generation_script_sha256'])
        fixture.write_receipt(fixture.gen,'seed-2',packing['generation_outputs'],packing['generation_script_sha256'])
        with self.assertRaisesRegex(ValueError,'distinct'):packaging_request(packing,fixture.gen,root)
        fixture.write_receipt(fixture.gen,'gen',packing['generation_outputs'],packing['generation_script_sha256'])
        request=packaging_request(packing,fixture.gen,root);exec(request['code'],{'beaver_input':lambda p:str(root/p),'beaver_output':lambda p:str(root/p)})
        packed=root/'SYNTHETIC-pack.json';fixture.write_receipt(packed,'pack',[r['path']for r in request['outputs']]);verify_packaging(fixture.report,root,PREFIX,fixture.gen,packed)
        for (name,digest),receipt in zip(seed_receipt_sidecars(bound),receipts):
            sidecar=root/PREFIX/name;self.assertEqual(sidecar.read_bytes(),receipt.read_bytes());self.assertEqual(hashlib.sha256(sidecar.read_bytes()).hexdigest(),digest)
        fixture.write_receipt(packed,'seed-3',[r['path']for r in request['outputs']])
        with self.assertRaisesRegex(ValueError,'chain'):verify_packaging(fixture.report,root,PREFIX,fixture.gen,packed)
        fixture.write_receipt(packed,'pack',[r['path']for r in request['outputs']])
        sidecar.write_bytes(sidecar.read_bytes()+b' ')
        with self.assertRaises(ValueError):verify_packaging(fixture.report,root,PREFIX,fixture.gen,packed)

if __name__=='__main__':unittest.main()
