import ast, base64, copy, hashlib, json, unittest, zlib
from source_layout.lossless_transport import encode_sources, expanded_ast, decoder_source, MAX_LITERAL, MAX_TOTAL

def validator(r):
    n = len(json.dumps(r, ensure_ascii=False, separators=(',', ':')).encode())
    if n > 12000:
        raise ValueError('limit')
    return n

class TransportTests(unittest.TestCase):

    def setUp(self):
        self.value = '曲面 source\n"hello"\t' * 3000
        self.r = {'code': 'x = ' + repr(self.value) + '\ny = ' + repr(self.value) + '\n', 'outputs': ['model.glb'], 'timeoutSeconds': 120}

    def test_exact_sources_ast_and_fields(self):
        old = copy.deepcopy(self.r)
        out, rec = encode_sources(self.r, validator)
        self.assertEqual(self.r, old)
        self.assertLess(validator(out), 12000)
        self.assertEqual(expanded_ast(out['code'], rec), ast.dump(ast.parse(old['code']), include_attributes=False))
        self.assertEqual({k: v for k, v in out.items() if k != 'code'}, {k: v for k, v in old.items() if k != 'code'})
        ns = {}
        exec(out['code'], ns)
        self.assertEqual(ns['x'].encode(), self.value.encode())
        self.assertEqual(ns['x'], ns['y'])

    def test_verifier_rejects_actual_payload_tampering(self):
        out, rec = encode_sources(self.r, validator)
        name = rec['records'][0]['encoded']
        with self.assertRaises(ValueError):
            expanded_ast(out['code'].replace(repr(name), repr('AAAA'), 1), rec)

    def test_verifier_rejects_decoder_tampering(self):
        out, rec = encode_sources(self.r, validator)
        with self.assertRaises(ValueError):
            expanded_ast(out['code'].replace('raw.decode(', 'raw.fake_decode(', 1), rec)

    def test_fitting_identity(self):
        r = {'code': 'x=1'}
        out, rec = encode_sources(r, validator)
        self.assertIs(out, r)
        self.assertFalse(rec['changed'])

    def test_unicode_doc_future_decorator(self):
        self.r['code'] = '"模块说明"\nfrom __future__ import annotations\ndef deco(f): return f\n@deco\ndef f():\n    "函数说明"\n    return ' + repr(self.value) + '\n'
        out, rec = encode_sources(self.r, validator)
        self.assertEqual(expanded_ast(out['code'], rec), ast.dump(ast.parse(self.r['code']), include_attributes=False))

    def test_prefix_collision(self):
        self.r['code'] = '_beaver_lossless_total = 99\n' + self.r['code']
        out, rec = encode_sources(self.r, validator)
        self.assertNotEqual(rec['prefix'], '_beaver_lossless_')
        ns = {}
        exec(out['code'], ns)
        self.assertEqual(ns['_beaver_lossless_total'], 99)

    def test_nfkc_identifier_collision(self):
        self.r['code'] = '_ｂeaver_lossless_source_0 = 99\n' + self.r['code']
        out, rec = encode_sources(self.r, validator)
        ns = {}
        exec(out['code'], ns)
        self.assertEqual(ns['x'], self.value)
        self.assertEqual(ns['_beaver_lossless_source_0'], 99)

    def test_duplicate_source_assignment_rejects(self):
        out, rec = encode_sources(self.r, validator)
        name = rec['records'][0]['name']
        with self.assertRaises(ValueError):
            expanded_ast(out['code'] + '\n' + name + '=99\n', rec)

    def test_relocated_header_rejects(self):
        out, rec = encode_sources(self.r, validator)
        with self.assertRaises(ValueError):
            expanded_ast('z=1\n' + out['code'], rec)

    def test_future_annotation_string_unchanged(self):
        annotation = 'annotation ' * 1000
        r = {'code': 'from __future__ import annotations\ndef f(x: ' + repr(annotation) + '): pass\n' + self.r['code']}

        def limit(v):
            n = len(json.dumps(v).encode())
            if n > 30000:
                raise ValueError('limit')
            return n
        out, rec = encode_sources(r, limit)
        a = {}
        b = {}
        exec(r['code'], a)
        exec(out['code'], b)
        self.assertEqual(a['f'].__annotations__, b['f'].__annotations__)
        self.assertEqual(expanded_ast(out['code'], rec), ast.dump(ast.parse(r['code']), include_attributes=False))

    def test_no_effective_repair(self):
        with self.assertRaises(ValueError):
            encode_sources({'code': '#' + 'x' * 13000}, validator)

    def test_repaired_still_over_limit_reject(self):
        with self.assertRaises(ValueError):
            encode_sources(self.r, lambda r: (_ for _ in ()).throw(ValueError('alwaysreject')))

    def decoder(self):
        ns = {}
        exec(decoder_source('_t_'), ns)
        return (ns, ns['_t_inflate'])

    def test_hash_tamper(self):
        _, f = self.decoder()
        data = b'a' * 5000
        with self.assertRaises(ValueError):
            f(base64.b64encode(zlib.compress(data)).decode(), len(data), '0' * 64)

    def test_size_and_type_bounds(self):
        _, f = self.decoder()
        for n in (-1, True, MAX_LITERAL + 1, 1.5):
            with self.assertRaises(ValueError):
                f('', n, '')

    def test_encoded_bound(self):
        _, f = self.decoder()
        with self.assertRaises(ValueError):
            f('A' * (4 * MAX_LITERAL), 1, '')

    def test_malformed_base64(self):
        _, f = self.decoder()
        with self.assertRaises(ValueError):
            f('!!!', 1, '')

    def test_truncated_or_concatenated_stream(self):
        _, f = self.decoder()
        data = b'a' * 5000
        packed = zlib.compress(data)
        for p in (packed[:-1], packed + b'extra', packed + packed):
            with self.assertRaises(ValueError):
                f(base64.b64encode(p).decode(), len(data), hashlib.sha256(data).hexdigest())

    def test_bomb_expected_size(self):
        _, f = self.decoder()
        p = zlib.compress(b'a' * MAX_LITERAL)
        with self.assertRaises(ValueError):
            f(base64.b64encode(p).decode(), 1, hashlib.sha256(b'a').hexdigest())

    def test_total_bound(self):
        ns, f = self.decoder()
        ns['_t_total'] = MAX_TOTAL
        data = b'x'
        with self.assertRaises(ValueError):
            f(base64.b64encode(zlib.compress(data)).decode(), 1, hashlib.sha256(data).hexdigest())

    def test_invalid_utf8(self):
        _, f = self.decoder()
        data = b'\xff'
        with self.assertRaises(UnicodeDecodeError):
            f(base64.b64encode(zlib.compress(data)).decode(), 1, hashlib.sha256(data).hexdigest())

    def test_docstrings_not_encoded(self):
        self.r['code'] = repr(self.value) + '\nx=' + repr(self.value) + '\n'
        with self.assertRaises(ValueError):
            encode_sources(self.r, validator)
if __name__ == '__main__':
    unittest.main()

class NativeFallbackTests(unittest.TestCase):
    def test_existing_wrapper_uses_lossless_only_after_dedup(self):
        from source_layout.transport_sources import share_overflowing_native_sources, validate_native_frame
        value = 'def source_rule():\n    return "smooth face"\n' * 26000
        request = {'code': 'source = ' + repr(value) + '\n', 'outputs': ['source.py'], 'timeoutSeconds': 120}
        original = copy.deepcopy(request)
        with self.assertRaises(ValueError):
            validate_native_frame(request)
        result = share_overflowing_native_sources(request)
        self.assertLessEqual(validate_native_frame(result), 1024 * 1024)
        namespace = {}
        exec(result['code'], namespace)
        self.assertEqual(namespace['source'].encode('utf-8'), value.encode('utf-8'))
        self.assertEqual(request, original)
        self.assertEqual(result['outputs'], request['outputs'])

    def test_exact_full_frame_boundary_retains_identity(self):
        from source_layout.transport_sources import share_overflowing_native_sources, validate_native_frame
        request = {'code': '#', 'outputs': [], 'timeoutSeconds': 120}
        request['code'] += 'x' * (1024 * 1024 - validate_native_frame(request))
        self.assertEqual(validate_native_frame(request), 1024 * 1024)
        self.assertIs(share_overflowing_native_sources(request), request)
        request['code'] += 'x'
        with self.assertRaisesRegex(ValueError, 'serialized request frame'):
            share_overflowing_native_sources(request)
