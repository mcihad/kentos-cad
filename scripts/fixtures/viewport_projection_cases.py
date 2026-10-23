"""Independent model-metre/paper-um/pixel arithmetic, with the same source under five project display units."""
import json,math
from pathlib import Path
cases=[]
for n,(unit,per_metre) in enumerate([('m',1.),('mm',1000.),('cm',100.),('in',1000/25.4),('ft',1/.3048)]):
    for denominator in [50.,100.,37.5,.5]:
        for rotation in [0,30000]:
            center=[487150.,4417820.] if n%2 else [0.,0.]
            length=.05 if denominator<1 else 5.
            world=[[center[0]-length/2,center[1]],[center[0]+length/2,center[1]],[center[0],center[1]+length/4],[center[0]+.0001,center[1]+.0001]]
            frame=dict(left=110000,top=100000,width=200000,height=100000)
            anchor=[210000.,150000.];a=math.radians(rotation/1000);c=math.cos(a);s=math.sin(a);k=1e6/denominator
            paper=[[anchor[0]+((p[0]-center[0])*c+(p[1]-center[1])*s)*k,anchor[1]+((p[0]-center[0])*s-(p[1]-center[1])*c)*k] for p in world]
            inverse=lambda x,y:[center[0]+(x*c+y*s)/k,center[1]+(x*s-y*c)/k]
            corners=[inverse(x,y) for x,y in [(-100000,-50000),(100000,-50000),(100000,50000),(-100000,50000)]]
            expected=dict(paper=paper,pixels=[[(p[0]-110000)*.005,(p[1]-100000)*.005] for p in paper],corners=corners,bounds=[min(p[0] for p in corners),min(p[1] for p in corners),max(p[0] for p in corners),max(p[1] for p in corners)],lengthMm=length*1000/denominator)
            cases.append(dict(name=f'{unit}-1:{denominator}-{rotation}',unit=unit,sourceUnitsPerMetre=per_metre,modelLength=length,sourceLength=length*per_metre,center=center,denominator=denominator,rotation=rotation,frame=frame,world=world,pixelsPerUm=.005,expected=expected))
Path('fixtures/sheet/v1/viewport-projection.json').write_text(json.dumps(dict(schema='kentos.sheet.viewport-projection/1',cases=cases),ensure_ascii=False,indent=2)+'\n')
