"""Independent six-sheet 2D architectural sets and differently sized shared master maps."""
import copy,json,math,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts/fixtures'))
from sheets_edit_command_cases import SETUP,sheet
cases=[]
for scale in [20.,37.5,50.,100.,200.]:
 for angle in [0,30,90,180]:
  origin=(500000.125,4500000.25) if angle in [30,90] else (0.,0.)
  model=copy.deepcopy(SETUP);model['entities']=[];model['settings']['namedViews']=[]
  def entity_text(word,x,y):
   model['entities'].append({'kind':'text','id':len(model['entities'])+1,'layerId':'cizim','attrs':{},'p':{'x':x,'y':y},'text':word,'height':.0025*scale,'rotation':angle})
  entity_text('CENTER',*origin)
  theta=math.radians(angle);far=(origin[0]+.15*scale*math.cos(theta),origin[1]+.15*scale*math.sin(theta));entity_text('FAR',*far)
  small=sheet('small','A-001 Küçük ortak sayfa',1);small['page']={'paper':'a4','orientation':'portrait','size':{'width':210000,'height':297000},'margins':dict(top=10000,right=10000,bottom=10000,left=10000)}
  m=copy.deepcopy(small['items'][0]);m.update(id='shared-map',name='Ortak model görünümü',frame=dict(left=10000,top=30000,width=190000,height=200000),constraints=dict(h='leftRight',v='top',relativeTo='margins'))
  m['kind']['view'].update(center=dict(x=origin[0],y=origin[1]),scale=1,ratio=dict(paper=1.,model=scale),rotation=angle*1000)
  small.update(items=[],master='shared');large=copy.deepcopy(small);large.update(id='large',name='A-002 Büyük ortak sayfa');large['page']['paper']='a3';large['page']['orientation']='landscape';large['page']['size']['width']=420000
  pages=[small,large]
  for n,(token,name)in enumerate([('PLAN','A-101 Kat planı'),('ELEV','A-201 Görünüş'),('SECT','A-301 Kesit'),('DETAIL','A-501 Detay')]):
   x=origin[0]+1000.*(n+1);y=origin[1];entity_text(token,x,y);model['entities'][-1]['rotation']=0.
   model['entities'].append({'kind':'line','id':len(model['entities'])+1,'layerId':'cizim','attrs':{},'a':dict(x=x-2.5,y=y-1.),'b':dict(x=x+2.5,y=y-1.)})
   d=scale/50.
   # Four manual 2D architectural drawings, in separate named model regions.
   model['entities'][-2]['p']['y']=y-2.2*d
   model['entities'][-1]['a']['y']=y-3.0*d;model['entities'][-1]['b']['y']=y-3.0*d
   def path(points,closed=True):
    model['entities'].append({'kind':'polyline','id':len(model['entities'])+1,'layerId':'cizim','attrs':{},'pts':[dict(x=x+u*d,y=y+v*d) for u,v in points],'closed':closed})
   def rectangle(l,t,r,b):path([(l,t),(r,t),(r,b),(l,b)])
   if token=='PLAN':
    rectangle(-2.5,-1.5,2.5,1.5);rectangle(-2.35,-1.35,2.35,1.35)
    path([(0,-1.35),(0,-.2)],False);path([(0,.6),(0,1.35)],False)
    rectangle(.5,-.9,1.9,.9)
    for step in range(5):path([(-2.3,-1.0+step*.4),(-1.0,-1.0+step*.4)],False)
   elif token=='ELEV':
    rectangle(-2.5,-1.5,2.5,1.5);path([(-2.8,1.5),(0,2.3),(2.8,1.5)],False)
    rectangle(-1.9,-.3,-.8,.8);rectangle(.8,-.3,1.9,.8);rectangle(-.35,-1.5,.35,.5)
   elif token=='SECT':
    rectangle(-2.7,-1.7,2.7,-1.5);rectangle(-2.7,1.5,2.7,1.7);rectangle(-2.5,-1.5,-2.25,1.5);rectangle(2.25,-1.5,2.5,1.5)
    path([(-2.25,-1.3),(0,-1.3),(0,.4),(2.25,.4)],False)
   else:
    rectangle(-.8,-1.7,-.5,1.7);rectangle(.5,-1.7,.8,1.7);rectangle(-.5,-.1,.5,.1);rectangle(-2.,-1.7,2.,-1.4)
    path([(-2.,-1.4),(-2.5,-.9),(-1.5,-.9),(-2.,-.4),(-1.,-.4)],False)
   v=dict(id=f'view-{token}',name=name,center=dict(x=x,y=y),width=.390*scale,height=.200*scale,twist=0.);model['settings']['namedViews'].append(v)
   page=sheet(token.lower(),name,1);item=page['items'][0];item['frame']=dict(left=10000,top=30000,width=390000,height=200000);item['name']=name;item['kind']['source']=dict(type='viewLink',view=copy.deepcopy(v));item['kind']['view'].update(center=dict(x=x,y=y),scale=1,ratio=dict(paper=1.,model=scale));pages.append(page)
  book=dict(schema='kentos.sheet/1',sheets=pages,masters=[dict(id='shared',name='Ortak ana sayfa',page=copy.deepcopy(small['page']),items=[m],guides=[])],assets=[],variables=[])
  cases.append(dict(name=f'set {scale} rotation {angle}',model=model,book=book,expected=dict(pages=['small','large','plan','elev','sect','detail'],words=[['CENTER'],['CENTER','FAR'],['PLAN'],['ELEV'],['SECT'],['DETAIL']],sourceCount=len(model['entities']))))
path=ROOT/'fixtures/sheet/v1/multipage-sets.json';text=json.dumps(dict(version=1,cases=cases),ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert path.read_text()==text
else:path.write_text(text)
