"""Sparse support gradient with exact scalar finite differences on guide variables."""
import numpy as np


def make_forehead_objective_jacobian(objective,evaluate,start,target,weights,ids,guide_nodes,quads):
    index={node:i for i,node in enumerate(ids)}
    guide_columns=sorted(2*index[node]+axis for node in set(guide_nodes)&set(ids) for axis in (0,1))
    support_columns=sorted(set(range(len(start)))-set(guide_columns))
    support=set(support_columns);row_map={c:[] for c in support_columns};neighbors={c:set() for c in support_columns}
    count=len(quads)
    for k,quad in enumerate(quads):
        columns=sorted(2*index[node]+axis for node in quad if node in index for axis in (0,1))
        columns=[c for c in columns if c in support]
        rows=[4*k+j for j in range(4)]+[4*count+k]
        for c in columns:row_map[c].extend(rows);neighbors[c].update(set(columns)-{c})
    assignments={};groups=[]
    for c in sorted(support_columns,key=lambda c:(-len(neighbors[c]),c)):
        unavailable={assignments[n] for n in neighbors[c] if n in assignments}
        color=next(k for k in range(len(groups)+1) if k not in unavailable)
        if color==len(groups):groups.append([])
        groups[color].append(c);assignments[c]=color
    rows={c:np.asarray(row_map[c],int) for c in support_columns}
    def residual(result):return np.r_[(result[1]-90).ravel()/100,np.sqrt(.1)*np.log(result[2])]
    def jacobian(values):
        base=evaluate(values)
        if base is None:raise ValueError('Objective derivative requires a valid frozen-author sample')
        base_cost=objective(values);r0=residual(base)
        steps=np.maximum(np.abs(values)*1e-8,1e-5)
        gradient=np.zeros_like(values)
        gradient[support_columns]=(2*weights[support_columns]*(values[support_columns]-target[support_columns])
                                   +.04*(values[support_columns]-start[support_columns]))/10000
        for columns in groups:
            trial=values.copy();trial[columns]+=steps[columns];result=evaluate(trial)
            if result is None:
                for column in columns:
                    trial=values.copy();trial[column]+=steps[column];single=evaluate(trial);step=steps[column]
                    if single is None:
                        trial[column]=values[column]-steps[column];single=evaluate(trial);step=-steps[column]
                    if single is None:raise ValueError('No valid local source derivative for support')
                    selected=rows[column]
                    gradient[column]+=2*np.dot(r0[selected],(residual(single)-r0)[selected]/step)/10000
            else:
                difference=residual(result)-r0
                for column in columns:
                    selected=rows[column]
                    gradient[column]+=2*np.dot(r0[selected],difference[selected]/steps[column])/10000
        for column in guide_columns:
            trial=values.copy();trial[column]+=steps[column];step=steps[column]
            if evaluate(trial) is None:trial[column]=values[column]-steps[column];step=-steps[column]
            if evaluate(trial) is None:raise ValueError('No valid local source derivative for guide')
            gradient[column]=(objective(trial)-base_cost)/step
        if not np.isfinite(gradient).all():raise ValueError('Nonfinite forehead objective derivative')
        return gradient
    return jacobian,{'guide_variables':len(guide_columns),'support_variables':len(support_columns),
                     'support_colors':len(groups),'scope':'Numerical gradient acceleration, unchanged scalar objective and final guards'}
