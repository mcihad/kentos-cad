#!/usr/bin/env python3
"""ADR 0678: explicit persistent UUID subsets, independently measured rectangles.

No KentOS, SDK or geometry result is imported. Four fixed source UUIDs name
an outer 10 by 8 metre wall, a partition, and two columns. Decimal arithmetic
measures the chosen whole/left room and its sole chosen 2 by 2 metre hole.
The other independent source-scope generator supplies setup metadata only.
"""
import argparse
from copy import deepcopy
from decimal import Decimal as D
import json
from pathlib import Path
from space_scope_cases import setup, LABEL, DELTA
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'fixtures/commands/v1/cad.spaces.sources.json'
UIDS=[f'00000000-0000-4000-8000-{n:012d}'for n in range(1,5)]
def completed(output,warnings=None):return dict(status='completed',output=output,warnings=warnings or [])
def case(unit,method,partition,far):
    d,seed=setup(unit,far);d['version']=2;d['uids']=UIDS
    # v1 may omit the default, but the independent v2 file writer requires
    # the serialized wall justification explicitly.
    for entity in d['entities']:
        if entity['kind']=='wall':entity['justify']='center'
    d['name']='Kalıcı kaynak alt kümesi bağımsız kabulü'
    members=[UIDS[i]for i in ([0,1,2]if partition else [0,2])]
    width=D(5 if partition else 10);delta=DELTA[method]
    by={m:float((width+v)*(D(8)+v)-4)for m,v in DELTA.items()}
    spec=dict(seed=seed,method=method,columns='all',sources=list(reversed(members)))
    inp=dict(layerId='mahal',spaces=[spec])
    e=dict(kind='space',id=5,layerId='mahal',attrs={},parts='$any',seed=seed,columns='all',sources=members)
    if method!='net':e['method']=method
    row=dict(uid='$uidOf:5',method=method,columns='all',sources=members,area=by[method],
        perimeter=float(2*(width+D(8)+2*delta)+8),byMethod=by,
        rule=f'alan: halkaların alanı, delikler düşülür ({LABEL[method]}); hacim: alan × yükseklik; kolon alanı: Her yöntemde')
    state=dict(ids=[1,2,3,4,5],entities={'5':e},canUndo=True,dirty=True,revision='changed')
    return dict(name=f'{unit}/{method}/{"left"if partition else "whole"}/{"far"if far else "local"}',setup=d,steps=[
        dict(op='plan',input=inp,result=completed(dict(spaces=[dict(e,id=0)],revision='$current')),expect=dict(revision='same',ids=[1,2,3,4])),
        dict(op='execute',input=inp,result=completed(dict(uids=['$any'],ids=[5],revision='$current')),expect=state),
        dict(op='execute',command='cad.spaces.quantities',input={},within=1e-6,result=completed(dict(spaces=[row],walls='$any',openings=[],totals='$any')),expect=dict(revision='same')),
        dict(op='undo',returns='Mahal',expect=dict(ids=[1,2,3,4],canUndo=False,canRedo=True,revision='changed')),
        dict(op='redo',returns='Mahal',expect=state),
    ])
def copy_case(command):
    c=case('m','net',False,False);c['name']='Partial source copy warning: '+command
    if command=='cad.entities.transform':
        args=dict(uids=['$uidOf:5'],copy=True,transform=dict(kind='move',dx=20.,dy=0.))
    elif command=='cad.entities.array':
        args=dict(uids=['$uidOf:5'],layout=dict(kind='grid',rows=1,cols=2,dx=20.,dy=0.))
    else:
        p=lambda x,y:dict(x=x,y=y)
        ring=lambda a,b,c,d:dict(pts=[p(a,b),p(c,b),p(c,d),p(a,d)])
        room=dict(kind='space',id=1,layerId='mahal',attrs={},seed=p(2.,6.),sources=[UIDS[0],UIDS[2]],columns='all',
            parts=[dict(outer=ring(.1,.1,9.9,7.9),holes=[ring(2.,3.,4.,5.)])])
        args=dict(clip=dict(objects=[room],uids=['00000000-0000-4000-8000-000000000005'],
            layers=deepcopy(c['setup']['layers']),base=p(0.,0.)),at=p(20.,0.))
    warn=dict(code='room_sources_not_copied',message='1 mahal: bütün kaynakları birlikte kopyalanmadığı için kopya bağımsız çizilmiş sınır oldu.')
    c['steps'].append(dict(op='execute',command=command,input=args,result=completed('$any',[warn]),
        expect=dict(ids=[1,2,3,4,5,6],entities={'6':dict(kind='space',id=6,layerId='mahal',attrs={},parts='$any')},canUndo=True)))
    c['steps'].append(dict(op='undo',expect=dict(ids=[1,2,3,4,5],entities={'5':deepcopy(c['steps'][1]['expect']['entities']['5'])})))
    return c
def document():
    cases=[case(u,m,p,f)for u in ['mm','cm','m','in','ft']for m in DELTA for p in [False,True]for f in [False,True]]
    cases.extend(copy_case(c)for c in ['cad.entities.transform','cad.entities.array','cad.entities.paste'])
    return dict(format='kentos.command-cases',version=1,command='cad.spaces.create',commandVersion=1,
        title='Kalıcı kaynak UUID alt kümesi: üç yöntem, beş birim, iki başlangıç, bütün/sol oda, kısmi kopya uyarısı',
        note=__doc__.strip(),setup=deepcopy(cases[0]['setup']),cases=cases)
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');parser.add_argument('--output',type=Path,default=OUT);args=parser.parse_args()
    data=json.dumps(document(),ensure_ascii=False,indent=2)+'\n'
    if args.check:assert args.output.read_text()==data
    else:args.output.write_text(data)
    print(len(document()['cases']),'independent persistent-source cases')
