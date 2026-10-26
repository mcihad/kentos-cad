"""Public query timing plus complete identity/order/model invariants; not rendering FPS."""
import argparse,json
from pathlib import Path
from statistics import median
from time import perf_counter_ns
from kentos import cad
B=Path(__file__).resolve().parent;R=B.parents[2]
parser=argparse.ArgumentParser();parser.add_argument('--rooms',type=int,default=10000);parser.add_argument('--lines',type=int,default=20000);parser.add_argument('--missing',type=int,default=3);args=parser.parse_args()
assert args.rooms>=5 and args.lines>=0 and 0<=args.missing<=args.rooms-2
setup=json.loads((R/'fixtures/commands/v1/cad.tags.check.json').read_text())['setup'];setup['entities']=[]
for i in range(args.rooms):
 x=5*i
 room=dict(kind='space',id=i+1,layerId='mahal',attrs={},name=f'Mahal {i+1}',arch=dict(level='zemin'),parts=[dict(outer=dict(pts=[dict(x=x,y=0),dict(x=x+4,y=0),dict(x=x+4,y=3),dict(x=x,y=3)]))])
 if i<2:room['number']='DUP'
 elif i>=2+args.missing:room['number']=f'N-{i:030}'
 setup['entities'].append(room)
for i in range(args.lines):
 setup['entities'].append(dict(kind='line',id=args.rooms+i+1,layerId='cizim',attrs={},a=dict(x=500000+i,y=4400000),b=dict(x=500001+i,y=4400000)))
doc=cad.Document.from_bytes(json.dumps(setup).encode());before=doc.to_bytes();uids=[r.uid for r in doc.entities()][:args.rooms]
times=[]
for op in [cad.tags.check.plan,cad.tags.check]:
 for _ in range(3):
  start=perf_counter_ns();result=op(doc,missing_space_numbers=True);times.append((perf_counter_ns()-start)/1e6)
  rows=[r.to_json()for r in result.findings]
  assert len(rows)==args.rooms+1+args.missing
  assert rows[0]['code']=='tag_duplicate_number'and rows[0]['uids']==uids[:2]
  for i,row in enumerate(rows[1:1+args.missing],3):
   assert row==dict(code='tag_missing_space_number',message=f'“Mahal {i}” mahalının numarası yok (Zemin kat).',uids=[uids[i-1]],at=dict(x=(i-1)*5+2.,y=1.5))
  for i,row in enumerate(rows[1+args.missing:]):
   assert row['code']=='tag_untagged'and row['uids']==[uids[i]]
   assert row['at']==dict(x=5*i+2.,y=1.5)
  assert doc.to_bytes()==before
old=[f.to_json()for f in cad.tags.check(doc).findings]
assert old==[rows[0],*rows[1+args.missing:]];assert doc.to_bytes()==before
record=dict(profile='debug public Python SDK',rooms=args.rooms,unrelatedLines=args.lines,missing=args.missing,planMs=times[:3],queryMs=times[3:],medianMs=median(times),maxMs=max(times),findings=len(rows),note='Every finding UUID/order/point, missing messages and full model checked on all six runs; old default findings retained. Not release/p95/memory/GPU/frame acceptance.')
(B/f'timing-{args.rooms}-{args.lines}-{args.missing}.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps(record,indent=2))
