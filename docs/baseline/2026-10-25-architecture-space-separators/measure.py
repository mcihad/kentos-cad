"""Public Python -> native end-to-end room workload. Independent rectangular expectations.
Run with .run/py/bin/python; records observations, not an interactive-render SLA.
"""
import json,time,platform,sys
from pathlib import Path
from kentos import cad
ROOT=next(p for p in Path(__file__).resolve().parents if (p/'Cargo.toml').is_file() and (p/'fixtures/commands/v1').is_dir())
out=ROOT/'.run/architecture-space-separators'
setup=json.loads((ROOT/'fixtures/commands/v1/cad.spaces.separators.json').read_text())['setup']
setup['settings']['plotScale']=100;setup['settings']['drawingUnit']='m';setup['entities']=[]
N=int(sys.argv[1]) if len(sys.argv)>1 else 200
NOISE=int(sys.argv[2]) if len(sys.argv)>2 else 20000
entities=setup['entities'];paths=[]
for i in range(N):
 x=500000+(i%20)*15;y=4400000+(i//20)*12
 entities.append(dict(kind='wall',id=len(entities)+1,layerId='cizim',attrs={},closed=True,width=.2,pts=[dict(x=x,y=y),dict(x=x+10,y=y),dict(x=x+10,y=y+8),dict(x=x,y=y+8)]))
 entities.append(dict(kind='column',id=len(entities)+1,layerId='cizim',attrs={},at=dict(x=x+3,y=y+4),width=2,depth=2))
 paths.append(dict(pts=[dict(x=x+5,y=y),dict(x=x+5,y=y+8)]))
for i in range(NOISE):
 x=510000+(i%1000)*3;y=4500000+(i//1000)*3
 entities.append(dict(kind='line',id=len(entities)+1,layerId='cizim',attrs=dict(No=str(i)),a=dict(x=x,y=y),b=dict(x=x+2,y=y+1)))
times={}
def timed(name,fn):
 start=time.perf_counter();result=fn();times[name]=round((time.perf_counter()-start)*1000,3);print(name,times[name],flush=True);return result
doc=timed('open_json',lambda:cad.Document.from_bytes(json.dumps(setup).encode()))
roles=timed('create_separators',lambda:cad.spaces.separators(doc,operation=cad.SpaceSeparatorOperation.from_json(dict(kind='create',layerId='cizim',paths=paths))))
before=doc.to_bytes()
plan=timed('discover_plan',lambda:cad.spaces.discover.plan(doc,layer_id='mahal',columns='all'))
assert len(plan.proposals)==N*2 and doc.to_bytes()==before
for p in plan.proposals:assert p.conflicts==[]
made=timed('discover_execute',lambda:cad.spaces.discover(doc,layer_id='mahal',columns='all',expected_revision=plan.revision))
assert len(made.uids)==N*2
q=timed('quantities',lambda:cad.spaces.quantities(doc))
assert len(q.spaces)==N*2
expected=sorted([34.22,38.22]*N)
assert all(abs(a.area-b)<1e-6 for a,b in zip(sorted(q.spaces,key=lambda s:s.area),expected))
assert abs(sum(s.area for s in q.spaces)-72.44*N)<1e-5
timed('undo_discovery',doc.undo);assert doc.to_bytes()==before
timed('redo_discovery',doc.redo)
tags=timed('tags_create',lambda:cad.tags.create(doc,layer_id='mahal',tags=[cad.NewTag.from_json(dict(of=uid,content='{area}'))for uid in made.uids]))
table=timed('schedule_create',lambda:cad.schedules.create(doc,layer_id='mahal',at=cad.Vec2(x=501000,y=4400000),of='spaces',columns=[cad.ScheduleColumn.from_json(dict(field='area'))]))
with_tags=doc.to_bytes()
timed('move_one_separator_followers',lambda:cad.entities.transform.execute_input(doc,cad.EntitiesTransform.from_json(dict(uids=[roles.uids[0]],transform=dict(kind='move',dx=1,dy=0)))))
q=cad.spaces.quantities(doc)
expected=sorted([34.22,38.22]*(N-1)+[42.02,30.42])
assert all(abs(a.area-b)<1e-6 for a,b in zip(sorted(q.spaces,key=lambda s:s.area),expected))
texts=[doc.entity(uid).entity.to_json()['text']for uid in tags.uids]
assert sum('42.02'in t for t in texts)==1 and sum('30.42'in t for t in texts)==1
cells=json.dumps(doc.entity(table.uid).entity.to_json()['cells']);assert '42.02'in cells and '30.42'in cells
timed('undo_move_with_followers',doc.undo);assert doc.to_bytes()==with_tags
record=dict(platform=platform.platform(),python=sys.version,profile='debug native Python extension; process competed with Cargo build',structures=N,rooms=N*2,wall_sources=N,column_sources=N,separators=N,irrelevant_lines=NOISE,tags=len(tags.uids),schedule_rows=table.rows,timings_ms=times,oracle='independent 9.8×7.8, zero-width partition, 4 m² column; total 72.44 per structure; move gives 42.02/30.42; exact KCAD undo and tag/table text',status='pass',runAt=time.time())
(out/f'measure-{N}-{NOISE}.json').write_text(json.dumps(record,indent=2)+'\n')
