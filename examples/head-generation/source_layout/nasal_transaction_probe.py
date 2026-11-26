"""Temporary native test wrapper, loaded by Beaver's existing Blender job only.

Usage: exec(this_source, audit_namespace); audit_namespace['install'](shell)
No output mesh is manually patched: deliberate failures must restore the source;
the third invocation performs the ordinary shared refinement that is exported.
"""

def install(shell):
    import bpy
    import bmesh
    import functools
    original = shell.refine_nasal_source
    if getattr(original, '_nasal_fault_test_installed', False):
        raise ValueError('Nasal transaction fault test already installed')
    namespace = original.__globals__
    snapshot = namespace['nasal_mesh_snapshot']
    equal = namespace['assert_nasal_mesh_snapshot_equal']

    def mesh_ids():
        return {int(x.as_pointer()) for x in bpy.data.meshes}

    def no_mesh_leak(before, label):
        if mesh_ids() != before:
            raise AssertionError(label + ' leaked or removed a Mesh datablock')

    def key_isolation(incoming):
        baseline = mesh_ids()
        source, owner, staged = incoming, None, None
        synthetic = incoming.shape_keys is None
        try:
            if synthetic:
                source = bpy.data.meshes.new('NasalAudit_KeyIsolation_TEMP')
                source.from_pydata([(0,0,0),(1,0,0),(1,0,1),(0,0,1)],[],[(0,1,2,3)])
                owner = bpy.data.objects.new('NasalAudit_KeyIsolation_TEMP', source)
                owner.shape_key_add(name='Basis',from_mix=False)
                key = owner.shape_key_add(name='AuditShift',from_mix=False)
                key.data[0].co.y += 0.01
            before = snapshot(source)
            staged = source.copy()
            if (staged.shape_keys is None or
                    staged.shape_keys.as_pointer() == source.shape_keys.as_pointer()):
                raise AssertionError('Mesh.copy shared or omitted Key storage')
            equal(before,snapshot(staged),'Key isolation initial copy')
            block = staged.shape_keys.key_blocks[-1]
            if not len(block.data):
                raise AssertionError('No shape-key coordinate available for isolation test')
            block.data[0].co.x += 0.125
            if snapshot(staged) == before:
                raise AssertionError('Key isolation sentinel mutation did not take effect')
            equal(before,snapshot(source),'Original after staged Key mutation')
            result = {'verified': True, 'source': 'synthetic_keyed_mesh' if synthetic else 'incoming_mesh',
                      'independent_key_ids': True, 'staged_key_write_preserved_original': True}
        finally:
            if staged is not None:
                bpy.data.meshes.remove(staged)
            if owner is not None:
                bpy.data.objects.remove(owner)
            if synthetic and source is not incoming:
                bpy.data.meshes.remove(source)
        no_mesh_leak(baseline,'Key isolation')
        return result

    @functools.wraps(original)
    def audited(data, ordinals, authored):
        before, baseline = snapshot(data), mesh_ids()
        proof = {'key_copy_isolation': key_isolation(data)}
        equal(before,snapshot(data),'Original after Key isolation test')
        validator = namespace['validate_nasal_joint_domain']
        preflight_error = RuntimeError('NASAL_TEST_INJECTED_PREFLIGHT_FAILURE')
        preflight_calls = 0
        def reject_preflight(*args, **kwargs):
            nonlocal preflight_calls
            preflight_calls += 1
            raise preflight_error
        namespace['validate_nasal_joint_domain'] = reject_preflight
        try:
            try:
                original(data,ordinals,authored)
            except Exception as error:
                if error is not preflight_error:
                    raise
            else:
                raise AssertionError('Expected preflight sentinel was not raised')
        finally:
            namespace['validate_nasal_joint_domain'] = validator
        if preflight_calls != 1:
            raise AssertionError('Unexpected preflight callback count')
        equal(before,snapshot(data),'Original after injected preflight rejection')
        no_mesh_leak(baseline,'Injected preflight rejection')
        proof['preflight_rejection'] = {'sentinel_caught': True, 'original_payload_unchanged': True,
                                        'no_staged_mesh_leak': True}

        real_new = bmesh.new
        transfer_error = RuntimeError('NASAL_TEST_INJECTED_AFTER_REAL_FINAL_TRANSFER')
        original_pointer = int(data.as_pointer())
        original_writes = 0
        injected_after_native_write = False
        class BMeshProxy:
            def __init__(self, native):
                self.native = native
            def __getattr__(self, name):
                return getattr(self.native,name)
            def to_mesh(self, target):
                nonlocal original_writes, injected_after_native_write
                result = self.native.to_mesh(target)
                if int(target.as_pointer()) == original_pointer:
                    original_writes += 1
                    if original_writes == 1:
                        injected_after_native_write = True
                        raise transfer_error
                return result
        bmesh.new = lambda *args, **kwargs: BMeshProxy(real_new(*args, **kwargs))
        try:
            try:
                original(data,ordinals,authored)
            except Exception as error:
                if error is not transfer_error:
                    raise
            else:
                raise AssertionError('Expected final-transfer sentinel was not raised')
        finally:
            bmesh.new = real_new
        if not injected_after_native_write or original_writes != 2:
            raise AssertionError('Expected one real commit write and one native rollback write')
        equal(before,snapshot(data),'Original after injected final-transfer failure')
        no_mesh_leak(baseline,'Injected final-transfer failure')
        proof['post_write_rollback'] = {'sentinel_caught': True, 'injected_after_native_write': True,
            'original_target_writes': original_writes, 'original_payload_unchanged': True,
            'no_staged_mesh_leak': True}

        result = original(data,ordinals,authored)
        no_mesh_leak(baseline,'Normal transaction')
        if not result.get('nasal_joint_transaction',{}).get('committed'):
            raise AssertionError('Final ordinary transaction did not commit')
        result['nasal_transaction_fault_injection'] = proof
        return result
    audited._nasal_fault_test_installed = True
    shell.refine_nasal_source = audited
    return {'installed': True, 'entry_point': 'refine_nasal_source'}
