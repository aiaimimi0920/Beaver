from face import *
from math import atan2


def save_map(path, pixels, w=512, h=256):
    im = bpy.data.images.new(path.rsplit("/", 1)[-1], width=w, height=h, alpha=True)
    im.colorspace_settings.name = "Non-Color"
    im.pixels = pixels
    im.filepath_raw = beaver_output(path)
    im.file_format = "PNG"
    im.save()


def bake_control_maps(out):
    parts = [o for o in PARTS if o["npr_role"] == "Face"]
    colors = []
    eye = []
    lip = []
    for o in parts:
        c = tuple(o.data.color_attributes["Color"].data[0].color)
        if c not in colors:
            colors.append(c)
            eye.append(
                o.get("ocular_surface", False)
                or "sclera" in o.name
                or any((k in o.name for k in ["Iris", "Pupil", "Catchlight"]))
            )
            lip.append("lip line" in o.name)
    ilm = []
    makeup = []
    hair = []
    for py in range(256):
        y = 1.5 + 0.31 * (py + 0.5) / 256
        for px in range(512):
            u = (px + 0.5) / 512
            x = (u - 0.5) * 0.235
            e = 0.0002
            zz = (depth(x + e, y) - depth(x - e, y)) / (2 * e)
            nose_slope = (nose(x + e, y) - nose(x - e, y)) / (2 * e)
            nose_weight = STYLE["nose_sdf_scale"] * (0.6 + 0.4 * exp(-(((y - 1.61) / 0.009) ** 2)))
            zz -= nose_slope * (1 - nose_weight)
            threshold = max(0.025, min(0.975, (atan2(-zz, 1) + pi / 2) / pi))
            flat_region = 0.6 < u < 0.95 and (py + 0.5) / 256 > 0.9
            slot = max(0, min(len(colors) - 1, int((u - 0.6) / 0.35 * len(colors))))
            iris_island = 0.6 < u < 0.95 and 0.05 < (py + 0.5) / 256 < 0.4
            ear_island = 0.6 < u < 0.95 and 0.5 < (py + 0.5) / 256 < 0.85
            eye_pixel = (flat_region and eye[slot] or iris_island) and (not ear_island)
            ilm.extend((0.5 if eye_pixel else 1, 1 if eye_pixel else 0, 0, threshold))
            makeup.extend((0, 1 if flat_region and lip[slot] else 0, 0, 1))
            hair.extend((0.55, 0.5, 0.03, 0))
    save_map(out + "face_ilm.png", ilm)
    save_map(out + "face_map.png", makeup)
    save_map(out + "hair_ilm.png", hair)


def iris_color(u, v):
    x = (u - 0.5) * 2
    y = (v - 0.5) * 2
    r = sqrt(x * x + y * y)
    a = atan2(y, x)
    shade = 0.2 + 0.7 * max(0, min(1, (0.65 - y) / 1.4))
    fibers = 0.016 * sin(43 * a + 7 * r) + 0.009 * sin(91 * a - 11 * r)
    bright = shade + fibers * min(1, max(0, (r - 0.28) * 3))
    contrast = STYLE["iris_depth_contrast"] * max(-0.75, min(0.75, -y))
    c = [
        max(0, min(1, STYLE["iris_dark_rgb"][i] + STYLE["iris_light_rgb"][i] * bright + contrast))
        for i in range(3)
    ]
    crescent = exp(-(((r - 0.65) / 0.17) ** 2)) * max(0, min(1, (-y - 0.05) / 0.65))
    c = [
        c[i] * (1 - 0.55 * crescent) + STYLE["iris_crescent_rgb"][i] * 0.55 * crescent
        for i in range(3)
    ]
    rim = exp(-(((r - 0.77) / 0.025) ** 2)) * 0.11
    c = [c[i] * (1 - rim) for i in range(3)]
    if r > 0.84:
        t = min(1, (r - 0.84) / 0.15)
        c = [c[i] * (1 - t) + STYLE["iris_rim_rgb"][i] * t for i in range(3)]
    pupil = sqrt((x / STYLE["iris_pupil_aspect"]) ** 2 + y * y)
    if pupil < STYLE["iris_pupil_radius"]:
        t = max(
            0,
            min(
                1,
                (pupil - STYLE["iris_pupil_radius"] + STYLE["iris_pupil_softness"])
                / STYLE["iris_pupil_softness"],
            ),
        )
        c = [c[i] * t + [0.005, 0.014, 0.014][i] * (1 - t) for i in range(3)]
    for gx, gy, rad, strength in [(-0.2, 0.18, 0.047, 0.9), (0.27, -0.34, 0.024, 0.55)]:
        d = sqrt((x - gx) ** 2 + (y - gy) ** 2)
        amount = max(0, min(1, (rad - d) / 0.018)) * strength
        c = [c[i] * (1 - amount) + [0.98, 1.0, 0.96][i] * amount for i in range(3)]
    disc = sqrt((x / 0.53) ** 2 + ((y + 0.53) / 0.42) ** 2)
    fill = STYLE["iris_graphic_disc_strength"] * max(0, min(1, (1 - disc) / 0.15))
    c = [c[i] * (1 - fill) + [0.58, 0.53, 0.63][i] * fill for i in range(3)]
    outline = 0.16 * exp(-(((disc - 1) / 0.055) ** 2))
    c = [c[i] * (1 - outline) for i in range(3)]
    accent = max(0, min(1, (1 - sqrt((x / 0.095) ** 2 + ((y - 0.18) / 0.16) ** 2)) / 0.2)) * 0.7
    c = [c[i] * (1 - accent) + STYLE["iris_accent_rgb"][i] * accent for i in range(3)]
    return (*c, 1)


def eye_atlas_color(u, v):
    x = (u - 0.5) * (2 * STYLE["eye_half_width_m"])
    y = (v - 0.5) * 0.044
    iu = x / (2 * STYLE["iris_half_width_m"]) + 0.5
    iv = y / 0.036 + 0.5
    radius = sqrt(((iu - 0.5) * 2) ** 2 + ((iv - 0.5) * 2) ** 2)
    if radius >= 1:
        return (*WHITE, 1)
    return iris_color(iu, iv)


def ear_color(u, v):
    x = (u - STYLE["ear_arch_center_u"]) / STYLE["ear_arch_radius_u"]
    y = (v - STYLE["ear_arch_center_v"]) / STYLE["ear_arch_radius_v"]
    radius = sqrt(x * x + y * y)
    concha = STYLE["ear_concha_tint_strength"] * exp(
        -(((u - STYLE["ear_concha_center_u"]) / 0.20) ** 2)
        - ((v - STYLE["ear_concha_center_v"]) / 0.20) ** 2
    )
    arch_mask = max(0, min(1, (v - (STYLE["ear_arch_center_v"] - 0.04)) / 0.08))
    helix = (
        STYLE["ear_helix_stroke_strength"]
        * exp(-(((radius - 0.80) / STYLE["ear_helix_softness"]) ** 2))
        * arch_mask
    )
    fold_u = STYLE["ear_fold_center_u"] + 0.065 * sin(pi * max(0, min(1, (v - 0.38) / 0.40)))
    fold_u -= STYLE["ear_fold_curl"] * exp(-(((v - 0.39) / 0.075) ** 2))
    fold = (
        STYLE["ear_inner_fold_strength"]
        * exp(-(((u - fold_u) / STYLE["ear_fold_width"]) ** 4))
        * exp(-(((v - 0.57) / 0.18) ** 4))
    )
    fork_t = max(0, min(1, (v - 0.62) / 0.18))
    fork_u = STYLE["ear_fold_center_u"] + 0.18 * fork_t * fork_t
    fork = (
        STYLE["ear_inner_fork_strength"]
        * exp(-(((u - fork_u) / 0.028) ** 2))
        * exp(-(((v - 0.70) / 0.10) ** 4))
    )
    lobe_mask = max(0, min(1, (v - 0.24) / 0.18))
    strength = max(0, min(0.65, concha + helix + fold + fork)) * lobe_mask
    pigment = (0.79, 0.43, 0.37)
    return tuple(SKIN[i] * (1 - strength) + pigment[i] * strength for i in range(3)) + (1,)


def bake_base_atlas(role, parts, colors, out):
    w, h = (1024, 1024) if role == "Face" else (1024, 512)
    image = bpy.data.images.new(role + " detailed atlas", width=w, height=h, alpha=True)
    pixels = []
    for iy in range(h):
        v = (iy + 0.5) / h
        for ix in range(w):
            u = (ix + 0.5) / w
            slot = min(len(colors) - 1, int(u * len(parts)))
            c = colors[slot]
            if role == "Face":
                slot = max(0, min(len(colors) - 1, int((u - 0.6) / 0.35 * len(colors))))
                c = colors[slot]
            if role == "Face" and 0.6 < u < 0.95 and (0.05 < v < 0.4):
                pixels.extend(eye_atlas_color((u - 0.6) / 0.35, (v - 0.05) / 0.35))
                continue
            if role == "Face" and 0.6 < u < 0.95 and (0.5 < v < 0.85):
                eu = (u - 0.6) / 0.35
                ev = (v - 0.5) / 0.35
                c = ear_color(eu, ev)
                pixels.extend(c)
                continue
            if role == "Hair" and 0.55 < u < 0.98 and (0.05 < v < 0.95):
                a = (u - 0.55) / 0.43 * 2 * pi
                t = (v - 0.05) / 0.9
                flow = a + 0.05 * sin(pi * t)
                ridge = 0.025 * cos(14 * flow) + 0.012 * cos(29 * flow)
                shade = 0.94 + 0.04 * sin(pi * t) + ridge * sin(pi * t) ** 0.5
                c = tuple((colors[0][i] * shade for i in range(3))) + (1,)
                pixels.extend(c)
                continue
            if role == "Hair":
                across = u * len(parts) % 1
                band = exp(-(((v - (0.6 + 0.045 * sin(pi * across))) / 0.055) ** 2))
                shade = (0.86 + 0.14 * sin(pi * v * 0.65)) * (
                    0.89 + 0.11 * sin(pi * across) ** 0.45
                ) + 0.035 * band
                c = (c[0] * shade, c[1] * shade, c[2] * shade, c[3])
            elif 0.02 < u < 0.55 and 0.02 < v < 0.98:
                c = (*SKIN, 1)
                x = ((u - 0.02) / 0.53 - 0.5) * 0.235
                y = 1.5 + (v - 0.02) / 0.96 * 0.31
                blush = 0.08 * exp(-(((abs(x) - 0.068) / 0.022) ** 2) - ((y - 1.606) / 0.018) ** 2)
                c = tuple(
                    (c[i] * (1 - blush) + [0.97, 0.49, 0.45][i] * blush for i in range(3))
                ) + (1,)
                t = (abs(x) / H - C["eye_center_x"]) / C["eye_half_width"]
                if abs(t) < 0.93:
                    _, lid_y = eye_contour(1, t, True)
                    crease_y = lid_y + 0.027 * H * max(0, 1 - t * t) ** 0.65
                    crease = (
                        STYLE["lid_crease_strength"]
                        * exp(-(((y - crease_y) / (0.0028 * H)) ** 2))
                        * max(0, 1 - t * t)
                    )
                    c = tuple(
                        (c[i] * (1 - crease) + [0.7, 0.31, 0.33][i] * crease for i in range(3))
                    ) + (1,)
                ny = (y - Y0 - H * STYLE["nose_plane_bottom_height"]) / (
                    H * STYLE["nose_plane_height"]
                )
                nx = (x + 0.001) / (H * STYLE["nose_plane_width"])
                signed = min(ny, 1 - ny, nx, ny - nx)
                accent = STYLE["nose_plane_tint_strength"] * max(0, min(1, signed / 0.1))
                c = tuple(
                    (c[i] * (1 - accent) + [0.76, 0.47, 0.43][i] * accent for i in range(3))
                ) + (1,)
                tip_tint = 0.045 * exp(-((x / 0.0038) ** 2) - ((y - 1.614) / 0.0055) ** 2)
                c = tuple(
                    (c[i] * (1 - tip_tint) + [0.91, 0.56, 0.49][i] * tip_tint for i in range(3))
                ) + (1,)
            pixels.extend(c)
    image.pixels = pixels
    image.filepath_raw = beaver_output(out + role.lower() + "_base.png")
    image.file_format = "PNG"
    image.save()
    image.filepath = "//" + role.lower() + "_base.png"
    return image
