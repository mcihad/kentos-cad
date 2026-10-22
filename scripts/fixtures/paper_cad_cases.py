"""Independent analytic paper copies: micrometres and exact circles/arcs, no KentOS calls."""
import json,math
from pathlib import Path
cases=[]
for radius in [0.0001,0.0002,0.00049,0.0005,0.001,0.01,0.05]:
    for kind in ['circle','arc']:
        e=dict(kind=kind,id=1,layerId='0',attrs={},c=dict(x=.07,y=.09),r=radius)
        if kind=='arc':e.update(a0=0,a1=math.pi/2)
        cases.append(dict(name=f'{kind}-{radius}',entity=e,radiusUm=radius*1e6,center=[.07,.09]))
for angle in [0,.2,1.4,3.1]:
    for full in [True,False]:
        e=dict(kind='ellipse',id=1,layerId='0',attrs={},c=dict(x=.12,y=.13),major=dict(x=.03*math.cos(angle),y=.03*math.sin(angle)),ratio=.4,t0=0,t1=math.tau if full else 2.1)
        cases.append(dict(name=f'ellipse-{angle}-{full}',entity=e,majorLength=.03,ratio=.4))
e=dict(kind='nurbs',id=1,layerId='0',attrs={},degree=2,ctrl=[dict(x=.05,y=0),dict(x=.05,y=.05),dict(x=0,y=.05)],knots=[0,0,0,1,1,1],weights=[1,2**-.5,1],fit=[dict(x=.05,y=0),dict(x=0,y=.05)],tangents=[dict(x=0,y=1),dict(x=-1,y=0)],fitTolerance=1e-5)
cases.append(dict(name='rational-quarter-circle',entity=e,radiusUm=50000))
e=dict(kind='polyline',id=1,layerId='0',attrs={},pts=[dict(x=.02,y=.02),dict(x=.06,y=.02),dict(x=.06,y=.08)],bulges=[1,0,0],closed=False,widths=[[.001,.002],[.002,.001],[0,0]])
cases.append(dict(name='wide-bulged-line',entity=e))
Path('fixtures/sheet/v1/paper-cad.json').write_text(json.dumps({'schema':'kentos.sheet.paper-cad/1','cases':cases},ensure_ascii=False,indent=2)+'\n')
