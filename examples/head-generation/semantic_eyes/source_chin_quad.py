"""Close the chin-side start with a quad instead of a collapsed triangular fan."""


import inspect


def limit_chin_height_warp():
    """Bound the low source height warp against radial chart Jacobian reversal."""
    from math import exp, isfinite
    requested = C['chin_return_lift']
    factor = 1.0
    worst = None
    for i in range(401):
        y = Y0 + .04*i/400
        step = 1e-6
        w = width(y)
        derivative = (width(y+step)-width(y-step))/(2*step)
        a = H*requested*exp(-(y-1.544)/.008)
        b = H*C['chin_front_lift']*exp(-(y-Y0)/(.02*H))
        base = w*(1-b/(.02*H))
        influence = -w*a/.008-2*derivative*a
        if not all(isfinite(v) for v in (base,influence)) or base<=0:
            raise ValueError('Invalid unwarped chin height chart')
        if influence<0:
            bound = .85*base/(-influence)
            if bound<factor:
                factor=max(0.0,bound)
                worst=y
    C['chin_return_lift']=requested*factor
    return {'requested_lift':requested,'effective_lift':C['chin_return_lift'],
            'factor':factor,'limiting_height_m':worst,
            'scope':'source radial-height chart; final geometry and intersections separately verified'}


def adapt(sources):
    out=dict(sources)
    shell=out['shell.py']
    old='    previous = chin_columns\n    side_path = [cols - 1]'
    new='''    previous = chin_columns
    initial_return = []
    for x, negz, y in base:
        initial_return.append(len(verts))
        verts.append((x,negz+0.001,y-0.00025))
    for i in range(len(base)-1):
        shell_faces.append((previous[i+1],previous[i],initial_return[i],initial_return[i+1]))
    previous = initial_return
    under_depth_rows = [initial_return]
    side_path = [initial_return[-1]]'''
    assert shell.count(old)==1
    shell=shell.replace(old,new,1)
    old='    shell_faces.append((outer[0], side_path[1], outer[1]))'
    new='    shell_faces.append((outer[0], side_path[0], side_path[1], outer[1]))'
    assert shell.count(old)==1
    shell=shell.replace(old,new,1)
    shell=shell.replace('"chin_side_triangles": 1','"chin_side_triangles": 0')
    shell=shell.replace('"chin_side_quads": len(side_path) - 2','"chin_side_quads": len(side_path) - 1')
    old = '    for back, lift, spread in C["chin_underside_stations"]:'
    new = ('    first_back,first_lift,first_spread = C["chin_underside_stations"][0]\n'
           '    support_stations = [(first_back*t,first_lift*t,1+(first_spread-1)*t) for t in (1/3,2/3)]\n'
           '    for back, lift, spread in support_stations + C["chin_underside_stations"]:')
    assert shell.count(old)==1
    shell=shell.replace(old,new,1)
    marker='        for i in range(len(base) - 1):'
    assert shell.count(marker)==1
    shell=shell.replace(marker,'        under_depth_rows.append(current)\n'+marker,1)
    out['face.py'] += '\n\n'+inspect.getsource(limit_chin_height_warp)
    out['pipeline.py'] = out['pipeline.py'].replace('report = build_face()',
        "from face import limit_chin_height_warp\n_chin_chart = limit_chin_height_warp()\nreport = build_face()\nreport['chin_height_chart_guard'] = _chin_chart",1)
    out['shell.py']=shell
    return out
