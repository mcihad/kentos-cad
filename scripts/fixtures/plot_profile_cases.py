"""Independent indexed/named plot table keys; same RGB never implies the same ACI."""
import json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];cases=[]
for n in range(20):
 named=n>=10;index=n%10+1;key=dict(type='named',name=f'Kalem {index}') if named else dict(type='aci',index=index)
 style=dict(color=dict(type=['object','black','grayscale'][n%3]),screening=25*(n%4+1),lineWeight=100+50*n,dash=[2000,1000] if n%2 else [],opacity=50 if n%2 else 100,cap=['butt','round','square'][n%3],join=['miter','round','bevel'][n%3])
 p=dict(id=f'profile-{n}',name=f'Ofis {n}',kind='named' if named else 'colorDependent',entries=[dict(key=key,style=style)],assignments=[dict(target=dict(type='modelLayer',id='cizim'),key=key),dict(target=dict(type='paperItem',id='box'),key=key)])
 if named:p['defaultStyle']=key['name']
 cases.append(dict(name=f"{p['kind']} {index}",exchange=dict(schema='kentos.plot/1',profile=p),expected=dict(key=key,unassigned=named)))
p=ROOT/'fixtures/sheet/v1/plot-profiles.json';text=json.dumps(dict(version=1,cases=cases),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
