"""Independent Autodesk DXF group streams; integrity refusals, not a KentOS round-trip oracle.
Run --check to verify both sources and command expectations without regenerating them.
"""
import json
from pathlib import Path
import sys
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/formats/v1/cad-io'
def pairs(*p): return ''.join(f'{k}\n{v}\n' for k,v in p)
def section(name, body): return pairs((0,'SECTION'),(2,name))+body+pairs((0,'ENDSEC'))
def drawing(body, blocks=''):
    return (section('HEADER',pairs((9,'$ACADVER'),(1,'AC1032'),(9,'$INSUNITS'),(70,6)))+
        (section('BLOCKS',blocks) if blocks else '')+section('ENTITIES',body)+pairs((0,'EOF'))).encode()
def path(normal=None, width=False, arc=False):
    return pairs((0,'LWPOLYLINE'),(5,'AF'),(8,'0'),(90,3),(70,0),(38,7))+(
        pairs((210,normal[0]),(220,normal[1]),(230,normal[2])) if normal else '')+(
        pairs((43,.25)) if width else '')+pairs((10,0),(20,0),(42,.5 if arc else 0),(10,3),(20,0),(10,3),(20,4))
def sources():
    out=[]
    for i, normal in enumerate([(0,.6,.8),(.6,0,.8),(.6,.8,0),(0,-.6,.8),(.000001,0,1)]):
        for feature in ['width','bulge']:
            out.append((f'ocs-{i}-{feature}',drawing(path(normal, feature=='width',feature=='bulge')),{}))
    for i,(sx,sy,angle) in enumerate([(2,3,0),(-2,3,90),(2,-3,30),(.5,4,270),(1,1.000000000001,0)]):
        for feature in ['width','bulge']:
            block=pairs((0,'BLOCK'),(2,'B'),(70,0),(10,0),(20,0))+path(None,feature=='width',feature=='bulge')+pairs((0,'ENDBLK'))
            insert=pairs((0,'INSERT'),(5,'AA'),(8,'0'),(2,'B'),(10,10),(20,20),(41,sx),(42,sy),(50,angle))
            out.append((f'nonuniform-{i}-{feature}',drawing(insert,block),{'explodeBlocks':True}))
    return out
def build():
    setup=json.loads((ROOT/'fixtures/commands/v1/file.import.dxf.json').read_text())['setup']
    cases=[]
    for name,data,options in sources():
        for op in ['plan','execute']:
            cases.append({'name':f'{name} / {op}: kayıplı dönüşüm çizimi değiştirmez','steps':[{
                'op':op,'input':{'path':f'$fixture:formats/v1/cad-io/{name}.dxf',**options},
                'result':{'status':'failed','error':{'code':'polyline_geometry_loss','message':'$any','path':'allowPolylineLoss'}},
                'expect':{'ids':[1],'revision':'same','canUndo':False}}]})
    doc={'format':'kentos.command-cases','version':1,'command':'file.import.dxf','commandVersion':1,
        'title':'Çoklu çizgi bütünlüğü','note':__doc__.strip(),'setup':setup,'cases':cases}
    files={OUT/f'{name}.dxf':data for name,data,_ in sources()}
    files[ROOT/'fixtures/commands/v1/file.import.polyline-policy.json']=(json.dumps(doc,ensure_ascii=False,indent=2)+'\n').encode()
    for file,data in files.items():
        if '--check' in sys.argv:
            if file.read_bytes()!=data:raise SystemExit(f'fixture drift: {file}')
        else:file.write_bytes(data)
    print(f'{len(sources())} independent sources; {len(cases)} command cases')
if __name__=='__main__':build()
