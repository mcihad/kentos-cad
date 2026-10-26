"""ADR 0672 independent rectangular column union cases; no KentOS imports.

Two 2×2 squares at (3,4)/(4,4) overlap by 1×2: union 3×2 = 6 m²,
perimeter 10 m. A separate upper-level square contributes 4 m² / 8 m only
when no level is selected. Wall centre rectangle 10×8, width 0.2; its
Net/Gross offsets are ±0.1. All expected sizes use Decimal arithmetic.
"""
from copy import deepcopy
from decimal import Decimal as D
from pathlib import Path
import argparse
import json

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/cad.spaces.columns.json'
LABEL = {'net': 'Net (iç yüzler)', 'centerline': 'Duvar ekseni', 'gross': 'Brüt (dış yüzler)'}
POLICY = {'net': 'Yalnız net', 'all': 'Her yöntemde'}
BASE = {'net': D('9.8')*D('7.8'), 'centerline': D(80), 'gross': D('10.2')*D('8.2')}
PERIM = {'net': D('35.2'), 'centerline': D(36), 'gross': D('36.8')}

def completed(output):
    return {'status': 'completed', 'output': output, 'warnings': []}

def setup(unit):
    d = deepcopy(json.loads((ROOT/'fixtures/commands/v1/cad.spaces.create.json').read_text())['setup'])
    d['name'] = 'Bağımsız kolon birleşimi'
    d['settings']['drawingUnit'] = unit
    # Equal elevations deliberately must not equate level identities.
    d['settings']['arch']['levels'][1]['elevation'] = 0.0
    d['entities'] = [
        dict(kind='wall', id=1, layerId='duvar', attrs={}, closed=True, width=.2,
             arch={'level':'zemin'}, pts=[{'x':x,'y':y} for x,y in [(0.,0.),(10.,0.),(10.,8.),(0.,8.)]]),
        *[dict(kind='column', id=i, layerId='cizim', attrs={}, width=2., depth=2.,
               at={'x':x,'y':4.}, arch={'level':level})
          for i,x,level in [(2,3.,'zemin'),(3,4.,'zemin'),(4,8.,'kat-1')]],
    ]
    return d

def applies(policy, method):
    return policy == 'all' or (policy == 'net' and method == 'net')

def case(unit, method, policy, level):
    deduction = D(6 if level else 10)
    boundary = D(10 if level else 18)
    by = {m:float(v-(deduction if applies(policy,m) else D(0))) for m,v in BASE.items()}
    area = D(str(by[method]))
    spec = {'seed':{'x':5.,'y':6.}, 'method':method}
    if policy is not None: spec['columns'] = policy
    if level: spec['arch'] = {'level':'zemin'}
    inp = {'layerId':'mahal', 'spaces':[spec]}
    entity = {'kind':'space','id':5,'layerId':'mahal','attrs':{}, 'parts':'$any', 'seed':spec['seed']}
    if method != 'net': entity['method'] = method
    if policy in POLICY: entity['columns'] = policy
    if level: entity['arch'] = spec['arch']
    row = {'uid':'$uidOf:5','method':method,'area':float(area),
           'perimeter':float(PERIM[method]+(boundary if applies(policy,method) else D(0))),
           'byMethod':by,
           'rule':f'alan: halkaların alanı, delikler düşülür ({LABEL[method]}); hacim: alan × yükseklik'}
    if policy in POLICY:
        row['columns'] = policy
        row['rule'] += '; kolon alanı: '+POLICY[policy]
    if level: row.update(level='zemin',height=3.,volume=float(area*D(3)))
    state = {'ids':[1,2,3,4,5],'entities':{'5':entity},'canUndo':True,'dirty':True,'revision':'changed'}
    return dict(name=f'{unit} / {method} / {policy or "legacy"} / {"zemin" if level else "tüm katlar"}',setup=setup(unit),steps=[
        dict(op='plan',input=inp,result=completed({'spaces':[dict(entity,id=0)],'revision':'$current'}),expect={'revision':'same','ids':[1,2,3,4]}),
        dict(op='execute',input=inp,result=completed({'uids':['$any'],'ids':[5],'revision':'$current'}),expect=state),
        dict(op='execute',command='cad.spaces.quantities',input={},within=1e-6,
             result=completed({'spaces':[row],'walls':'$any','openings':[], 'totals':'$any'}),expect={'revision':'same'}),
        dict(op='undo',returns='Mahal',expect={'ids':[1,2,3,4],'canUndo':False,'canRedo':True,'revision':'changed'}),
        dict(op='redo',returns='Mahal',expect=state),
    ])

def document():
    cases = [case(u,m,p,True) for u in ['mm','cm','m','in','ft'] for m in BASE for p in [None,'ignore','net','all']]
    cases += [case('m',m,p,False) for m in BASE for p in [None,'ignore','net','all']]
    return dict(format='kentos.command-cases',version=1,command='cad.spaces.create',commandVersion=1,
                title='Kolon alanı: örtüşen iz, açık kat, üç yöntem, beş birim, legacy ve tek undo',
                note=__doc__.strip(),setup=setup('m'),cases=cases)

if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    text=json.dumps(document(),ensure_ascii=False,indent=2)+'\n'
    if args.check:
        if OUT.read_text()!=text: raise SystemExit(f'{OUT}: stale')
    else: OUT.write_text(text)
    print(f'{len(document()["cases"])} independent column command cases {"checked" if args.check else "written"}')
