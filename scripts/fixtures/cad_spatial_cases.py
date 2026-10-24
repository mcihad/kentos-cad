#!/usr/bin/env python3
"""Independent 50-digit DXF placement oracle: Autodesk OCS, base, scale, rotation, array, parents.

No KentOS imports. Quarter turns and 3-4-5 normals make the expected coordinates rational.
`--check` compares generated DXFs, SI results and command scenarios without rewriting them.
"""
from decimal import Decimal as D, getcontext
import json
from pathlib import Path
import sys

getcontext().prec = 50
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/formats/v1/cad-spatial'

def point(values): return tuple(D(str(v)) for v in values)
def plus(a, b): return tuple(x+y for x, y in zip(a, b))
def minus(a, b): return tuple(x-y for x, y in zip(a, b))
def cross(a, b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def normalized(v):
    length = sum(x*x for x in v).sqrt()
    return tuple(x/length for x in v)
def axes(normal):
    n = normalized(point(normal))
    x = normalized(cross(point((0,1,0) if abs(n[0]) < D(1)/64 and abs(n[1]) < D(1)/64 else (0,0,1)), n))
    return x, normalized(cross(n, x)), n
def ocs(p, normal):
    basis = axes(normal)
    return tuple(sum(p[j]*basis[j][i] for j in range(3)) for i in range(3))
def placed(p, spec, cell=(0,0)):
    # INSERT's point is in OCS. MINSERT spacing is rotated, but not scaled.
    p = tuple(v*s for v,s in zip(minus(p,point(spec['base'])),point(spec['scale'])))
    p = plus(p,point((cell[0]*spec.get('dc',0),cell[1]*spec.get('dr',0),0)))
    x,y,z = p
    p = {0:(x,y,z),90:(-y,x,z),180:(-x,-y,z),270:(y,-x,z)}[spec['rotation'] % 360]
    return ocs(plus(p,point(spec['p'])),spec['normal'])

def tags(pairs): return ''.join(f'{k}\n{v}\n' for k,v in pairs)
def xyz(code, p): return [(code,p[0]),(code+10,p[1]),(code+20,p[2])]
def section(name, body): return tags([(0,'SECTION'),(2,name)])+body+tags([(0,'ENDSEC')])
def insert(name, spec):
    return tags([(0,'INSERT'),(5,f'F{int(name[1:])+1:X}'),(8,'0'),(2,name),*xyz(10,spec['p']),
                 (41,spec['scale'][0]),(42,spec['scale'][1]),(43,spec['scale'][2]),
                 (50,spec['rotation']),*xyz(210,spec['normal']),
                 (70,spec.get('cols',1)),(71,spec.get('rows',1)),(44,spec.get('dc',0)),(45,spec.get('dr',0))])
def block(name, spec, contents):
    return tags([(0,'BLOCK'),(8,'0'),(2,name),(70,0),*xyz(10,spec['base'])])+contents+tags([(0,'ENDBLK')])

def primitive(kind):
    if kind == 'line':
        pts = [point((1,2,3)),point((4,6,8))]
        return tags([(0,'LINE'),(5,'A1'),(8,'0'),*xyz(10,pts[0]),*xyz(11,pts[1])]), pts
    if kind == 'point':
        pts = [point(('1.5',-2,5))]
        return tags([(0,'POINT'),(5,'A1'),(8,'0'),*xyz(10,pts[0])]), pts
    if kind == 'poly3d':
        pts = [point((0,0,1)),point((3,0,2)),point((3,4,7))]
        content = tags([(0,'POLYLINE'),(5,'A1'),(8,'0'),(66,1),(70,8),*xyz(10,(0,0,0))])
        content += ''.join(tags([(0,'VERTEX'),(8,'0'),(70,32),*xyz(10,p)]) for p in pts)
        return content+tags([(0,'SEQEND'),(8,'0')]), pts
    if kind in ('lwpolyline','poly2d'):
        local = [point((0,0,7)),point((3,0,7)),point((3,4,7))]
        normal = (3,0,4)
        if kind == 'lwpolyline':
            body = tags([(0,'LWPOLYLINE'),(5,'A1'),(100,'AcDbEntity'),(8,'0'),(100,'AcDbPolyline'),
                         (90,3),(70,0),(38,7),*xyz(210,normal)])
            body += ''.join(tags([(10,p[0]),(20,p[1])]) for p in local)
        else:
            body = tags([(0,'POLYLINE'),(5,'A1'),(8,'0'),(66,1),(70,0),*xyz(10,(0,0,7)),*xyz(210,normal)])
            # A 2D POLYLINE's plane elevation wins over a VERTEX's own z.
            body += ''.join(tags([(0,'VERTEX'),(8,'0'),(70,0),*xyz(10,(p[0],p[1],99))]) for p in local)
            body += tags([(0,'SEQEND'),(8,'0')])
        return body, [ocs(p,normal) for p in local]
    if kind in ('solid','trace'):
        pts = [point((0,0,7)),point((3,0,7)),point((3,4,7)),point((0,4,7))]
        normal = (0,3,4)
        body = tags([(0,kind.upper()),(5,'A1'),(8,'0'),*xyz(210,normal),
                     *xyz(10,pts[0]),*xyz(11,pts[1]),*xyz(12,pts[3]),*xyz(13,pts[2])])
        return body, [ocs(p,normal) for p in pts]
    pts = [point((0,0,1)),point((3,0,2)),point((3,4,3)),point((0,4,2))]
    return tags([(0,'3DFACE'),(5,'A1'),(8,'0'),*(tag for i,p in enumerate(pts) for tag in xyz(10+i,p))]), pts

BASE = dict(base=(1,-2,3),p=(10,20,30),scale=(2,3,4),rotation=90,normal=(0,0,1))
def pose(**values): return {**BASE,**values}
POSES = [
    ('negative-z',[pose(normal=(0,0,-1))]),
    ('tilted-yz',[pose(normal=(0,3,4))]),
    ('tilted-xz',[pose(normal=(-3,0,4),scale=(-2,3,'.5'),rotation=270)]),
    ('edge-on',[pose(normal=(0,1,0),rotation=0)]),
    ('nested',[pose(normal=(3,0,4),scale=(-2,3,'.5'),rotation=270),pose(normal=(0,3,4))]),
    ('nested-negative',[pose(normal=(0,3,4)),pose(normal=(0,0,-1),rotation=180,scale=(3,-2,4))]),
    ('array',[pose(normal=(0,3,4),cols=3,rows=2,dc=7,dr=11)]),
    ('large-coordinates',[pose(normal=(0,3,4),p=(1000000000,-2000000000,3000000000),
                               scale=('.001','.003','.002'))]),
]

def entity(kind, pts):
    xy = lambda p: dict(x=float(p[0]),y=float(p[1]))
    if kind == 'line': return dict(kind='line',a=xy(pts[0]),b=xy(pts[1]),za=float(pts[0][2]),zb=float(pts[1][2]))
    if kind == 'point': return dict(kind='point',p=xy(pts[0]),z=float(pts[0][2]))
    return dict(kind='polygon' if kind in ('solid','trace','face') else 'polyline',pts=[xy(p) for p in pts],zs=[float(p[2]) for p in pts])

def magnitude(value):
    if isinstance(value,dict): return max(map(magnitude,value.values()),default=0)
    if isinstance(value,list): return max(map(magnitude,value),default=0)
    return abs(value) if isinstance(value,(int,float)) else 0

def build():
    files, cases = {}, []
    for index,(label,poses) in enumerate(POSES):
        factor = D('.001') if index % 2 == 0 or label == 'large-coordinates' else D(1)
        for kind in ('line','point','poly3d','lwpolyline','poly2d','solid','trace','face'):
            name = f'{label}-{kind}'
            body, points = primitive(kind)
            blocks = ''
            for i, placement in enumerate(poses):
                blocks += block(f'B{i}',placement,body)
                body = insert(f'B{i}',placement)
            data = (section('HEADER',tags([(9,'$ACADVER'),(1,'AC1032'),(9,'$INSUNITS'),(70,4 if factor == D('.001') else 6)]))
                    + section('BLOCKS',blocks)+section('ENTITIES',body)+tags([(0,'EOF')]))
            expected = []
            outer = poses[-1]
            for row in range(outer.get('rows',1)):
                for col in range(outer.get('cols',1)):
                    transformed = points
                    for i, placement in enumerate(poses):
                        transformed = [placed(p,placement,(col,row) if i == len(poses)-1 else (0,0)) for p in transformed]
                    expected.append(entity(kind,[tuple(c*factor for c in p) for p in transformed]))
            files[OUT/f'{name}.dxf'] = data
            cases.append(dict(name=name,source=f'formats/v1/cad-spatial/{name}.dxf',expected=expected))
    files[OUT/'expected.json'] = json.dumps(dict(format='kentos.cad-spatial',cases=cases),ensure_ascii=False,indent=2)+'\n'
    # Product path: actual import -> one undo -> redo, UI/Python/MCP all use this command.
    setup = json.loads((ROOT/'fixtures/commands/v1/file.import.dxf.json').read_text())['setup']
    commands = []
    for case in cases:
        count = len(case['expected'])
        # Same predeclared arithmetic bound as the per-coordinate formats test;
        # this runner accepts one absolute bound for the whole step.
        within = magnitude(case['expected'])*32*sys.float_info.epsilon+1e-12
        commands.append(dict(name=case['name'],steps=[dict(op='execute',within=within,input=dict(path='$fixture:'+case['source'],explodeBlocks=True),
            result=dict(status='completed',output='$any',warnings=[]),
            expect=dict(ids=list(range(1,count+2)),fields={str(i+2):e for i,e in enumerate(case['expected'])})),
            dict(op='undo',expect=dict(ids=[1],canRedo=True)),dict(op='redo',within=within,expect=dict(ids=list(range(1,count+2)),
                fields={str(i+2):e for i,e in enumerate(case['expected'])}))]))
    files[ROOT/'fixtures/commands/v1/file.import.spatial.json'] = json.dumps(dict(format='kentos.command-cases',version=1,
        command='file.import.dxf',commandVersion=1,title='Bloklarda tam XYZ yerleştirme',note=__doc__.strip(),setup=setup,cases=commands),
        ensure_ascii=False,indent=2)+'\n'
    for path, data in files.items():
        if '--check' in sys.argv:
            if path.read_text() != data: raise SystemExit(f'fixture drift: {path}')
        else:
            path.parent.mkdir(parents=True,exist_ok=True)
            path.write_text(data)
    print(f'{len(cases)} independent XYZ sources and command scenarios')

if __name__ == '__main__': build()
