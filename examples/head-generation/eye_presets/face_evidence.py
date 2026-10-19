"""Scope native evidence to one complete face/eye parameter snapshot."""
import hashlib
import json

FIELDS = ('eye_parameters', 'surface_parameters', 'silhouette_parameters',
          'style_profile', 'blink', 'shading_policy', 'reviewed_support_bound_m')

def configuration_sha256(spec):
    data = {name: spec[name] for name in FIELDS}
    return hashlib.sha256(json.dumps(data, sort_keys=True, separators=(',', ':'),
                                    allow_nan=False).encode()).hexdigest()

def validate_fixed_combination(spec):
    if any(name not in spec for name in FIELDS):
        raise ValueError('Missing fixed face configuration field')
    evidence = spec['evidence']
    if configuration_sha256(spec) != evidence['configuration_sha256']:
        raise ValueError('Native evidence does not match this complete face configuration')
    if evidence.get('captures_verified') != 84 or evidence.get('mouth_captures_verified') != 18:
        raise ValueError('Incomplete native expression evidence')
    if not evidence.get('width_gate_passed') or not evidence.get('visual_self_review_passed'):
        raise ValueError('Incomplete native width or visual evidence')
    if spec.get('eye_preset_id') != 'clean-downturned-810':
        raise ValueError('Unverified eye and silhouette combination')
