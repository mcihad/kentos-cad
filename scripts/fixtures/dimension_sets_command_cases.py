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
        if e['kind']!='opening' or not on_face(e,face):continue
        if spec.get('openings','ends')=='centers':
            p=point(sum(q['x'] for q in e['place'])/4,sum(q['y'] for q in e['place'])/4);nodes.append(((p['x']-line[0]['x'])*u[0]+(p['y']-line[0]['y'])*u[1],p,center(uid)))
        else:
            for i in ([0,2] if face=='left' else [1,3]):
                p=cp(e['place'][i]);nodes.append(((p['x']-line[0]['x'])*u[0]+(p['y']-line[0]['y'])*u[1],p,ref(uid,i)))
    nodes.sort(key=lambda v:v[0]);nodes=[v for i,v in enumerate(nodes) if not i or v[0]!=nodes[i-1][0]]
    at=spec['at'];offset=(at['x']-line[0]['x'])*n[0]+(at['y']-line[0]['y'])*n[1];side=-1 if offset<0 else 1
    rows=spec.get('rows',['openings','overall']);gap=float(D(str(spec.get('gapMm',7.5)))*D(str(d['settings']['plotScale']))/D(1000));places={r['row']:r['at'] for r in spec.get('rowPlaces',[])}
    suppressed=spec.get('suppressed',[]);entities=[];spans=[];uids=[];drawn=[]
    for i,row in enumerate(rows):
        row_at=places.get(row,point(at['x']+n[0]*side*gap*i,at['y']+n[1]*side*gap*i))
        chosen=ends if row=='overall' else nodes
        for first,last in zip(chosen,chosen[1:]):
            span=dict(row=row,a=first[2],b=last[2])
            # A span measuring the same two places as one drawn before it is not drawn twice (an edge without
            # openings); a removed one is not drawn, so the other row's stays.
            if span in suppressed or (span['a'],span['b']) in drawn:continue
            drawn.append((span['a'],span['b']))
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
    for host,message in [(OTHER,'Kaynak duvar bulunamadı. Başka bir duvar seçin.'),(OPEN,'Kaynak bir duvar değil. Ölçülecek duvarı seçin.')]:
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

# ADR0700: closed and multi-edge walls. Face polygons are mitered offsets of the
# path, built here by intersecting offset lines (no KentOS code); a face vertex
# i belongs to path vertex i, an edge k runs from vertex k to k+1 (closed: wraps).
ROOM='0192f5a1-6000-7000-8000-000000000101'
DOOR='0192f5a1-6000-7000-8000-000000000102'
WINDOW='0192f5a1-6000-7000-8000-000000000103'
NICHE='0192f5a1-6000-7000-8000-000000000104'
FLUE='0192f5a1-6000-7000-8000-000000000105'

def on_face(e,face):
    """Whether an opening shows on the wall's `face` (ADR 0719 §4): a flue on none, a niche on its own only."""
    if e.get('form')=='flue':return False
    return e.get('form')!='niche' or e.get('side','left')==face

def unit(a,b):
    dx=b['x']-a['x'];dy=b['y']-a['y'];l=math.hypot(dx,dy);return (dx/l,dy/l),l

def face_polygon(pts,closed,h):
    """The face h to the left (h<0: right) of the path, its corners mitered."""
    n=len(pts);edges=n if closed else n-1;out=[]
    def offset_line(k):
        (ux,uy),_=unit(pts[k],pts[(k+1)%n]);nx,ny=-uy,ux
        return (pts[k]['x']+nx*h,pts[k]['y']+ny*h),(ux,uy)
    for i in range(n):
        if not closed and i==0:
            (px,py),_=offset_line(0);out.append(point(px,py));continue
        if not closed and i==n-1:
            (px,py),(ux,uy)=offset_line(n-2);_,l=unit(pts[n-2],pts[n-1]);out.append(point(px+ux*l,py+uy*l));continue
        (p1,u1),(p2,u2)=offset_line((i-1)%n),offset_line(i%edges)
        # p1 + t*u1 = p2 + s*u2
        det=u1[0]*(-u2[1])-u1[1]*(-u2[0]);rx=p2[0]-p1[0];ry=p2[1]-p1[1]
        t=(rx*(-u2[1])-ry*(-u2[0]))/det;out.append(point(p1[0]+u1[0]*t,p1[1]+u1[1]*t))
    return out

def ccw(pts):
    a=sum(pts[i]['x']*pts[(i+1)%len(pts)]['y']-pts[(i+1)%len(pts)]['x']*pts[i]['y'] for i in range(len(pts)));return a>0

def walled(pts,closed=True,width=.25,openings=(),bulges=None):
    """A drawing of one wall `pts` and its openings, each (edge, centre from the edge's start, width, uid, form)."""
    d=cp(SEED);d.pop('dimensionSets',None);d['entities']=d['entities'][:1];d['uids']=[ROOM]
    d['origin']=cp(pts[0]);d['entities'][0].update(pts=cp(pts),width=width,justify='center')
    if closed:d['entities'][0]['closed']=True
    if bulges:d['entities'][0]['bulges']=bulges
    d['layers'] += [layer('hidden','Gizli',False),layer('locked','Kilitli',True,True),layer('group','Grup',kind='group')]
    n=len(pts);start=[0.]
    for k in range(n if closed else n-1):start.append(start[-1]+unit(pts[k],pts[(k+1)%n])[1])
    for i,(edge,c,ow,uid,form,*extra) in enumerate(openings):
        (ux,uy),_=unit(pts[edge],pts[(edge+1)%n]);nx,ny=-uy,ux;v=pts[edge];t=width/2
        place=[point(v['x']+ux*a+nx*o,v['y']+uy*a+ny*o) for a,o in [(c-ow/2,t),(c-ow/2,-t),(c+ow/2,t),(c+ow/2,-t)]]
        d['entities'].append(dict(kind='opening',id=2+i,layerId='0',attrs={},host=ROOM,at=start[edge]+c,width=ow,form=form,**(extra[0] if extra else {}),place=place))
        d['uids'].append(uid)
    return d

def edge_geometry(d,edge,face,at,*,rows=('openings','overall'),mode='ends',gap_mm=7.5):
    """One set's dimensions on `edge`, `face` ('left'/'right'), first row through `at`: entities and spans."""
    w=d['entities'][0];pts=w['pts'];closed=w.get('closed',False);n=len(pts);t=w['width']/2
    poly=face_polygon(pts,closed,t if face=='left' else -t);part=0 if face=='left' else 1
    a,b=poly[edge],poly[(edge+1)%n];(ux,uy),length=unit(a,b);nx,ny=-uy,ux
    along=lambda p:(p['x']-a['x'])*ux+(p['y']-a['y'])*uy
    ends=[(0.,a,ref(ROOM,edge,part)),(length,b,ref(ROOM,(edge+1)%n,part))];nodes=ends.copy()
    for uid,e in zip(d['uids'],d['entities']):
        if e['kind']!='opening' or not on_face(e,face):continue
        mid=point(sum(q['x'] for q in e['place'])/4,sum(q['y'] for q in e['place'])/4)
        if not -1e-9<=along(mid)<=length+1e-9 or abs((mid['x']-a['x'])*nx+(mid['y']-a['y'])*ny)>t+1e-9:continue
        if mode=='centers':nodes.append((along(mid),mid,center(uid)))
        else:
            for i in ([0,2] if face=='left' else [1,3]):nodes.append((along(e['place'][i]),cp(e['place'][i]),ref(uid,i)))
    nodes.sort(key=lambda v:v[0])
    off=(at['x']-a['x'])*nx+(at['y']-a['y'])*ny;side=-1 if off<0 else 1;gap=float(D(str(gap_mm))*D(str(d['settings']['plotScale']))/D(1000))
    height=float(D('2.5')*D(str(d['settings']['plotScale']))/D(1000));entities=[];spans=[];drawn=[]
    for i,row in enumerate(rows):
        row_at=point(at['x']+nx*side*gap*i,at['y']+ny*side*gap*i);chosen=ends if row=='overall' else nodes
        for first,last in zip(chosen,chosen[1:]):
            if (first[2],last[2]) in drawn:continue
            drawn.append((first[2],last[2]))
            entities.append(dict(kind='dimension',id=0,layerId='0',attrs={},height=height,a=cp(first[1]),b=cp(last[1]),
                offset=(row_at['x']-first[1]['x'])*nx+(row_at['y']-first[1]['y'])*ny,style='linear',angle=math.degrees(math.atan2(uy,ux)),
                refs=dict(a=cp(first[2]),b=cp(last[2]))))
            spans.append(dict(row=row,a=first[2],b=last[2]))
    return entities,spans,(a,b,(nx,ny),length)

def around_geometry(d,distance,face=None,**kw):
    """Every straight edge, on the outer face of a closed wall (by the path's turn) or the given one of an open
    wall, the first row `distance` out from the face."""
    w=d['entities'][0];pts=w['pts'];n=len(pts);closed=w.get('closed',False);bulges=w.get('bulges') or [0.]*n
    if closed:face='right' if ccw(pts) else 'left'
    out=[]
    for k in range(n if closed else n-1):
        if bulges[k]!=0:continue
        t=w['width']/2;poly=face_polygon(pts,closed,t if face=='left' else -t);a,b=poly[k],poly[(k+1)%n];(ux,uy),_=unit(a,b);nx,ny=-uy,ux
        out_n=(-nx,-ny) if face=='right' else (nx,ny);mid=point((a['x']+b['x'])/2+out_n[0]*distance,(a['y']+b['y'])/2+out_n[1]*distance)
        out.append((k,face,mid)+edge_geometry(d,k,face,mid,**kw)[:2])
    return out

def edge_cases():
    out=[]
    room=[point(0,0),point(8,0),point(8,6),point(0,6)]
    openings=((0,2.,1.,DOOR,'door'),(1,3.,1.2,WINDOW,'window'))
    def plan_case(name,d,inp,edge,face,mode='ends',rows=('openings','overall')):
        e,sp,_=edge_geometry(d,edge,face,inp['at'],rows=rows,mode=mode,gap_mm=inp.get('gapMm',7.5))
        out.append(case('Kenar: '+name,d,step('plan',inp,completed(dict(entities=e,spans=sp,removed=[],uids=[],revision='$current')),state(d,metadata=[]))))
    d=walled(room,openings=openings)
    plan_case('dikdörtgen duvarın alt kenarı, dış yüz, kapı aralıkları',d,dict(layerId='0',wall=ROOM,edge=0,face='right',at=point(4,-1)),0,'right')
    plan_case('dikdörtgen duvarın alt kenarı, iç yüz',d,dict(layerId='0',wall=ROOM,edge=0,face='left',at=point(4,1)),0,'left')
    plan_case('sağ kenar, dış yüz, pencere',d,dict(layerId='0',wall=ROOM,edge=1,face='right',at=point(9,3)),1,'right')
    plan_case('üst kenar batıya gider, dış yüz',d,dict(layerId='0',wall=ROOM,edge=2,face='right',at=point(4,7)),2,'right')
    plan_case('sol kenar güneye gider, kapanış kenarı',d,dict(layerId='0',wall=ROOM,edge=3,face='right',at=point(-1,3)),3,'right')
    plan_case('pencere merkezleri, yalnız açıklık sırası',d,dict(layerId='0',wall=ROOM,edge=1,face='right',at=point(9,3),openings='centers',rows=['openings']),1,'right',mode='centers',rows=('openings',))
    plan_case('yalnız toplam: dış boy kalınlıkla 8.25 m',d,dict(layerId='0',wall=ROOM,edge=0,face='right',at=point(4,-1),rows=['overall']),0,'right',rows=('overall',))
    plan_case('kâğıt aralığı 10 mm',d,dict(layerId='0',wall=ROOM,edge=0,face='right',at=point(4,-1),gapMm=10.),0,'right')
    far=[point(500000+p['x'],4400000+p['y']) for p in room]
    d=walled(far,openings=openings)
    plan_case('büyük koordinatlarda dış yüz',d,dict(layerId='0',wall=ROOM,edge=0,face='right',at=point(500004,4399999)),0,'right')
    rot=[point(math.cos(math.radians(30))*p['x']-math.sin(math.radians(30))*p['y'],math.sin(math.radians(30))*p['x']+math.cos(math.radians(30))*p['y']) for p in room]
    d=walled(rot,openings=openings)
    mid=point((rot[0]['x']+rot[1]['x'])/2+math.sin(math.radians(30))*1.,(rot[0]['y']+rot[1]['y'])/2-math.cos(math.radians(30))*1.)
    plan_case('30° dönük oda',d,dict(layerId='0',wall=ROOM,edge=0,face='right',at=mid),0,'right')
    cw=[room[0],room[3],room[2],room[1]]
    d=walled(cw,openings=((3,6.,1.,DOOR,'door'),))
    plan_case('saat yönünde çizilmiş oda: dış yüz solda',d,dict(layerId='0',wall=ROOM,edge=3,face='left',at=point(2,-1)),3,'left')
    ell=[point(0,0),point(6,0),point(6,4)]
    d=walled(ell,closed=False,openings=((1,2.,.9,DOOR,'door'),))
    plan_case('açık L duvarın ikinci kenarı, gönyeli köşeden',d,dict(layerId='0',wall=ROOM,edge=1,face='right',at=point(7,2)),1,'right')
    plan_case('açık L duvarın ilk kenarı, iç köşeye kadar',d,dict(layerId='0',wall=ROOM,edge=0,face='left',at=point(3,1)),0,'left')
    # ADR 0719 §4: a niche on its own face only, a flue on none.
    nf=((0,2.,1.,DOOR,'door'),(0,4.5,.6,NICHE,'niche',dict(operation='fixed',side='right',depth=.1)),(0,6.5,.2,FLUE,'flue',dict(operation='fixed',depth=.1)))
    d=walled(room,openings=nf)
    plan_case('niş kendi yüzünde ölçülür, baca ölçülmez (dış yüz)',d,dict(layerId='0',wall=ROOM,edge=0,face='right',at=point(4,-1)),0,'right')
    plan_case('öbür yüzün nişi ve baca ölçülmez (iç yüz)',d,dict(layerId='0',wall=ROOM,edge=0,face='left',at=point(4,1)),0,'left')
    # Refusals: which edge, an edge not there, an arc edge.
    d=walled(room,openings=openings)
    out.append(case('Ret: çok kenarlı duvarda kenar verilmedi',d,step('plan',dict(layerId='0',wall=ROOM,at=point(4,-1)),error('invalid_dimension_set','Duvarın 4 kenarı var; ölçülecek kenarı (edge) verin ya da bütün kenarları (around) seçin.','edge'),state(d))))
    out.append(case('Ret: duvarda olmayan kenar',d,step('plan',dict(layerId='0',wall=ROOM,edge=4,at=point(4,-1)),error('invalid_dimension_set','Duvarda 4 numaralı kenar yok; kenarlar 0 ile 3 arasında.','edge'),state(d))))
    arc=walled(room,bulges=[0.,0.,.25,0.])
    out.append(case('Ret: yay kenar',arc,step('plan',dict(layerId='0',wall=ROOM,edge=2,at=point(4,7)),error('invalid_dimension_set','2 numaralı kenar yay; ölçü dizisi yalnız düz kenarı ölçer.','edge'),state(arc))))
    d=fresh()
    out.append(case('Ret: iki uçlu duvarda 1 numaralı kenar',d,step('plan',dict(layerId='0',wall=WALL,edge=1,at=point(4,1)),error('invalid_dimension_set','Duvarda 1 numaralı kenar yok; kenarlar 0 ile 0 arasında.','edge'),state(d))))
    e,sp,_=row_geometry(d,dict(layerId='0',wall=WALL,at=point(4,1)))
    out.append(case('İki uçlu duvarda edge 0 eskisiyle aynı plan',d,step('plan',dict(layerId='0',wall=WALL,edge=0,at=point(4,1)),completed(dict(entities=e,spans=sp,removed=[],uids=[],revision='$current')),state(d,metadata=[]))))
    # Around: every straight edge on the outer face, one step.
    def around_case(name,d,inp,distance,undo=True):
        planned=around_geometry(d,distance,face=inp.get('face','left'),gap_mm=inp.get('gapMm',7.5))
        entities=[e for g in planned for e in g[3]];spans=[s for g in planned for s in g[4]]
        out.append(case('Bütün kenarlar plan: '+name,d,step('plan',inp,completed(dict(entities=entities,spans=spans,removed=[],uids=[],revision='$current')),state(d,metadata=[]))))
        base=len(d['entities']);ids=list(range(base+1,base+1+len(entities)))
        fields={str(i):{k:v for k,v in ent.items() if k in ('a','b','height','offset','refs','style','angle')} for i,ent in zip(ids,entities)}
        result=completed(dict(set='$any',sets=['$any']*len(planned),uids=['$any']*len(entities),created=['$any']*len(entities),removed=[],changed=[],revision='$current'))
        steps=[step('execute',inp,result,dict(ids=list(range(1,base+1))+ids,fields=fields,canUndo=True,dirty=True,revision='changed'))]
        if undo:
            steps.append(dict(op='undo',expect=dict(ids=list(range(1,base+1)),dimensionSets=[],canUndo=False,canRedo=True)))
            steps.append(dict(op='redo',expect=dict(ids=list(range(1,base+1))+ids,fields=fields,canUndo=True)))
        out.append(case('Bütün kenarlar: '+name,d,*steps))
    d=walled(room,openings=openings)
    around_case('dikdörtgen oda, kapı ve pencere, dört dış zincir tek adımda',d,dict(layerId='0',wall=ROOM,around=True,edge=0,at=point(4,-1.125)),1.)
    around_case('ilk sıranın uzaklığı başka kenarın yüzünden',d,dict(layerId='0',wall=ROOM,around=True,edge=1,at=point(9.625,3)),1.5,undo=False)
    d=walled(cw,openings=((3,6.,1.,DOOR,'door'),))
    around_case('saat yönünde oda: dış yüz yine dışarıda',d,dict(layerId='0',wall=ROOM,around=True,edge=3,at=point(4,-1.125)),1.,undo=False)
    d=walled(ell,closed=False,openings=((1,2.,.9,DOOR,'door'),))
    around_case('açık L duvar: verilen yüz iki kenarda',d,dict(layerId='0',wall=ROOM,around=True,face='right',edge=0,at=point(3,-1.125)),1.,undo=False)
    return out

def build():
    out={}
    for suffix,cases in [('create',create_cases()+edge_cases()),('edit',edit_cases()),('query',query_cases())]:
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
