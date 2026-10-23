"""Independent physical calibration: exact rational mm conversion, no KentOS code."""
import copy,json,sys
from fractions import Fraction
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts/fixtures'))
from sheets_edit_command_cases import SETUP,sheet,stroke
cases=[]
for n in range(20):
 denominator=[1,2,5,10,20,25,50,75,100,200][n%10]
 metres=Fraction(denominator,10) # A 100 mm mark on paper.
 origin= [3000000,4400000] if n>=10 else [0,0]
 model=copy.deepcopy(SETUP);model['settings']['drawingUnit']=['mm','cm','m'][n%3]
 model['entities'][0]['a']={'x':origin[0]-float(metres)/2,'y':origin[1]}
 model['entities'][0]['b']={'x':origin[0]+float(metres)/2,'y':origin[1]}
 model['layers'][0]['style']['color']='#2255aa'
 s=sheet('page','Baskı kalibrasyonu',denominator)
 m=s['items'][0];m['kind']['view'].update(center=dict(zip(['x','y'],origin)),rotation=90000 if n>=10 else 0)
 m['kind']['viewLocked']=True
 s['items'].append(dict(id='calibration',name='100 mm kontrol çizgisi',frame=dict(left=30000,top=265000,width=100000,height=1000),locked=True,kind=dict(type='line',points=[[0,0],[1000000,0]],stroke=stroke('#cc3344',200))))
 cases.append(dict(name=f'{n}: 1:{denominator}',model=model,sheet=s,expected=dict(lengthMM=float(metres*1000/denominator),sourceMM=float(metres*1000),toleranceMM=.002)))
text=json.dumps(dict(version=1,cases=cases),ensure_ascii=False,indent=2)+'\n'
p=ROOT/'fixtures/sheet/v1/calibration.json'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)

# Hand-authored template must keep fixed physical axes independent of paper size.
t=json.loads((ROOT/'crates/shared/sheet/templates/baski-kalibrasyonu.json').read_text())
items={i['id']:i for i in t['sheet']['items']}
assert items['horizontal']['frame']['width']==100000
assert items['vertical']['frame']['height']==100000
assert items['model']['kind']['viewLocked']
assert items['model']['bindings']==[dict(property='map.scale',expression='50')]
assert len({i['name'] for i in items.values()})==len(items)
