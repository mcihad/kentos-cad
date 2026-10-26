"""Opt-in net tags: debug public SDK timing and full model invariants, not renderer FPS."""
import argparse
import json
from pathlib import Path
from statistics import median
from time import perf_counter_ns
from kentos import cad
B=Path(__file__).resolve().parent;R=B.parents[2]
setup=json.loads((R/'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
parser=argparse.ArgumentParser();parser.add_argument('--columns',choices=['net','ignore'],default='net');parser.add_argument('--unrelated',type=int,default=0);args_cli=parser.parse_args()
count=200;setup['entities']=[]
for i in range(count):
    x=10*i
    setup['entities'] += [dict(kind='wall',id=2*i+1,layerId='duvar',attrs={},width=.2,closed=True,
        pts=[dict(x=x,y=0),dict(x=x+6,y=0),dict(x=x+6,y=4),dict(x=x,y=4)],arch=dict(level='zemin')),
        dict(kind='column',id=2*i+2,layerId='kolon',attrs={},width=.5,at=dict(x=x+4,y=2),arch=dict(level='zemin'))]
for i in range(args_cli.unrelated):
    setup['entities'].append(dict(kind='line',id=2*count+i+1,layerId='cizim',attrs={},a=dict(x=1000000+i,y=1000000),b=dict(x=1000001+i,y=1000000)))
doc=cad.Document.from_bytes(json.dumps(setup).encode())
created=cad.spaces.create(doc,layer_id='mahal',spaces=[cad.NewSpace.from_json(dict(seed=dict(x=10*i+2,y=2),method='gross',columns=args_cli.columns,arch=dict(level='zemin'))) for i in range(count)])
args=dict(layer_id='etiket',tags=[cad.NewTag.from_json(dict(of=u,at=dict(x=10*i+8,y=2),content='{area}/{netArea}',leader=True)) for i,u in enumerate(created.uids)])
def timed(action):
    start=perf_counter_ns();result=action();return result,(perf_counter_ns()-start)/1e6
before=doc.to_bytes();_,plan_ms=timed(lambda:cad.tags.create.plan(doc,**args));assert doc.to_bytes()==before
made,create_ms=timed(lambda:cad.tags.create(doc,**args))
original={r.uid:r.entity.to_json() for r in doc.entities()}
initial_net='21.79' if args_cli.columns=='net' else '22.04'
assert all(original[u]['text']==f'26.04 m²/{initial_net} m²' for u in made.uids)
# A physical column changes only the first gross room's net result.
column=next(r.uid for r in doc.entities() if r.entity.id==2)
times=[]
for i in range(10):
    dx=4 if i%2==0 else -4
    _,elapsed=timed(lambda:cad.entities.transform(doc,uids=[column],transform=cad.MoveTransform(dx=dx,dy=0)))
    times.append(elapsed)
    for row in doc.entities():
        e=row.entity.to_json();expected=dict(original[row.uid])
        if row.uid==column and i%2==0:expected['at']=dict(x=8.,y=2.)
        if row.uid==made.uids[0] and i%2==0 and args_cli.columns=='net':expected['text']='26.04 m²/22.04 m²'
        assert e==expected,(i,row.uid)
result=dict(profile='debug public Python SDK',rooms=count,tags=count,sources=2*count,entities=len(original),
    columns=args_cli.columns,unrelated=args_cli.unrelated,planMs=plan_ms,createMs=create_ms,columnMoveMs=times,medianMs=median(times),maxMs=max(times),
    note='Ten source moves, all objects/UUIDs/places compared after each. Native end-to-end command only; not release/p95/GPU/frame/memory acceptance.')
filename='timing.json' if args_cli.columns=='net' and args_cli.unrelated==0 else f'timing-{args_cli.columns}-{args_cli.unrelated}.json'
(B/filename).write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
