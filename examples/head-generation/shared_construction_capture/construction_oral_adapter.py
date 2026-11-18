"""Guarded opt-in oral producer and donor boundaries; default source stays untouched."""
from construction_instrument import function_edit, replace


def instrument_oral(sources):
    out = dict(sources)
    out['shell.py'] = function_edit(out['shell.py'], 'build_face', [
        ('            build_oral_cavity([verts[i] for i in rings[0]])',
         '            from construction_oral_native import begin_oral\n'
         '            build_oral_cavity([verts[i] for i in rings[0]], construction=begin_oral(_construction, rings[0]))')])
    out['mouth.py'] = function_edit(out['mouth.py'], 'build_oral_cavity', [
        ('def build_oral_cavity(half_edge):', 'def build_oral_cavity(half_edge, construction=None):'),
        ('    # Closed ellipsoid tongue: regular bands, poles hidden at the rear/bottom.',
         '    from construction_oral_native import bind_oral\n'
         '    bind_oral(oral, construction, rings, vs, fs)\n'
         '    # Closed ellipsoid tongue: regular bands, poles hidden at the rear/bottom.')])
    return out


def donor_boundary(pipeline):
    return replace(pipeline,
        '        report["oral_skin_point_ownership"] = restore_oral_skin_donors(members[0],_oral_skin_donors)',
        '        from construction_oral_fields import verify_oral_transfer\n'
        '        report["oral_skin_point_ownership"] = verify_oral_transfer(\n'
        '            members[0], _construction_joined, _oral_skin_donors, restore_oral_skin_donors)')
