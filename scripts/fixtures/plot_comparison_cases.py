"""Known paper control points for colour/mono/gray, masks and ordered CAD ink. No KentOS imports."""
import copy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
base=json.loads((ROOT/'fixtures/sheet/v1/plot-ink.json').read_text())['cases'][0]
model=copy.deepcopy(base['model']);model['entities']=[]
def pt(x,y):return dict(x=x,y=y)
def ring(x,y,w,h):return [pt(x,y),pt(x+w,y),pt(x+w,y+h),pt(x,y+h)]
def add(kind,**kw):
 e=dict(kind=kind,id=len(model['entities'])+1,layerId='cizim',attrs={});e.update(kw);model['entities'].append(e)
# Hatch behind masked text, and wipeout between a background and foreground line.
add('hatch',ring=ring(0,0,3,5),pattern=dict(type='lines',angle=0,spacing=.15),color='#cc3333')
add('text',p=pt(.4,2),text='MASK',height=.5,rotation=0,mask=True,color='#2244cc')
for y in [1,2,3,4]:add('line',a=pt(3.5,y),b=pt(6.5,y),color='#cc3333',lineWeight=.5)
add('polygon',pts=ring(4.25,1.5,1.5,2),wipeout=dict(frame=False))
add('line',a=pt(4.5,2.5),b=pt(5.5,2.5),color='#2244cc',lineWeight=.5)
add('hatch',ring=ring(7,0,3,5),pattern=dict(type='lines',angle=45,spacing=.15),color='#cc3333')
add('hatch',ring=ring(8,1,1,3),pattern=dict(type='solid',angle=0,spacing=1),color='#339966')
# Linked PNG bytes are created by the runner from a deterministic single-colour image.
add('image',file='$image',p=pt(10.5,0),width=3,height=5,rotation=0)
book=dict(version=1,masters=[],sheets=[])
profiles=[]
for n,(name,mode) in enumerate([('Renkli','object'),('Tek renk','black'),('Gri','grayscale')]):
 s=copy.deepcopy(base['sheet']);s['id']=f'comparison-{n}';s['name']=name;s['items']=[dict(id=f'heading-{n}',name='Başlık',frame=dict(left=30000,top=20000,width=360000,height=20000),kind=dict(type='text',content=f'{name} — baskı karşılaştırması',style=dict(size=6000))),dict(id=f'footer-{n}',name='Kontrol',frame=dict(left=30000,top=270000,width=360000,height=10000),kind=dict(type='text',content='1:50 · Raster pikselleri renklerini korur · Beyaz alanlar maske boyasıdır',style=dict(size=3000)))]
 for i,label in enumerate(['Tarama + yazı maskesi','Wipeout + ön çizgi','Tarama sırası','Raster altlık','PDF altlık']):
  s['items'].append(dict(id=f'label-{n}-{i}',name=label,frame=dict(left=30000+i*70000,top=90000,width=68000,height=15000),kind=dict(type='text',content=label,style=dict(size=3500))))
 s['items'].append(dict(id=f'map-{n}',name=name,frame=dict(left=30000,top=60000,width=360000,height=200000),kind=dict(type='map',view=dict(type='fixed',center=pt(8,2.5),scale=50,rotation=0))))
 p=dict(id=f'profile-{n}',name=name,kind='named',entries=[dict(key=dict(type='named',name='Ink'),style=dict(color=dict(type=mode),opacity=100))],assignments=[],defaultStyle='Ink');profiles.append(p)
 s['plot']=dict(settings=dict(profile=p['id'],profileSnapshot=p));book['sheets'].append(s)
book['plotProfiles']=profiles
# Ground -> paper: x=50+20*x mm, y=210-20*y mm, independent of implementation.
checks=[dict(name='PDF white knockout',at=[354,196],rgb=[255,255,255]),dict(name='PDF underlay ink',at=[350,200],rgb=[204,51,51]),dict(name='text mask blank',at=[57.2,165],rgb=[255,255,255]),dict(name='wipeout blank',at=[150,170],rgb=[255,255,255]),dict(name='wipeout foreground',at=[150,160],rgb=[34,68,204]),dict(name='solid in front of hatch',at=[220,160],rgb=[51,153,102]),dict(name='raster keeps pixels',at=[290,160],rgb=[20,160,200])]
p=ROOT/'fixtures/sheet/v1/plot-comparison.json';text=json.dumps(dict(version=1,model=model,book=book,checks=checks),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
