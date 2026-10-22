"""Independent paper/ground plot ink, physical pens and white knockout expectations."""
import copy,json,math,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
profiles=json.loads((ROOT/'fixtures/sheet/v1/plot-profiles.json').read_text())['cases']
page=json.loads((ROOT/'fixtures/sheet/v1/page-setups.json').read_text())['cases'][0]
def ink(rgb,s):
 if rgb[:3]==[255,255,255]:return '#ffffff'
 c=s['color'];v=list(map(float,rgb[:3]));alpha=rgb[3] if len(rgb)==4 else 255
 if c['type']=='black':v=[0.,0.,0.]
 elif c['type']=='grayscale':v=[.2126*v[0]+.7152*v[1]+.0722*v[2]]*3
 elif c['type']=='color':v=[int(c['value'][i:i+2],16) for i in [1,3,5]]
 v=[math.floor(255-(255-x)*s['screening']/100+.5) for x in v]
 if s.get('opacity')is not None:alpha=math.floor(s['opacity']*255/100+.5)
 return '#'+''.join(f'{x:02x}' for x in v)+(f'{alpha:02x}' if alpha<255 else '')
cases=[]
for n,c in enumerate(profiles):
 p=copy.deepcopy(c['exchange']['profile']);style=p['entries'][0]['style']
 if n%6==5:style['dash']=[2000,1000,500]
 if n%6==4:style['dash']=[0,1000]
 if n%4==3:style['color']=dict(type='color',value='#cc3333')
 s=copy.deepcopy(page['sheet']);s['items'][0]['kind']['fill']='#2255aa';s['items'][1]['frame'].update(left=180000,top=50000)
 model=copy.deepcopy(page['model']);model['layers'][0]['style']['color']='#000000';model['entities'].append(dict(kind='polyline',id=2,layerId='cizim',attrs={},pts=[dict(x=0,y=1),dict(x=2.5,y=2.5),dict(x=5,y=1)],closed=False))
 s['items'].append(dict(id='map',name='5 m çizgi ve birleşim',frame=dict(left=30000,top=150000,width=200000,height=100000),kind=dict(type='map',view=dict(type='fixed',center=dict(x=2.5,y=1.25),scale=50,rotation=0))))
 if n%3==2:
  for entity in model['entities']:
   for name in ['a','b','p']:
    if name in entity:entity[name]['x']+=500000.125;entity[name]['y']+=4500000.25
   for point in entity.get('pts',[]):point['x']+=500000.125;point['y']+=4500000.25
  s['items'][-1]['kind']['view']['center']['x']+=500000.125;s['items'][-1]['kind']['view']['center']['y']+=4500000.25
 setup=copy.deepcopy(page['setup']);setup['settings']['profile']=p['id']
 cases.append(dict(name=c['name'],model=model,sheet=s,profile=p,setup=setup,expected=dict(boxColor=ink([34,85,170],style),modelColor=ink([0,0,0],style),white=ink([255,255,255],style),widthMm=style['lineWeight']/1000,dashMm=[v/1000 for v in style['dash']]*(2 if len(style['dash'])%2 else 1),cap=style['cap'],join=style['join'],modelLengthMm=100.)))
p=ROOT/'fixtures/sheet/v1/plot-ink.json';text=json.dumps(dict(version=1,cases=cases),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
