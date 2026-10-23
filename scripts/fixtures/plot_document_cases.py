"""PDF document expectations from SFNT font names and analytic circles; no KentOS runtime."""
import copy,json,struct,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
base=json.loads((ROOT/'fixtures/sheet/v1/plot-ink.json').read_text())['cases'][0]
faces=[('architects-daughter','ArchitectsDaughter',400,False),*[(k,f,w,i) for k,f,weights in [('arimo','Arimo',[400,500,600,700]),('barlow','Barlow',[400,500,600]),('courier-prime','CourierPrime',[400,700]),('plex-mono','IBMPlexMono',[400,500]),('overpass','Overpass',[400,500,600,700]),('quicksand','Quicksand',[400,500,600,700])] for w in weights for i in ([False,True] if k in ['barlow','courier-prime'] and w==400 else [False])]]
def postscript(path):
 b=path.read_bytes();count=struct.unpack_from('>H',b,4)[0]
 for n in range(count):
  tag,_,at,_=struct.unpack_from('>4sIII',b,12+16*n)
  if tag==b'name':break
 _,count,start=struct.unpack_from('>HHH',b,at)
 for n in range(count):
  platform,_,_,name,length,offset=struct.unpack_from('>HHHHHH',b,at+6+12*n)
  if name==6:
   raw=b[at+start+offset:at+start+offset+length];return raw.decode('utf-16be' if platform in [0,3] else 'mac_roman')
 raise AssertionError(path)
model=copy.deepcopy(base['model']);model['entities']=[model['entities'][0],dict(kind='circle',id=2,layerId='cizim',attrs={},c=dict(x=2.5,y=1.25),r=1.,color='#336699'),dict(kind='text',id=3,layerId='cizim',attrs={},p=dict(x=1,y=2.5),text='MODEL 5000 mm',height=.15,rotation=0)]
model['entities'].append(dict(kind='hatch',id=4,layerId='cizim',attrs={},ring=[dict(x=4,y=1),dict(x=4.8,y=1),dict(x=4.8,y=2.2),dict(x=4,y=2.2)],pattern=dict(type='lines',angle=45,spacing=.1),color='#bb8800'))
cases=[]
for n,(family,file,weight,italic) in enumerate(faces):
 s=copy.deepcopy(base['sheet']);s['id']=f'quality-{n}';s['name']=f'{family} {weight} {italic}';s['items']=[];r=(n+1)*1000
 text=f'{n:02} Çığ İŞ ğü fi fl ölçü 1:50'
 s['items'].append(dict(id=f'text-{n}',name='Yazı',frame=dict(left=30000,top=20000,width=350000,height=20000),kind=dict(type='text',content=text,style=dict(font=family,weight=weight,italic=italic,size=4000))))
 s['items'].append(dict(id=f'circle-{n}',name='Analitik daire',frame=dict(left=60000-r,top=90000-r,width=2*r,height=2*r),kind=dict(type='shape',shape=dict(type='cad',reference=dict(width=2*r,height=2*r),geometry=dict(type='circle',center=[r,r],radius=r)),stroke=dict(color='#cc3333',width=250))))
 for tag,left,top,width,height,geometry,color in [
  ('nurbs',100000,70000,50000,50000,dict(type='nurbs',degree=2,control=[[50000,0],[50000,50000],[0,50000]],knots=[0,0,0,1,1,1],weights=[1,2**-.5,1]),'#339966'),
  ('bezier',175000,70000,20000,20000,dict(type='bezier',points=[[0,20000],[10000,0],[20000,20000]]),'#9944aa'),
  ('ellipse',240000,70000,50000,30000,dict(type='ellipse',center=[25000,15000],major=[20000,10000],ratio=.4,start=0,sweep=6.283185307179586),'#dd6600')]:
  s['items'].append(dict(id=f'{tag}-{n}',name=tag,frame=dict(left=left,top=top,width=width,height=height),kind=dict(type='shape',shape=dict(type='cad',reference=dict(width=width,height=height),geometry=geometry),stroke=dict(color=color,width=250))))
 s['items'].append(dict(id=f'caption-{n}',name='Eğri kontrolü',frame=dict(left=30000,top=120000,width=350000,height=10000),kind=dict(type='text',content=f'R = {n+1} mm · Model dairesi R = 1000 mm · 1:50',style=dict(size=3000))))
 m=copy.deepcopy(base['sheet']['items'][-1]);m['id']=f'map-{n}';s['items'].append(m)
 fontfile=f'{file}-{weight}'+('-italic' if italic else '')+'.ttf'
 cases.append(dict(name=s['name'],sheet=s,expected=dict(text=text,postscript=postscript(ROOT/'apps/desktop/assets/fonts/drawing'/fontfile),paperCircle=dict(center=[60.,207.],radius=n+1),modelCircle=dict(center=[130.,97.],radius=20.),pageMm=[420.,297.])))
p=ROOT/'fixtures/sheet/v1/plot-document.json';dpi_cases=[]
for n,dpi in enumerate([72,96,127,150,200,254,300,400,508,600]*2):
 s=copy.deepcopy(cases[0]['sheet']);s['id']=f'raster-{n}';s['name']=f'Raster {n}';s['items']=[copy.deepcopy(base['sheet']['items'][-1])];m=s['items'][0];m['id']=f'raster-map-{n}';m['frame'].update(left=30000,top=50000,width=40000,height=20000);m['kind']['view']['center']=dict(x=1,y=.5);s['export']=dict(dpi=dpi)
 import math
 dpi_cases.append(dict(sheet=s,pixels=[math.ceil(40*dpi/25.4),math.ceil(20*dpi/25.4)]))
out=json.dumps(dict(version=1,model=model,cases=cases,dpiCases=dpi_cases),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==out
else:p.write_text(out)
