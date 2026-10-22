"""Independent source identities and exact clip snap candidates, in metres (no KentOS runtime)."""
import copy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
base=json.loads((ROOT/'fixtures/sheet/v1/viewport-annotations.json').read_text())['cases'];cases=[]
for src in base:
 c=copy.deepcopy(src);s=c['expected']['physical'];w=.390*s
 c['operation']['clip']={'type':'polygon','points':[[0,0],[750000,0],[750000,750000],[500000,750000],[500000,1000000],[0,1000000]]}
 c['model']['entities'] += [{'kind':'point','id':n,'layerId':'geom','attrs':{},'p':{'x':x*w,'y':0.}} for n,x in [(4,.251),(5,.247)]]
 c['snap']={'at':{'x':.2495*w,'y':0.},'aperture':.006*w,'kinds':['node'],'expectedSlot':5}
 origin={'x':500000.125,'y':4500000.25} if ' half' in c['name'] or ' missing' in c['name'] else {'x':0.,'y':0.}
 for entity in c['model']['entities']:
  for key in ['p','a','b']:
   if key in entity:
    for axis in ['x','y']:entity[key][axis]+=origin[axis]
 c['sheet']['items'][0]['kind']['view']['center']=origin.copy()
 for axis in ['x','y']:c['snap']['at'][axis]+=origin[axis]
 c['origin']=origin
 c['expected']['selectedSlots']=[1,2]+([3] if c['expected']['visible'] else [])+[5]
 cases.append(c)
path=ROOT/'fixtures/sheet/v1/viewport-identities.json';text=json.dumps({'version':1,'cases':cases},ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert path.read_text()==text
else:path.write_text(text)
