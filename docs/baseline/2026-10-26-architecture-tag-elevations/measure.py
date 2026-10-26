"""Debug SDK probe of the new settings refresh; not a renderer benchmark."""
import json
from decimal import Decimal
from pathlib import Path
from time import perf_counter_ns
from statistics import median

from kentos import cad

B = Path(__file__).resolve().parent
R = B.parents[2]
setup = json.loads((R/'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
setup['entities'] = [dict(kind='space', id=i+1, layerId='mahal', attrs={},
    number=f'R-{i+1:04}', arch=dict(level='zemin'), height=2.75,
    parts=[dict(outer=dict(pts=[dict(x=5*i+x, y=y) for x,y in [(0,0),(4,0),(4,3),(0,3)]]))])
    for i in range(1000)]
doc = cad.Document.from_bytes(json.dumps(setup).encode())
cad.tags.create(doc,layer_id='etiket',all=cad.TagAll.from_json(dict(
    kinds=['space'],content='R {number}\nZ {floorElevation}\nT {ceilingElevation}')))
before = {row.uid:row.entity.to_json() for row in doc.entities()}
assert len(before) == 2000
times = []
for i in range(11):
    elevation = Decimal(i)/8
    arch = doc.settings.to_json()['arch']
    arch['levels'][0]['elevation'] = float(elevation)
    start = perf_counter_ns()
    cad.settings.set(doc,arch=cad.ArchSettings.from_json(arch))
    times.append((perf_counter_ns()-start)/1e6)
    for row in doc.entities():
        e=row.entity.to_json()
        if e['kind'] == 'space':
            assert e == before[row.uid]
        else:
            expected=dict(before[row.uid])
            source=before[e['of']]
            expected['text']=f'R {source["number"]}\nZ {elevation:.3f}\nT {elevation+Decimal("2.75"):.3f}'
            assert e == expected
result=dict(profile='debug CPython public SDK', rooms=1000, tags=1000,
    coldNoChangeMs=times[0],changedSettingsMs=times[1:],
    medianMs=median(times[1:]),maxMs=max(times[1:]),
    note='Ten changed settings, no persistent benchmark target. All 2,000 entities and UUIDs compared after each call; no renderer/GPU/release/memory claim.')
(B/'settings-timing.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
