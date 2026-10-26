#!/usr/bin/env python3
"""Independent room-number contracts: Python integer suffixes and rectangle centres.

No KentOS library/recorded answers. Unit/translation cases use exact Decimal
coordinates, arbitrary-size Python integers generate expected number strings.
"""
import argparse
import copy
import json
from decimal import Decimal as D
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'fixtures/commands/v1/cad.spaces.number.json'

def setup(unit='m',far=False):
    s=copy.deepcopy(json.loads((ROOT/'fixtures/commands/v1/cad.walls.rectangle.json').read_text())['setup'])
    s['name']='Mahal numaralandırma bağımsız kabul';s['settings']['drawingUnit']=unit
    s['settings']['arch']['levels'].append(dict(id='bir',name='Birinci kat',elevation=3.,height=3.))
    x,y=(D(500000),D(4400000)) if far else (D(0),D(0))
    s['entities']=[]
    for i,(a,b) in enumerate([(0,0),(6,0),(0,5),(6,5)],1):
        p=lambda c,d:dict(x=float(x+D(a)+D(c)),y=float(y+D(b)+D(d)))
        s['entities'].append(dict(kind='space',id=i,layerId='duvar',attrs={},number=f'OLD-{i}',name=f'Oda {i}',usage='Konut',
            arch=dict(level='zemin' if i<4 else 'bir'),height=3.,parts=[dict(outer=dict(pts=[p(0,0),p(4,0),p(4,3),p(0,3)]))]))
    return s

def ok(output):return dict(status='completed',output=output,warnings=[])
def uid(i):return f'$uidOf:{i}'
def numbered(s,ids,first):
    n=len(first)-len(first.rstrip('0123456789'));head,tail=first[:-n],first[-n:]
    out=[]
    for k,i in enumerate(ids):
        e=copy.deepcopy(s['entities'][i-1]);e['number']=head+str(int(tail)+k).zfill(n);out.append(e)
    return out

def build():
    base=setup();cases=[]
    for unit in ['m','mm','cm','in','ft']:
        for far in [False,True]:
            s=setup(unit,far);x,y=(500000.,4400000.)if far else(0.,0.)
            for direction,ids in [(None,[3,1,4,2]),((1,0),[1,3,2,4]),((-1,0),[4,2,3,1]),((0,1),[2,1,4,3])]:
                a=dict(uids=[uid(i)for i in [3,1,4,2]],first='Z-009')
                if direction:a['order']=dict(kind='direction',**{'from':dict(x=x,y=y)},to=dict(x=x+direction[0],y=y+direction[1]))
                e=numbered(s,ids,a['first']);output=dict(spaces=e,kept=[uid(i)for i in ids],revision='$current')
                state=dict(ids=[1,2,3,4],entities={str(v['id']):v for v in e},canUndo=True)
                original=dict(ids=[1,2,3,4],entities={str(v['id']):v for v in s['entities']},canUndo=False)
                cases.append(dict(name=f'{unit}/{far}/{direction}: explicit order identity and one undo',setup=s,steps=[
                    dict(op='validate',input=a,result=ok(None),expect=dict(original,revision='same')),
                    dict(op='plan',input=a,result=ok(output),expect=dict(original,revision='same')),
                    dict(op='execute',input=a,result=ok(output),expect=dict(state,revision='changed')),
                    dict(op='execute',input=a,result=ok(output),expect=dict(state,revision='same')),
                    dict(op='undo',returns='Mahalleri numarala',expect=dict(original,canRedo=True,revision='changed')),
                    dict(op='redo',returns='Mahalleri numarala',expect=dict(state,canRedo=False,revision='changed'))]))
    for first in ['099','A-9999999999999999999999999999999999999999','Kat İ-000','0']:
        ids=[1,2,3];e=numbered(base,ids,first)
        cases.append(dict(name='Suffix arbitrary precision: '+first,steps=[dict(op='execute',input=dict(uids=[uid(i)for i in ids],first=first),result=ok(dict(spaces=e,kept=[uid(i)for i in ids],revision='$current')),expect=dict(entities={str(v['id']):v for v in e},revision='changed'))]))
    def fail(name,args,code,path,s=None,extra=None):
        s=copy.deepcopy(s or base);status='conflict'if code=='revision_conflict'else'failed';error=dict(code=code,path=path,message='$any')
        if status=='conflict':error['revision']='$current'
        step=dict(op='execute',input=args,result=dict(status=status,error=error),expect=dict(entities={str(v['id']):v for v in s['entities']},canUndo=False,dirty=False,revision='same'))
        if extra:step.update(extra)
        cases.append(dict(name=name,setup=s,steps=[step]))
    a=dict(uids=[uid(1),uid(2)],first='Z-001')
    fail('Empty selection',dict(a,uids=[]),'no_entities','uids')
    fail('Invalid UUID',dict(a,uids=['bad']),'invalid_uid','uids[0]')
    fail('Missing UUID',dict(a,uids=['00000000-0000-0000-0000-000000000001']),'entity_not_found','uids[0]')
    fail('Repeated room',dict(a,uids=[uid(1),uid(1)]),'invalid_space_number','uids[1]')
    fail('Too many UUIDs',dict(a,uids=[uid(1)]*10001),'invalid_space_number','uids')
    for first in ['', 'A', ' Z-001', 'Z-001 ', 'Z-001\n', '\tZ-001', '٠١', 'A'*256+'1']:
        fail('Invalid first '+repr(first),dict(a,first=first),'invalid_space_number','first')
    # Carry would increase 256 digits to 257, refusing all selected updates.
    fail('Sequence length carry',dict(a,first='9'*256),'invalid_space_number','first')
    fail('Old revision',dict(a,expectedRevision='9999'),'revision_conflict','expectedRevision')
    fail('Invalid revision',dict(a,expectedRevision='01'),'invalid_revision','expectedRevision')
    s=copy.deepcopy(base);s['entities'][1]['layerId']='kilitli';fail('Locked second is atomic',a,'layer_locked','uids[1]',s)
    s=copy.deepcopy(base);s['entities'][1]=dict(kind='line',id=2,layerId='duvar',attrs={},a=dict(x=20.,y=0.),b=dict(x=22.,y=0.));fail('Wrong kind second is atomic',a,'not_a_space','uids[1]',s)
    for hide,other in [(False,False),(True,False),(True,True)]:
        s=copy.deepcopy(base);s['entities'][3]['number']='Z-002'
        if hide:s['entities'][3]['layerId']='gizli'
        if not other:s['entities'][3]['arch']['level']='zemin'
        fail(f'Collision outside selection hidden={hide} other={other}',a,'space_number_conflict','first',s)
    for kind,to,extra in [('zero',dict(x=0.,y=0.),None),('nonfinite',dict(x=1.,y=0.),dict(nonFinite={'order.to.x':'Infinity'}))]:
        fail('Invalid direction '+kind,dict(a,order=dict(kind='direction',**{'from':dict(x=0.,y=0.)},to=to)), 'not_finite'if extra else'invalid_space_number','order.to.x'if extra else'order',extra=extra)
    s=copy.deepcopy(base);s['entities'][0]['number']='Z-002';s['entities'][1]['number']='Z-002';e=numbered(s,[1,2],'Z-001')
    cases.append(dict(name='Selected duplicates may be repaired together',setup=s,steps=[dict(op='execute',input=a,result=ok(dict(spaces=e,kept=[uid(1),uid(2)],revision='$current')),expect=dict(entities={str(v['id']):v for v in e},revision='changed'))]))
    return dict(format='kentos.command-cases',version=1,command='cad.spaces.number',commandVersion=1,title='Independent room numbering',setup=base,cases=cases)
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');args=p.parse_args();d=build();text=json.dumps(d,ensure_ascii=False,indent=2)+'\n'
    if args.check:
        if not OUT.exists()or OUT.read_text()!=text:raise SystemExit('Stale room numbering cases')
    else:OUT.write_text(text)
    print(f'{len(d["cases"])} independent room numbering cases')
