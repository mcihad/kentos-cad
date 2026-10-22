"""Independent viewport camera/ratio cases. Metres and micrometres; no KentOS calls."""
import copy,json,math
from pathlib import Path
from sheets_edit_command_cases import sheet
out=[]
base=sheet('page','Görünüm',100)
i=base['items'][0];i['id']='viewport';i['padding']=2000;i['kind']['labelBand']=3000
# Effective content: 380 x 220 mm.
center=[2.5,0.];size=[380000,220000]
def add(name,request,before=None,error=None):
    s=copy.deepcopy(before or base);item=s['items'][0];v=item['kind']['view'];c=[v['center']['x'],v['center']['y']];r=v.get('ratio',dict(paper=1,model=v['scale']));rotation=v['rotation']
    source=request.get('source',item['kind'].get('source',dict(type='drawing')));locked=request.get('locked',item['kind'].get('viewLocked',False))
    if not error:
        if source['type']=='viewSnapshot' and 'source' in request:
            named=source['view'];c=[named['center']['x'],named['center']['y']];rotation=round(math.degrees(named['twist'])*1000)%360000;r=dict(paper=1,model=max(named['width']*1e6/size[0],named['height']*1e6/size[1]))
        rotation=request.get('rotation',rotation)%360000
        if 'bounds' in request:
            x,y,X,Y=request['bounds'];c=[(x+X)/2,(y+Y)/2];a=math.radians(rotation/1000);co=abs(math.cos(a));sn=abs(math.sin(a));r=dict(paper=1,model=max(((X-x)*co+(Y-y)*sn)*1e6/size[0],((X-x)*sn+(Y-y)*co)*1e6/size[1]));source=dict(type='drawing')
        if 'center' in request:c=[request['center']['x'],request['center']['y']]
        if 'scale' in request:r=dict(paper=1,model=request['scale'])
        r=request.get('ratio',r)
        a=math.radians(rotation/1000);co=math.cos(a);sn=math.sin(a);d=r['model']/r['paper'];corners=[]
        for x,y in [(-size[0]/2,-size[1]/2),(size[0]/2,-size[1]/2),(size[0]/2,size[1]/2),(-size[0]/2,size[1]/2)]:corners.append([c[0]+(x*co+y*sn)*d/1e6,c[1]+(x*sn-y*co)*d/1e6])
        expected=dict(center=c,denominator=d,rotation=rotation,corners=corners,bounds=[min(p[0] for p in corners),min(p[1] for p in corners),max(p[0] for p in corners),max(p[1] for p in corners)],locked=locked,source=source)
    else:expected=dict(error=error)
    out.append(dict(name=name,sheet=s,operation=dict(op='setViewport',id='viewport',**request),expected=expected))
for paper,model in [(1,20),(1,37.5),(1,100),(2,1),(3,2),(1,100000),(1,1),(100,1)]:add(f'Oran {paper}:{model}',dict(ratio=dict(paper=paper,model=model)))
for rotation in [30000,90000,270000,-45000]:add(f'Dönüş {rotation}',dict(rotation=rotation,center=dict(x=20.5,y=-30.25),ratio=dict(paper=1,model=37.5)))
for bounds,rotation in [([0,0,5,3],0),([487150,4417820,487155,4417823],0),([0,0,5,3],30000),([-7,-4,6,8],90000)]:add(f'Kapsam {bounds} {rotation}',dict(bounds=bounds,rotation=rotation))
for width,height,twist in [(5,3,0),(10,2,math.pi/6),(.1,.15,math.pi/2)]:
    view=dict(id='view-plan',name='Kat planı',center=dict(x=487150,y=4417820),width=width,height=height,twist=twist)
    add(f'Adlı görünüm {twist}',dict(source=dict(type='viewSnapshot',view=view)))
add('Kamerayı kilitle',dict(locked=True))
locked=copy.deepcopy(base);locked['items'][0]['kind']['viewLocked']=True
add('Kilitli kamerada oran reddi',dict(ratio=dict(paper=1,model=50)),locked,'item_locked')
add('Kilidi açarak kamerayı değiştir',dict(locked=False,ratio=dict(paper=2,model=1)),locked)
ratio=copy.deepcopy(base);ratio['items'][0]['kind']['view']['ratio']=dict(paper=2,model=1)
add('1:N ayarı önceki oranı açıkça kaldırır',dict(scale=50),ratio)
named=copy.deepcopy(base);named['items'][0]['kind']['source']=dict(type='viewSnapshot',view=view)
add('Kaynak yeniden çizim olur',dict(source=dict(type='drawing')),named)
for ratio in [dict(paper=0,model=1),dict(paper=-1,model=2),dict(paper=1,model=1e9)]:add(f'Geçersiz oran {ratio}',dict(ratio=ratio),error='out_of_range')
add('Merkez ve kapsam çelişir',dict(bounds=[0,0,1,1],center=dict(x=0,y=0)),error='ambiguous_source')
add('Oran ve 1:N çelişir',dict(scale=50,ratio=dict(paper=1,model=100)),error='ambiguous_source')
add('Ters kapsam',dict(bounds=[1,1,0,0]),error='invalid_extent')
add('Nokta kapsamı',dict(bounds=[1,1,1,1]),error='invalid_extent')
add('Görünüm ve merkez çelişir',dict(source=dict(type='viewSnapshot',view=view),center=dict(x=0,y=0)),error='ambiguous_source')
# CAD-SHEET-11: retain the explicit pair, including more digits than UI display precision.
for paper,model in [(10,1),(5,1),(2,1),(1,1),(1,2),(1,5),(1,10),(1,20),(1,25),(1,50),(1,75),(1,100),(1,200),(1,250),(1,500),(1,1000),(1,2000),(1,5000),(1,37.123456789123),(3,2),(7,13),(0.123456789,17.987654321),(1,1e-6),(1,1e8)]:
    add(f'Tam saklanan CAD oranı {paper}:{model}',dict(ratio=dict(paper=paper,model=model)))
    out[-1]['expected']['ratio']=dict(paper=paper,model=model)
Path('fixtures/sheet/v1/viewports.json').write_text(json.dumps(dict(schema='kentos.sheet.viewports/1',cases=out),ensure_ascii=False,indent=2)+'\n')
