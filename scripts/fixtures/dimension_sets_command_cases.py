#!/usr/bin/env python3
"""ADR0695 command corpus from independent analytic wall-face arithmetic.

No KentOS module/executable is imported or run. The fixed persisted seed is
fixtures/kcad/v2/dimension-sets.json. Geometry below projects cardinal wall
faces and jambs, then constructs linear dimension rows from sorted abscissas;
paper gap is gapMm * plotScale / 1000. UUIDs of newly written objects are
wildcards or the fixture runner's slot identity, never application readback.
"""
from __future__ import annotations
import argparse
from copy import deepcopy as cp
from decimal import Decimal as D
import json
import math
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
DEST=ROOT/'fixtures/commands/v1'
SEED=json.loads((ROOT/'fixtures/kcad/v2/dimension-sets.json').read_text())
WALL=SEED['uids'][0]
MEMBER=SEED['uids'][1]
GROUP=SEED['dimensionSets'][0]['id']
OPEN='0192f5a1-6000-7000-8000-000000000004'
OTHER='0192f5a1-6000-7000-8000-00000000ffff'
CREATE='cad.dimensionsets.create'
EDIT='cad.dimensionsets.edit'
QUERY='cad.dimensionsets.query'

def point(x,y): return dict(x=float(x),y=float(y))
def ref(uid,index,part=0): return dict(uid=uid,place=dict(kind='vertex',part=part,ring=0,index=index))
def center(uid): return dict(uid=uid,place=dict(kind='center'))
def completed(output,warnings=()): return dict(status='completed',output=output,warnings=list(warnings))
def error(code,message,path,revision=None):
    out=dict(code=code,message=message,path=path)
    if revision is not None:out['revision']=revision
    return dict(status='conflict' if code=='revision_conflict' else 'failed',error=out)
def hidden(name='Gizli'):
    return dict(code='layer_hidden',message=f'“{name}” katmanı gizli; çizilen nesne görünmeyecek.',path='layerId')
def layer(id,name=None,visible=True,locked=False,kind='layer'):
    n=cp(SEED['layers'][0]);n.update(id=id,name=name or id,visible=visible,locked=locked,type=kind);return n

def fresh(opening=True,axis='east',origin=(0.,0.),length=8.,width=.25):
    d=cp(SEED);d.pop('dimensionSets',None);d['entities']=d['entities'][:1];d['uids']=[WALL]
    x,y=origin;u={'east':(1.,0.),'west':(-1.,0.),'north':(0.,1.),'south':(0.,-1.)}[axis]
    start=point(x,y);end=point(x+u[0]*length,y+u[1]*length)
    d['origin']=point(x,y);d['entities'][0].update(pts=[start,end],width=width,justify='center')
    d['layers'] += [layer('hidden','Gizli',False),layer('locked','Kilitli',True,True),layer('group','Grup',kind='group')]
    if opening:
        n=(-u[1],u[0]);a=3.5;b=4.5
        place=[point(x+u[0]*t+n[0]*h,y+u[1]*t+n[1]*h) for t,h in [(a,width/2),(a,-width/2),(b,width/2),(b,-width/2)]]
        d['entities'].append(dict(kind='opening',id=2,layerId='0',attrs={},host=WALL,at=4.,width=1.,form='opening',place=place));d['uids'].append(OPEN)
    return d

def row_geometry(d,spec,*,existing=None):
    """Independent cardinal projection: endpoints keep their actual source coordinates."""
    w=d['entities'][0];a,b=w['pts'];dx=b['x']-a['x'];dy=b['y']-a['y'];length=math.hypot(dx,dy);u=(dx/length,dy/length);n=(-u[1],u[0]);face=spec.get('face','left');sign=1 if face=='left' else -1
    line=[point(p['x']+n[0]*w['width']/2*sign,p['y']+n[1]*w['width']/2*sign) for p in (a,b)]
    part=0 if face=='left' else 1
    ends=[(0.,line[0],ref(WALL,0,part)),(length,line[1],ref(WALL,1,part))];nodes=ends.copy()
    for uid,e in zip(d['uids'],d['entities']):
        if e['kind']!='opening':continue
        if spec.get('openings','ends')=='centers':
            p=point(sum(q['x'] for q in e['place'])/4,sum(q['y'] for q in e['place'])/4);nodes.append(((p['x']-line[0]['x'])*u[0]+(p['y']-line[0]['y'])*u[1],p,center(uid)))
        else:
            for i in ([0,2] if face=='left' else [1,3]):
                p=cp(e['place'][i]);nodes.append(((p['x']-line[0]['x'])*u[0]+(p['y']-line[0]['y'])*u[1],p,ref(uid,i)))
    nodes.sort(key=lambda v:v[0]);nodes=[v for i,v in enumerate(nodes) if not i or v[0]!=nodes[i-1][0]]
    at=spec['at'];offset=(at['x']-line[0]['x'])*n[0]+(at['y']-line[0]['y'])*n[1];side=-1 if offset<0 else 1
    rows=spec.get('rows',['openings','overall']);gap=float(D(str(spec.get('gapMm',7.5)))*D(str(d['settings']['plotScale']))/D(1000));places={r['row']:r['at'] for r in spec.get('rowPlaces',[])}
    suppressed=spec.get('suppressed',[]);entities=[];spans=[];uids=[]
    for i,row in enumerate(rows):
        row_at=places.get(row,point(at['x']+n[0]*side*gap*i,at['y']+n[1]*side*gap*i))
        chosen=ends if row=='overall' else nodes
        for first,last in zip(chosen,chosen[1:]):
            span=dict(row=row,a=first[2],b=last[2])
            if span in suppressed:continue
            known=next((m for m in spec.get('members',[]) if all(m[k]==span[k] for k in ('row','a','b'))),None)
            old=existing if known is not None else None
            e=cp(old) if old else dict(kind='dimension',id=0,layerId=spec.get('layerId','0'),attrs={},height=spec.get('height',float(D('2.5')*D(str(d['settings']['plotScale']))/D(1000))))
            e.update(a=cp(first[1]),b=cp(last[1]),offset=(row_at['x']-first[1]['x'])*n[0]+(row_at['y']-first[1]['y'])*n[1],style='linear',angle=math.degrees(math.atan2(u[1],u[0])),refs=dict(a=cp(first[2]),b=cp(last[2])))
            if not old:e.update(cp(spec.get('look',{})))
            if spec.get('_newLayer'):e['layerId']=spec['layerId']
            if '_newLook' in spec:
                for key in ('dimLineColor','arrowSize','dimStyle'):e.pop(key,None)
                e.update(cp(spec['_newLook']));e['height']=spec['height']
            entities.append(e);spans.append(span)
            if known:uids.append(known['uid'])
    return entities,spans,uids

def plan_output(d,spec,existing=None,gid=None):
    entities,spans,uids=row_geometry(d,spec,existing=existing)
    removed=[m['uid'] for m in spec.get('members',[]) if m['uid'] not in uids]
    out=dict(entities=entities,spans=spans,removed=removed,uids=uids,revision='$current')
    if gid:out['set']=gid
    return out

def state(d,*,changed=False,metadata=None):
    expect=dict(ids=[e['id'] for e in d['entities']],revision='changed' if changed else 'same',canUndo=changed,dirty=changed)
    if metadata is not None:expect['dimensionSets']=metadata
    return expect

def step(op,input,result,expect=None,command=None):
    out=dict(op=op,input=input,result=result,within=1e-9)
    if expect is not None:out['expect']=expect
    if command:out['command']=command
    return out

def case(name,d,*steps):return dict(name=name,setup=d,steps=list(steps))

def create_cases():
    out=[]
    variants=[('sol yüz ve iki sıra',fresh(),{}),('sağ yüz, dışa aşağı sıralar',fresh(),dict(face='right',at=point(4,-1))),('yalnız açıklık aralıkları',fresh(),dict(rows=['openings'])),('yalnız toplam',fresh(),dict(rows=['overall'])),('sıraların kullanıcı sırası',fresh(),dict(rows=['overall','openings'])),('açıklık merkezlerinden',fresh(),dict(openings='centers')),('açıklık yokken yüzün bütünü',fresh(False),{}),('ters yönlü batıya duvar',fresh(axis='west'),{}),('düşey kuzey duvar',fresh(axis='north'),dict(at=point(-1,4))),('düşey güneye duvar',fresh(axis='south'),dict(at=point(1,-4))),('büyük dünya koordinatları',fresh(origin=(500000.,4400000.)),dict(at=point(500004,4400001))),('kâğıt aralığı 6mm',fresh(),dict(gapMm=6.)),('plotScale100 aralığı',fresh(),dict(gapMm=8.))]
    variants[-1][1]['settings']['plotScale']=100.
    for name,d,extra in variants:
        inp=dict(layerId='0',wall=WALL,at=point(4,1),**{})
        inp.update(extra);out.append(case('Plan: '+name,d,step('plan',inp,completed(plan_output(d,inp)),state(d,metadata=[]))))
    for rows in [['overall'],['openings'],['openings','overall']]:
        d=fresh();inp=dict(layerId='0',wall=WALL,at=point(4,1),rows=rows)
        e,sp,_=row_geometry(d,inp);ids=list(range(3,3+len(e)));fields={str(i):{k:v for k,v in entity.items() if k in ('a','b','height','offset','refs','style','angle')} for i,entity in zip(ids,e)}
        result=completed(dict(set='$any',uids=['$any']*len(e),created=['$any']*len(e),removed=[],changed=[],revision='$current'))
        made=dict(ids=[1,2]+ids,fields=fields,canUndo=True,dirty=True,revision='changed')
        out.append(case('Oluştur ve tek geri/ileri: '+','.join(rows),d,step('execute',inp,result,made),dict(op='undo',expect=dict(ids=[1,2],dimensionSets=[],canUndo=False,canRedo=True)),dict(op='redo',expect=dict(ids=[1,2]+ids,fields=fields,canUndo=True))))
    for key,value in [('gapMm',0.),('gapMm',-1.),('rows',[]),('rows',['overall','overall'])]:
        d=fresh();inp=dict(layerId='0',wall=WALL,at=point(4,1));inp[key]=value
        message='Ölçü dizisinin sıraları boş veya yineleniyor.' if key=='rows' else 'Ölçü dizisinin yeri sonlu, aralığı ve yazı yüksekliği pozitif olmalı.'
        out.append(case(f'Ret: {key}={value}',d,step('execute',inp,error('invalid_dimension_set',message,'operation'),state(d,metadata=[]))))
    for target,code,message in [('missing','layer_not_found','“missing” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.'),('locked','layer_locked','“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.'),('group','not_a_layer','“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.')]:
        d=fresh();out.append(case('Ret: hedef '+target,d,step('validate',dict(layerId=target,wall=WALL,at=point(4,1)),error(code,message,'layerId'),state(d))))
    for host,message in [(OTHER,'Kaynak duvar bulunamadı. Başka bir duvar seçin.'),(OPEN,'Bu araç şu anda iki uçlu düz duvarı ölçer. Kapalı, çok parçalı veya yaylı duvar desteklenmiyor.')]:
        d=fresh();out.append(case('Ret: kaynak '+host,d,step('plan',dict(layerId='0',wall=host,at=point(4,1)),error('invalid_dimension_set',message,'wall'),state(d))))
    d=fresh();inp=dict(layerId='hidden',wall=WALL,at=point(4,1));out.append(case('Gizli hedef planında görünür uyarı',d,step('plan',inp,completed(plan_output(d,inp),[hidden()]),state(d))))
    for key,value in [('dimensionStyle','Kayıp'),('expectedRevision','07'),('expectedRevision','999')]:
        d=fresh();inp=dict(layerId='0',wall=WALL,at=point(4,1));inp[key]=value
        if key=='dimensionStyle':res=error('unknown_style','“Kayıp” adlı ölçü stili yok. Projenin ölçü stillerinden birinin adını verin ya da alanı kaldırın (Standart).','dimensionStyle')
        elif value=='07':res=error('invalid_revision','Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.','expectedRevision')
        else:res=error('revision_conflict','Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.','expectedRevision','$current')
        out.append(case('Ret: '+key+' '+value,d,step('execute',inp,res,state(d))))
    d=fresh();v=step('plan',dict(layerId='0',wall=WALL,at=point(4,1)),error('not_finite','Ölçü sırasının yeri doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.','at.x'),state(d));v['nonFinite']={'at.x':'NaN'};out.append(case('Koordinat sonlu değil',d,v))
    return out

def edit_cases():
    out=[]
    base=cp(SEED);base['layers'] += [layer('hidden','Gizli',False),layer('locked','Kilitli',True,True),layer('group','Grup',kind='group')]
    def add(name,operation,change,*,mode='execute',seed=None,look=None):
        d=cp(seed or base);after=cp(d['dimensionSets'][0]);change(after);spec=cp(after)
        if operation.get('kind')=='set' and 'layerId' in operation:spec['_newLayer']=True
        if look is not None:spec['_newLook']=look
        entities,spans,kept=row_geometry(d,spec,existing=d['entities'][1]);removed=[m['uid'] for m in d['dimensionSets'][0].get('members',[]) if m['uid'] not in kept]
        inp=dict(id=GROUP,operation=operation)
        if mode=='plan':
            result=plan_output(d,spec,existing=d['entities'][1],gid=GROUP);result['removed']=removed
            out.append(case(name,d,step('plan',inp,completed(result),state(d))));return
        # New children use slots after the fixed seed; current members retain slot2.
        next_slot=3;new=[];members=[];fields={};written=[]
        for entity,span in zip(entities,spans):
            member=next((m for m in d['dimensionSets'][0].get('members',[]) if all(m[k]==span[k] for k in ('row','a','b'))),None)
            uid=member['uid'] if member else f'$uidOf:{next_slot}'
            slot=2 if member else next_slot
            if not member:next_slot+=1;new.append('$any')
            fields[str(slot)]={k:v for k,v in entity.items() if k in ('a','b','offset','height','refs','textShift','dimStyle','dimLineColor','arrowSize','layerId')}
            members.append(dict(uid=uid,**cp(span)));written.append(slot)
        after['members']=members
        for key in ('members','suppressed','rowPlaces'):
            if not after.get(key):after.pop(key,None)
        preserved=next((entity for entity,span in zip(entities,spans) if any(m['uid']==MEMBER and all(m[k]==span[k] for k in ('row','a','b')) for m in d['dimensionSets'][0].get('members',[]))),None)
        changed=[MEMBER] if preserved is not None and preserved!=d['entities'][1] else []
        result=completed(dict(set=GROUP,uids=['$any']*len(entities),created=new,removed=removed,changed=changed,revision='$current'))
        expect=dict(ids=[1]+sorted(written),fields=fields,dimensionSets=[after],canUndo=True,dirty=True,revision='changed')
        out.append(case(name,d,step('execute',inp,result,expect),dict(op='undo',expect=dict(ids=[1,2],dimensionSets=d['dimensionSets'],fields={'2':{'a':d['entities'][1]['a'],'b':d['entities'][1]['b'],'offset':d['entities'][1]['offset']}}))))
    add('Plan: sıra yeri',dict(kind='move',row='overall',at=point(2,4)),lambda s:s.update(rowPlaces=[dict(row='overall',at=point(2,4))]),mode='plan')
    add('Grubu dünya düzleminde bir metre taşı',dict(kind='move',at=point(0,2)),lambda s:s.update(at=point(0,2),rowPlaces=[dict(row='overall',at=point(2,3))]))
    add('Yalnız toplam sırasını taşı',dict(kind='move',row='overall',at=point(2,4)),lambda s:s.update(rowPlaces=[dict(row='overall',at=point(2,4))]))
    add('Yüzden2m sıra uzaklığı',dict(kind='rowOffset',row='overall',offset=2.),lambda s:s.update(rowPlaces=[dict(row='overall',at=point(2,2.125))]))
    add('Yüzün ters tarafında eksi2m sıra uzaklığı',dict(kind='rowOffset',row='overall',offset=-2.),lambda s:s.update(rowPlaces=[dict(row='overall',at=point(2,-1.875))]))
    add('Elle sıra yerlerini sıfırla',dict(kind='resetRows'),lambda s:s.pop('rowPlaces',None))
    add('Aralık metadata değişir, elle sıra yeri korunur',dict(kind='set',gapMm=8.),lambda s:s.update(gapMm=8.))
    add('Yalnız toplam sırasına geç',dict(kind='set',rows=['overall']),lambda s:(s.update(rows=['overall']),s.pop('suppressed',None)))
    add('Gizlenmiş açıklık sırası tek başına, aktif ölçü kalmaz',dict(kind='set',rows=['openings']),lambda s:(s.update(rows=['openings']),s.pop('rowPlaces',None)))
    add('Bastırılmış aralığı geri getir',dict(kind='restoreHidden'),lambda s:s.pop('suppressed',None))
    add('Sağ fiziksel yüze geç, kaynak anahtarı değişir',dict(kind='set',face='right'),lambda s:s.update(face='right'))
    add('Kaynak seçiminde merkez modu metadata korunur',dict(kind='set',openings='centers'),lambda s:s.update(openings='centers'))
    add('Plan: sağ yüz yeni öğeler ve eski UUID silinmesi',dict(kind='set',face='right'),lambda s:s.update(face='right'),mode='plan')
    for kind in ['detach','delete']:
        for op in ['plan','execute']:
            d=cp(base);inp=dict(id=GROUP,operation=dict(kind=kind));uids=[MEMBER] if kind=='detach' else [];removed=[MEMBER] if kind=='delete' else []
            result=completed(dict(set=GROUP,entities=[],spans=[],removed=removed,uids=uids,revision='$current')) if op=='plan' else completed(dict(set=GROUP,uids=uids,created=[],removed=removed,changed=[],revision='$current'))
            expect=state(d) if op=='plan' else dict(ids=[1,2] if kind=='detach' else [1],dimensionSets=[],canUndo=True,revision='changed')
            steps=[step(op,inp,result,expect)]
            if op=='execute':steps.append(dict(op='undo',expect=dict(ids=[1,2],dimensionSets=d['dimensionSets'])))
            out.append(case(f'{kind}: {op}, tek geri adım',d,*steps))
    for op in ['plan','execute']:
        d=cp(base);d['entities'][1]['textShift']=dict(along=.3,across=.2);entity=cp(d['entities'][1]);entity.pop('textShift')
        if op=='plan':result=completed(dict(set=GROUP,entities=[entity],spans=[{k:d['dimensionSets'][0]['members'][0][k] for k in ('row','a','b')}],removed=[],uids=[MEMBER],revision='$current'));expect=state(d)
        else:result=completed(dict(set=GROUP,uids=[MEMBER],created=[],removed=[],changed=[MEMBER],revision='$current'));expect=dict(ids=[1,2],fields={'2':{'textShift':None}},dimensionSets=d['dimensionSets'],canUndo=True,revision='changed')
        out.append(case(f'Yazı yerini sıfırla: {op}',d,step(op,dict(id=GROUP,operation=dict(kind='resetText',row='overall')),result,expect)))
    d=cp(base);d['settings']['dimensionStyles']=[dict(id='arch',name='Mimari',height=3.)]
    add('Stili değiştir, mevcut üyeye aynı görünüş/ölçü',dict(kind='set',dimensionStyle='Mimari'),lambda s:s.update(look=dict(dimStyle='arch'),height=.15),seed=d,look=dict(dimStyle='arch'))
    add('Hedef katmanı değiştir, yer ve kaynak aynı',dict(kind='set',layerId='hidden'),lambda s:s.update(layerId='hidden'))
    # This last case requires the target-layer visibility warning.
    out[-1]['steps'][0]['result']['warnings']=[hidden()]
    d=cp(base);inp=dict(id=OTHER,operation=dict(kind='delete'));out.append(case('Bilinmeyen grup yazmadan ret',d,step('execute',inp,error('dimension_set_not_found','Ölçü grubu bulunamadı; yeniden seçin.','id'),state(d))))
    d=cp(base);out.append(case('Boş sıra listesi atomik ret',d,step('execute',dict(id=GROUP,operation=dict(kind='set',rows=[])),error('invalid_dimension_set','Ölçü dizisinin sıraları boş veya yineleniyor.','operation'),state(d))))
    d=cp(base);d['layers'][0]['locked']=True;out.append(case('Kilitli ölçü katmanında detach ret',d,step('execute',dict(id=GROUP,operation=dict(kind='detach')),error('layer_locked','“0” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.','layerId'),state(d))))
    for key,value in [('dimensionStyle','Kayıp'),('layerId','missing'),('layerId','group'),('expectedRevision','07'),('expectedRevision','999')]:
        d=cp(base);inp=dict(id=GROUP,operation=dict(kind='set',gapMm=8.))
        if key=='expectedRevision':inp[key]=value
        else:inp['operation']={"kind":"set",key:value}
        if key=='dimensionStyle':res=error('unknown_style','“Kayıp” adlı ölçü stili yok. Projenin ölçü stillerinden birinin adını verin ya da alanı kaldırın (Standart).','dimensionStyle')
        elif key=='layerId' and value=='missing':res=error('layer_not_found','“missing” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.','layerId')
        elif key=='layerId':res=error('not_a_layer','“Grup” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.','layerId')
        elif value=='07':res=error('invalid_revision','Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.','expectedRevision')
        else:res=error('revision_conflict','Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.','expectedRevision','$current')
        out.append(case('Ret: '+key+' '+value,d,step('execute',inp,res,state(d))))
    d=cp(base);v=step('plan',dict(id=GROUP,operation=dict(kind='move',at=point(2,4))),error('not_finite','Ölçü sırasının yeri doğu (Y) değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.','operation.at.x'),state(d));v['nonFinite']={'operation.at.x':'NaN'};out.append(case('Sıra konumu sonlu değil',d,v))
    return out

def query_cases():
    out=[]
    def add(name,d,inp,expected,op='execute'):
        result=completed(None if op=='validate' else dict(sets=expected,revision='$current'))
        out.append(case(name,d,step(op,inp,result,dict(ids=[e['id'] for e in d['entities']],dimensionSets=d.get('dimensionSets',[]),canUndo=False,dirty=False,revision='same'))))
    for op in ['validate','plan','execute']:
        d=cp(SEED);add('Tam kalıcı metadata, salt okunur '+op,d,{},d['dimensionSets'],op)
    for inp,name in [(dict(ids=[GROUP]),'Kimlikten'),(dict(wall=WALL),'Kaynak duvardan'),(dict(ids=[GROUP],wall=WALL),'Kimlik ve duvar kesişimi'),(dict(ids=[]),'Boş kimlik filtresi tümü'),(dict(ids=[GROUP,GROUP]),'Yinelenen kimlik tek sonuç'),(dict(ids=[OTHER,GROUP]),'Bilinmeyen ve bilinen kimlik birlikte')]:
        d=cp(SEED);add(name,d,inp,d['dimensionSets'])
    for inp,name in [(dict(ids=[OTHER]),'Bilinmeyen kimlik boş'),(dict(wall=OTHER),'Bilinmeyen duvar boş'),(dict(ids=[GROUP],wall=OTHER),'Filtrelerin kesişimi boş'),(dict(ids=[OTHER],wall=WALL),'Duvar eşleşse kimlik eşleşmez')]:
        d=cp(SEED);add(name,d,inp,[])
    d=cp(SEED);d.pop('dimensionSets');add('Tarif yoksa ölçü tek başına grup olmaz',d,{},[])
    d=cp(SEED);d['layers'][0]['visible']=False;add('Gizli katman sorgudan düşmez',d,{},d['dimensionSets'])
    d=cp(SEED);d['layers'][0]['locked']=True;add('Kilitli katman salt okunur sorguya engel değil',d,{},d['dimensionSets'])
    # File-order slots restart at 1; the absent host and surviving member keep their persistent UUIDs.
    d=cp(SEED);d['entities']=d['entities'][1:];d['entities'][0]['id']=1;d['uids']=d['uids'][1:];d['dimensionSets'][0]['broken']='hostGone';add('Kaynak kayıp tarifi ve üyeler korunur',d,{},d['dimensionSets'])
    d=cp(SEED);d['entities'][0]['closed']=True;d['entities'][0]['pts'].append(point(8,4));d['dimensionSets'][0]['broken']='unsupportedWall';add('Desteklenmeyen kaynak gerekçesi korunur',d,{},d['dimensionSets'])
    d=cp(SEED);d['dimensionSets'][0]['rowPlaces']=[dict(row='overall',at=point(2,-2))];d['entities'][1]['offset']=-2.125;add('Negatif elle sıra yeri tam metadata',d,{},d['dimensionSets'])
    d=cp(SEED);d['dimensionSets'][0]['look']=dict(dimLineColor='#ABCDEF',arrowSize=1.2,decimals=4);add('Stil şablonu gerçek üye görünüşünden ayrı korunur',d,{},d['dimensionSets'])
    d=cp(SEED);d['dimensionSets'][0]['openings']='centers';add('Merkez kaynak seçimi korunur',d,{},d['dimensionSets'])
    d=cp(SEED);d['dimensionSets'][0]['rows']=['overall'];d['dimensionSets'][0].pop('suppressed');add('Tek sıra sorgusu kalıcı sıra seçimini korur',d,{},d['dimensionSets'])
    d=cp(SEED);dx,dy=500000.,4400000.
    for e in d['entities']:
        for p in e.get('pts',[])+[e[k] for k in ('a','b') if k in e]:p['x']+=dx;p['y']+=dy
    for p in [d['dimensionSets'][0]['at']]+[r['at'] for r in d['dimensionSets'][0]['rowPlaces']]:p['x']+=dx;p['y']+=dy
    add('Büyük koordinatlı yerleştirme tam hassasiyetle korunur',d,{},d['dimensionSets'])
    return out

def build():
    out={}
    for suffix,cases in [('create',create_cases()),('edit',edit_cases()),('query',query_cases())]:
        assert len(cases)>=20
        common=cp(cases[0]['setup'])
        for case in cases:
            if case['setup']==common:case.pop('setup')
        out[DEST/f'cad.dimensionsets.{suffix}.json']=dict(format='kentos.command-cases',version=1,command='cad.dimensionsets.'+suffix,commandVersion=1,title='Kalıcı otomatik mimari ölçü dizileri: '+suffix,note='ADR0695. Bağımsız kardinal yüz/pervaz hesabı; V2 sabit UUID seed; üretici KentOS kodunu çalıştırmaz. Yeni kimlikler yalnız yazma anında atanır.',setup=common,cases=cases)
    return out

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args();bad=[]
    for path,value in build().items():
        text=json.dumps(value,ensure_ascii=False,indent=2)+'\n'
        if args.check:
            if not path.exists() or path.read_text()!=text:bad.append(str(path.relative_to(ROOT)))
        else:path.write_text(text);print(path.relative_to(ROOT),len(value['cases']),'bağımsız durum')
    if bad:print('Güncel değil: '+', '.join(bad));return 1
    return 0
if __name__=='__main__':raise SystemExit(main())
