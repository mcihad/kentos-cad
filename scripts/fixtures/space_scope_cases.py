"""ADR 0645: independent rectangular source-scope cases, Decimal arithmetic.

An unassigned 10×8 m wall centre rectangle (width .2), a level-bound
partition at x=5, an unassigned 2×2 column at (3,4) and a foreign-level
2×2 column at (8,4). A left-side seed finds the whole unassigned room or
only the left mixed-source room. Equal elevations never equate identities.
No KentOS/native/SDK geometry is imported.
"""
from copy import deepcopy
from decimal import Decimal as D
from pathlib import Path
import argparse
import json
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'fixtures/commands/v1/cad.spaces.scope.json'
LABEL={'net':'Net (iç yüzler)','centerline':'Duvar ekseni','gross':'Brüt (dış yüzler)'}
DELTA={'net':D('-.2'),'centerline':D(0),'gross':D('.2')}
def completed(output):return {'status':'completed','output':output,'warnings':[]}
def setup(unit,far):
 d=deepcopy(json.loads((ROOT/'fixtures/commands/v1/cad.spaces.create.json').read_text())['setup'])
 d['name']='Atanmamış kaynak bağımsız kabulü';d['settings']['drawingUnit']=unit
 d['settings']['arch']['levels'][1]['elevation']=0.
 x,y=(500000.,4400000.) if far else (0.,0.)
 p=lambda a,b:{'x':x+a,'y':y+b}
 d['origin']={'x':0.,'y':0.}
 d['entities']=[
  dict(kind='wall',id=1,layerId='duvar',attrs={},closed=True,width=.2,pts=[p(0,0),p(10,0),p(10,8),p(0,8)]),
  dict(kind='wall',id=2,layerId='duvar',attrs={},width=.2,arch={'level':'zemin'},pts=[p(5,0),p(5,8)]),
  dict(kind='column',id=3,layerId='cizim',attrs={},width=2.,depth=2.,at=p(3,4)),
  dict(kind='column',id=4,layerId='cizim',attrs={},width=2.,depth=2.,at=p(8,4),arch={'level':'kat-1'}),
 ]
 return d,p(2,6)
def case(unit,method,scope,far):
 d,seed=setup(unit,far);unassigned=scope=='unassigned'
 width=D(10 if unassigned else 5)
 by={m:float((width+v)*(D(8)+v)-4)for m,v in DELTA.items()}
 area=D(str(by[method]));v=DELTA[method]
 spec=dict(seed=seed,method=method,columns='all')
 if scope is not None:spec['scope']=scope
 inp={'layerId':'mahal','spaces':[spec]}
 e=dict(kind='space',id=5,layerId='mahal',attrs={},parts='$any',seed=seed,columns='all')
 if method!='net':e['method']=method
 if unassigned:e['scope']=scope
 rule=f'alan: halkaların alanı, delikler düşülür ({LABEL[method]}); hacim: alan × yükseklik; kolon alanı: Her yöntemde'
 if unassigned:rule+='; kaynaklar: yalnız atanmamış'
 row=dict(uid='$uidOf:5',method=method,columns='all',area=float(area),perimeter=float(2*(width+D(8)+2*v)+8),byMethod=by,rule=rule)
 if unassigned:row['scope']=scope
 state={'ids':[1,2,3,4,5],'entities':{'5':e},'canUndo':True,'dirty':True,'revision':'changed'}
 return dict(name=f'{unit}/{method}/{scope or "legacy"}/{"far" if far else "local"}',setup=d,steps=[
  dict(op='plan',input=inp,result=completed({'spaces':[dict(e,id=0)],'revision':'$current'}),expect={'revision':'same','ids':[1,2,3,4]}),
  dict(op='execute',input=inp,result=completed({'uids':['$any'],'ids':[5],'revision':'$current'}),expect=state),
  dict(op='execute',command='cad.spaces.quantities',input={},within=1e-6,result=completed({'spaces':[row],'walls':'$any','openings':[],'totals':'$any'}),expect={'revision':'same'}),
  dict(op='undo',returns='Mahal',expect={'ids':[1,2,3,4],'canUndo':False,'canRedo':True,'revision':'changed'}),
  dict(op='redo',returns='Mahal',expect=state),
 ])
def document():
 return dict(format='kentos.command-cases',version=1,command='cad.spaces.create',commandVersion=1,
  title='Kalıcı kaynak kapsamı: atanmamış / bütün çizim, üç yöntem, beş birim, iki koordinat',note=__doc__.strip(),setup=setup('m',False)[0],
  cases=[case(u,m,s,far)for u in ['mm','cm','m','in','ft']for m in DELTA for s in [None,'project','unassigned']for far in [False,True]])
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args()
 result=json.dumps(document(),ensure_ascii=False,indent=2)+'\n'
 if a.check:assert OUT.read_text()==result
 else:OUT.write_text(result)
 print(len(document()['cases']),'independent source-scope cases')
