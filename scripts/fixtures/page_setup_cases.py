"""Independent paper-to-output affine expectations; no KentOS runtime."""
import copy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts/fixtures'))
from sheets_edit_command_cases import sheet,SETUP,stroke
cases=[]
for n in range(20):
 s=sheet('page','Baskı kontrolü',50);s['items']=[]
 s['page']['margins']=dict(top=10000,right=15000,bottom=20000,left=20000)
 # Ink frames determine extents: exactly 100 x 80 mm at (50,40).
 s['items']=[dict(id='box',name='100 × 80 mm',frame=dict(left=50000,top=40000,width=100000,height=80000),kind=dict(type='shape',shape=dict(type='rect',radius=0),stroke=stroke('#2255aa',350))),dict(id='mark',name='Ölçü yazısı',frame=dict(left=70000,top=60000,width=40000,height=10000),kind=dict(type='text',content='PRINT',style=dict(size=3000)))]
 area=['layout','window','extents','window','extents'][n%5];fit=n%5>=3;factor=[1.,.5,1.,1.5][n//5]
 settings=dict(area=dict(type=area),scale=dict(type='fit') if fit else dict(type='ratio',ratio=dict(paper=factor,model=1.)),offset=[(n//5)*1000,-(n//5)*500],center=n%2==1,device=dict(type='pdf'))
 if n//5==3:settings['device']=dict(type='system',name='Ofis yazıcısı')
 source=[0,0,420000,297000] if area=='layout' else [50000,40000,100000,80000]
 if area=='window':settings['area']['rect']=dict(zip(['left','top','width','height'],source))
 clip=[20000,10000,385000,267000]
 if area=='layout':f=1.;tx,ty=settings['offset']
 else:
  f=min(clip[2]/source[2],clip[3]/source[3]) if fit else factor
  cx=(clip[2]-source[2]*f)/2 if settings['center'] else 0
  cy=(clip[3]-source[3]*f)/2 if settings['center'] else 0
  tx=clip[0]+cx+settings['offset'][0]-source[0]*f;ty=clip[1]+cy+settings['offset'][1]-source[1]*f
 # First baseline is measured by core text layout; assert PDF rectangle corners/width separately.
 setup=dict(id='office',name='Ofis baskısı',page=copy.deepcopy(s['page']),settings=settings)
 cases.append(dict(name=f'{n}: {area} fit={fit}',model=copy.deepcopy(SETUP),sheet=s,setup=setup,expected=dict(source=source,clip=clip,factor=f,translate=[tx,ty],box=[50000*f+tx,40000*f+ty,100000*f,80000*f])))
p=ROOT/'fixtures/sheet/v1/page-setups.json';text=json.dumps(dict(version=1,cases=cases),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
