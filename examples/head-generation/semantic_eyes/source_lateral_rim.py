"""Opt-in initial side-lash guide on the selected aperture, before local seating."""


def adapt_lateral_rim(source, enabled):
    if not enabled:
        return source
    old_import = "from lateral_fit import regularize_lateral_sections, seat_outer_wings"
    old_body = '''    t = curve(f,1,STYLE["accent_lateral_center_profile"])
    x = side*H*(C["eye_center_x"]+C["eye_half_width"]*t+offset)
    return x, top*(1-f)+bottom*f'''
    new_body = '''    y = top*(1-f)+bottom*f
    corner = Y0+H*(C["eye_corner_y"]+C["eye_corner_slope"])
    guide_y = corner*(1-f)+bottom*f
    x = side*(eye_rim_x_at_height(guide_y, C)+H*offset)
    return x, y'''
    for old, new in ((old_import, old_import + ", eye_rim_x_at_height"),
                     (old_body, new_body)):
        if source.count(old) != 1:
            raise ValueError("Lateral rim source contract changed")
        source = source.replace(old, new, 1)
    old_return = "    return (result[0],result[1]*(1-blend)-forward_depth*blend,result[2])"
    new_return = '    value = (result[0],result[1]*(1-blend)-forward_depth*blend,result[2])\n    # The upper join has changed z. Build the new guide at that actual height.\n    phase = max(0.0,min(1.0,f/.20))\n    weight = phase*phase*(3-2*phase)\n    anchor_x = side*eye_rim_x_at_height(value[2],C)\n    target_x = anchor_x+side*H*offset\n    target_depth = lateral_attached_depth(target_x,value[2],anchor_x)+STYLE["accent_lateral_thickness_m"]+STYLE["accent_surface_clearance_m"]+relief\n    return (value[0]*(1-weight)+target_x*weight,\n            value[1]*(1-weight)-target_depth*weight,value[2])'
    if source.count(old_return) != 1:
        raise ValueError("Final lateral position contract changed")
    return source.replace(old_return, new_return, 1)


def adapt_lateral_depth(source, enabled):
    if not enabled:
        return source
    replacements = [
        ("'Final lateral guide requires structural redesign'",
         "('Final lateral guide requires structural redesign', f, base, target, delta)"),
        ("        shifts=depth_shifts(stations,centers)",
         "        assert len(stations)==len(centers) and len(stations)>1\n"
         "        assert stations[0]==0 and stations[-1]==1\n"
         "        shifts=[0.0 for _ in centers]  # initial chord shaping disabled"),
        ('"Lateral seating requires structural redesign"',
         '("Lateral seating requires structural redesign", base, hit, normal, phase, result)'),
        ('"maximum_section_shift_m":max(abs(d) for d in shifts)',
         '"initial_chord_shaping_applied":False,"maximum_section_shift_m":max(abs(d) for d in shifts)'),
    ]
    for old, new in replacements:
        if source.count(old) != 1:
            raise ValueError("Lateral depth source contract changed")
        source = source.replace(old, new, 1)
    return source
