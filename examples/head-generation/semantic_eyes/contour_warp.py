"""Shared optional upper-arch coordinate warp; physical eye layout is unchanged."""


def angular_coordinate(t, amount):
    if not amount or abs(t) >= 1:
        return t
    u = abs(t)
    q = (.30/.55)*u if u <= .55 else .30+(.70/.45)*(u-.55)
    return (1 if t >= 0 else -1)*((1-amount)*u+amount*q)


def angular_derivative(t, amount):
    if not amount or abs(t) >= 1:
        return 1.0
    # At shoulders use the outer segment's one-sided derivative.
    slope = .30/.55 if abs(t) < .55 else .70/.45
    return (1-amount)+amount*slope


def eye_warp(t, c):
    a = c.get("eye_upper_asymmetry", 0.0)
    g = c.get("eye_upper_angularity", 0.0)
    if not (a or g) or abs(t) >= 1:
        return t
    u = angular_coordinate(t, g)
    return u + a*(1-u*u)**3


def eye_warp_derivative(t, c):
    a = c.get("eye_upper_asymmetry", 0.0)
    g = c.get("eye_upper_angularity", 0.0)
    if not (a or g) or abs(t) >= 1:
        return 1.0
    u = angular_coordinate(t, g)
    return angular_derivative(t,g)*(1-6*a*u*(1-u*u)**2)


def upper_shape(t, c):
    u = eye_warp(t, c)
    return max(0.0, 1 - abs(u) ** c["eye_horizontal_power"]) ** c["eye_upper_power"]


def upper_shape_derivative(t, c):
    u = eye_warp(t, c)
    if abs(u) >= 1:
        raise ValueError(
            "Upper profile derivative is singular at the aperture endpoints"
        )
    if u == 0:
        return 0.0
    h, p = c["eye_horizontal_power"], c["eye_upper_power"]
    return (
        -p
        * h
        * (1 if u > 0 else -1)
        * abs(u) ** (h - 1)
        * (1 - abs(u) ** h) ** (p - 1)
        * eye_warp_derivative(t, c)
    )


def lower_coordinate(t, c):
    """Symmetric shoulder warp. For 0<=a<=1, derivative >=1/9.

    q'=1+3*a*u*(1-u)^2*(1-3*u), u=t^2. On [0,1],
    u*(1-u)^2<=4/27 and (1-3*u)>=-2, hence q'>=1-8*a/9.
    Identity outside the aperture; center and endpoint derivatives stay one.
    """
    a = c.get("eye_lower_shoulder", 0.0)
    if not a or abs(t) >= 1:
        return t
    return t + a * t**3 * (1 - t * t) ** 3


def lower_shape(t, c):
    q = lower_coordinate(t, c)
    return (
        max(0.0, 1 - abs(q) ** c["eye_lower_horizontal_power"]) ** c["eye_lower_power"]
    )


def upper_change(t, calibration, parameters):
    arch = (
        max(0.0, 1 - abs(t) ** calibration["eye_horizontal_power"])
        ** calibration["eye_upper_power"]
    )
    taper = parameters["aperture_corner_taper"]
    upper_power = calibration["eye_upper_power"] * parameters["upper_roundness"]
    upper_power = ((1 - taper) * upper_power + taper) * (
        1 - 0.3 * parameters["upper_corner_softness"]
    )
    new_arch = (
        max(
            0.0,
            1
            - abs(t)
            ** (
                calibration["eye_horizontal_power"]
                * parameters["upper_horizontal_roundness"]
            ),
        )
        ** upper_power
    )
    if parameters["upper_asymmetry"] or parameters["upper_angularity"]:
        shaped = dict(
            calibration,
            eye_horizontal_power=calibration["eye_horizontal_power"]
            * parameters["upper_horizontal_roundness"],
            eye_upper_power=upper_power,
            eye_upper_asymmetry=parameters["upper_asymmetry"],
            eye_upper_angularity=parameters["upper_angularity"],
        )
        new_arch = upper_shape(t, shaped)
    upper_height = calibration["eye_upper_arch"] * parameters["upper_arch"]
    if parameters["aperture_upper_share"]:
        total = upper_height + calibration["eye_lower_arch"] * parameters["lower_arch"]
        upper_height = total * parameters["aperture_upper_share"]
    return upper_height * new_arch - calibration["eye_upper_arch"] * arch
