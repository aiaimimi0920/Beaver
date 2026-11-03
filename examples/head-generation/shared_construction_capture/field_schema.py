"""Exact, stage-qualified field inventories for the private footprint protocol."""
import math
from joint_binding import digest, require

DOMAINS={'point','face','edge','corner','uv'}


def typed(value, kind):
    if kind=='bool':
        return type(value) is bool
    if kind=='int':
        return type(value) is int
    if kind=='string':
        return isinstance(value,str)
    if kind=='float':
        return type(value) in (int,float) and math.isfinite(value)
    if kind in ('float2','float3','float4'):
        return isinstance(value,(list,tuple)) and len(value)==int(kind[-1]) and all(typed(v,'float') for v in value)
    return False


def inventory(schema, invocation):
    require(schema['schema']=='complete-stage-fields-v1', 'Unknown field schema')
    require(schema['stage_id']==invocation['stage_id'], 'Field schema stage mismatch')
    require(digest(schema)==invocation['field_schema_sha256'], 'Field schema authority mismatch')
    require(set(schema['domains'])==DOMAINS, 'Missing field schema domain')
    for fields in schema['domains'].values():
        require(isinstance(fields,dict) and bool(fields), 'Empty field schema domain')
        require(all(k in ('bool','int','string','float','float2','float3','float4') for k in fields.values()),
                'Unsupported field type; explicit capture required')
    return schema['domains']


def fields(values, expected, domain):
    require(isinstance(values,dict) and set(values)==set(expected),
            'Incomplete or unexpected '+domain+' fields')
    require(all(typed(values[k],kind) for k,kind in expected.items()),
            'Wrong '+domain+' field type')
