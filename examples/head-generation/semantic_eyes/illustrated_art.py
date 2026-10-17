"""Opt-in original eye paint and pocket atlas; immutable checkpoint stays intact."""


def once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Illustrated eye source contract changed: " + old[:70])
    return source.replace(old, new, 1)


IRIS = """def iris_color(u, v):
    x, y = (u - 0.5) * 2, (v - 0.5) * 2
    r = sqrt(x*x + y*y)
    # Broad graphic value groups, with the lid-facing half deliberately dark.
    t = max(0., min(1., (0.5-y)/1.35))
    t = t*t*(3-2*t)
    top, bottom = (0.018,0.052,0.068), (0.22,0.64,0.47)
    c = [top[i]*(1-t)+bottom[i]*t for i in range(3)]
    crescent = exp(-((r-0.70)/0.14)**2)*max(0.,min(1.,(-y-0.10)/0.55))
    glow = (0.52,0.85,0.57)
    c = [c[i]*(1-0.72*crescent)+glow[i]*0.72*crescent for i in range(3)]
    # Restrained radial detail supports, rather than replaces, the large shapes.
    fibers = 0.008*sin(35*atan2(y,x)+5*r)*max(0.,1-abs(y))*min(1.,r*2)
    c = [max(0.,min(1.,v+fibers)) for v in c]
    edge = max(0.,min(1.,(r-0.88)/0.10))
    c = [c[i]*(1-edge)+(0.016,0.045,0.05)[i]*edge for i in range(3)]
    # Secondary reflection is smaller and dimmer than the independent key glint.
    d = sqrt(((x-0.38)/0.10)**2+((y+0.42)/0.075)**2)
    glint = max(0.,min(1.,(1-d)/0.25))*0.48
    c = [c[i]*(1-glint)+(0.63,0.86,0.83)[i]*glint for i in range(3)]
    return (*c,1)


def sclera_art_color(u,v):
    t = (u-0.5)*2.6
    y = Y0+H*(C["eye_corner_y"]+(v-0.5)*0.50)
    _, upper = eye_contour(1,max(-0.999,min(0.999,t)),True)
    # Painted upper occlusion follows the actual aperture, not a screen overlay.
    shadow = max(0.,min(1.,1-(upper-y)/(H*0.045))) * 0.32
    corner = exp(-((abs(t)-0.94)/0.16)**2)*0.13
    c = [WHITE[i]*(1-shadow)+(0.45,0.43,0.48)[i]*shadow for i in range(3)]
    c = [c[i]*(1-corner)+(0.82,0.56,0.58)[i]*corner for i in range(3)]
    return (*c,1)


"""


SKIN_HOLDOUT = """def ocular_skin_holdout(u, v):
    # Experimental author-controlled skin outline mask, in the spatial skin island.
    # UVs stay bound to the neutral surface during blink; no geometry is changed.
    if not STYLE.get("ocular_skin_outline_holdout", False):
        return 0.0
    if not (0.02 < u < 0.55 and 0.02 < v < 0.98):
        return 0.0
    x = ((u - 0.02) / 0.53 - 0.5) * 0.235
    y = 1.5 + (v - 0.02) / 0.96 * 0.31
    t = (abs(x) / H - C["eye_center_x"]) / C["eye_half_width"]
    center = Y0 + H * (C["eye_corner_y"] + C["eye_corner_slope"] * t)
    rx = 1.08 * H * C["eye_half_width"]
    arch = C["eye_upper_arch"] if y >= center else C["eye_lower_arch"]
    ry = H * arch + 0.002
    q = sqrt(((abs(x) - H*C["eye_center_x"])/rx)**2 + ((y-center)/ry)**2)
    if q <= 1.0:
        return 1.0
    if q >= 1.08:
        return 0.0
    f = (q - 1.0) / 0.08
    # Current shader only scales partial B widths when lip-fix is enabled.
    # With its default disabled, only the full-B plateau removes the outline;
    # this transition is authored data, not a claim of smooth rendered taper.
    return 1.0 - f*f*(3.0 - 2.0*f)


"""


def adapt_sources(original):
    sources = dict(original)
    texture = sources["textures.py"]
    start, end = texture.index("def iris_color("), texture.index("def eye_atlas_color(")
    texture = texture[:start] + IRIS + SKIN_HOLDOUT + texture[end:]
    texture = once(
        texture,
        '            if role == "Face" and 0.6 < u < 0.95 and 0.86 < v < 0.89:',
        """            if role == "Face" and 0.6 < u < 0.95 and 0.405 < v < 0.495:
                pixels.extend(sclera_art_color((u-0.6)/0.35,(v-0.405)/0.09))
                continue
            if role == "Face" and 0.6 < u < 0.95 and 0.86 < v < 0.89:""",
    )
    texture = once(
        texture,
        "            eye_pixel = (flat_region and eye[slot] or iris_island) and (not ear_island)",
        "            sclera_island = 0.6 < u < 0.95 and 0.405 < (py + 0.5) / 256 < 0.495\n"
        "            eye_pixel = (flat_region and eye[slot] or iris_island or sclera_island) and (not ear_island)",
    )
    texture = once(
        texture,
        "            makeup.extend((0, 1 if flat_region and lip[slot] else 0, 0, 1))",
        "            ink_island = 0.6 < u < 0.95 and 0.86 < (py + 0.5) / 256 < 0.89\n"
        "            holdout = flat_region or iris_island or sclera_island or ink_island\n"
        "            makeup.extend((0, 1 if flat_region and lip[slot] else 0, max(1 if holdout else 0, ocular_skin_holdout(u, (py + 0.5) / 256)), 1))",
    )
    sources["textures.py"] = texture
    sources["pipeline.py"] = once(
        sources["pipeline.py"],
        '            if o.get("iris_detail", False):',
        """            if o.name.startswith("Expanding recessed eye pocket "):
                t = (abs(v.x)/H-C["eye_center_x"])/C["eye_half_width"]
                u = 0.60+0.35*max(0.002,min(0.998,t/2.6+0.5))
                sy = ((v.z-Y0)/H-C["eye_corner_y"])/0.50+0.5
                vv = 0.405+0.09*max(0.002,min(0.998,sy))
            if o.get("iris_detail", False):""",
    )
    sources["pipeline.py"] = once(
        sources["pipeline.py"],
        'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)',
        'definition = definition.replace(\'face_map = ExtResource("11")\', \'face_map = ExtResource("11")\\nface_outline_control = ExtResource("11")\')\n'
        'Path(beaver_output(OUT + "aster_definition.tres")).write_text(definition)',
    )
    sources["eye_socket.py"] = once(
        sources["eye_socket.py"], "(0.012, 0.014, 0.024)", "(0.018, 0.055, 0.052)"
    )
    for name in ("textures.py", "pipeline.py", "eye_socket.py"):
        compile(sources[name], name, "exec")
    return sources
