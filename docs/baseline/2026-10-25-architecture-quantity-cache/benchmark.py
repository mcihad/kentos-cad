"""Same frozen public workload, full results; debug timings are observations, not UI budgets."""
import hashlib,json,platform,statistics,sys,time
from pathlib import Path
from kentos import cad
import kentos._native as native
root=Path.cwd();base=root/'docs/baseline/2026-10-25-architecture-quantity-cache';mode=sys.argv[1]
if mode=='before' and (base/'benchmark-before.json').exists():raise SystemExit('Frozen old measurement must not be overwritten; use after.')
if mode=='before':
 source=(root/'docs/baseline/2026-10-25-architecture-space-separators/measure.py').read_text()
 # Run the independent workload unchanged except its output location/profile label.
 source=source.replace("ROOT=Path(__file__).resolve().parents[2]","ROOT=Path.cwd()")
 source=source.replace("out=ROOT/'.run/architecture-space-separators'", "out=ROOT/'docs/baseline/2026-10-25-architecture-quantity-cache'")
 source=source.replace("q=timed('quantities',lambda:cad.spaces.quantities(doc))", "doc.save(out/'workload.kcad')\nq=timed('quantities',lambda:cad.spaces.quantities(doc))")
 source=source.replace("N=int(sys.argv[1]) if len(sys.argv)>1 else 200","N=200").replace("NOISE=int(sys.argv[2]) if len(sys.argv)>2 else 20000","NOISE=20000")
 source=source.replace("debug native Python extension; process competed with Cargo build", "debug native Python extension; no Cargo build running")
 exec(compile(source,str(root/'docs/baseline/2026-10-25-architecture-space-separators/measure.py'),'exec'),dict(__file__=str(root/'docs/baseline/2026-10-25-architecture-space-separators/measure.py'),__name__='__main__'))
doc=cad.Document.open(base/'workload.kcad');original=doc.to_bytes();times=[];answer=None
for i in range(5):
 start=time.perf_counter();q=cad.spaces.quantities(doc).to_json();times.append((time.perf_counter()-start)*1000)
 if answer is None:answer=q
 else:assert q==answer
 assert doc.to_bytes()==original
 print(mode,i,round(times[-1],3),flush=True)
 if i==0:
  if mode=='before':(base/'workload-quantities-before.json').write_text(json.dumps(q,ensure_ascii=False,indent=2)+'\n')
  else:assert q==json.loads((base/'workload-quantities-before.json').read_text())
areas=sorted(r['area']for r in answer['spaces']);expected=sorted([34.22,38.22]*200)
assert all(abs(a-b)<1e-6 for a,b in zip(areas,expected))and len(areas)==400
record=dict(mode=mode,platform=platform.platform(),profile='debug native Python extension; no Cargo build',extension=str(native.__file__),extensionSha256=hashlib.sha256(Path(native.__file__).read_bytes()).hexdigest(),workloadSha256=hashlib.sha256((base/'workload.kcad').read_bytes()).hexdigest(),timings_ms=times,median_ms=statistics.median(times),max_ms=max(times),runs=5,rooms=400,noise=20000,outputExact=True,documentUnchanged=True)
(base/('benchmark-'+mode+'.json')).write_text(json.dumps(record,indent=2)+'\n')
print('PASS',mode,record['median_ms'],flush=True)
