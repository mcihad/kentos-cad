"""Independent rectangle/Decimal expectations for complete native GUI saves."""
import copy,hashlib,importlib.util,json,math
from pathlib import Path
B=Path(__file__).resolve().parent;R=B.parents[2]
spec=importlib.util.spec_from_file_location('reader',R/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(spec);spec.loader.exec_module(reader)
def read(name):return reader.read((B/name).read_bytes())[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def same(a,b):
    if isinstance(a,(int,float)) and isinstance(b,(int,float)) and not isinstance(a,bool) and not isinstance(b,bool):
        return math.isclose(a,b,rel_tol=0,abs_tol=1e-9)
    if isinstance(a,dict) and isinstance(b,dict):return a.keys()==b.keys() and all(same(a[k],b[k]) for k in a)
    if isinstance(a,list) and isinstance(b,list):return len(a)==len(b) and all(same(x,y) for x,y in zip(a,b))
    return a==b
before=read('before.kcad');assert len(before['entities'])==4
source=before['uids'][3]
created=copy.deepcopy(before);actual=read('created.kcad');assert actual['uids'][:4]==before['uids'];assert len(actual['uids'])==5
created['uids'].append(actual['uids'][-1])
created['entities'].append(dict(kind='tag',id=5,layerId='etiket',attrs={},of=source,at=dict(x=9.,y=2.),anchor=dict(x=3.,y=2.),
    content='{number} {name}\nAlan {area}\nNet {netArea}',shape='rectangle',size=3.5,leader=True,
    text='N-01 Çalışma\nAlan 26.04 m²\nNet 21.79 m²'))
column=copy.deepcopy(created);column['entities'][1]['width']=1.;column['entities'][-1]['text']='N-01 Çalışma\nAlan 26.04 m²\nNet 21.04 m²'
small=copy.deepcopy(column);small['entities'][1]['width']=.75;small['entities'][-1]['text']='N-01 Çalışma\nAlan 26.04 m²\nNet 21.48 m²'
wall=copy.deepcopy(column);wall['entities'][0]['width']=.4
wall['entities'][3]['parts']=[dict(outer=dict(pts=[dict(x=-.2,y=-.2),dict(x=6.2,y=-.2),dict(x=6.2,y=4.2),dict(x=-.2,y=4.2)]))]
wall['entities'][-1]['text']='N-01 Çalışma\nAlan 28.16 m²\nNet 19.16 m²'
lost=copy.deepcopy(column);lost['entities'].pop(0);lost['uids'].pop(0);lost['entities'][2]['broken']='lostEnclosure';lost['entities'][-1]['text']='N-01 Çalışma\nAlan 26.04 m²\nNet'
# KCAD stores stable UUIDs; the independent reader reconstructs display IDs in
# document order (tools/kcad/kcad.py:5767). Deleting row one shifts those IDs.
for index,entity in enumerate(lost['entities'],1):entity['id']=index
expected={'before-rejected.kcad':before,'settings-only.kcad':before,'created.kcad':created,'create-undo.kcad':before,'create-redo.kcad':created,'column.kcad':column,
          'undo.kcad':created,'redo.kcad':column,'invalid.kcad':column,'wall.kcad':wall,'lost.kcad':lost,
          'recovered.kcad':column,'compact.kcad':small,'cancel.kcad':column,'reopened.kcad':column}
records=json.loads((B/'models.json').read_text());assert set(expected)=={r['file']for r in records}
for r in records:
    assert sha(B/r['file'])==r['sha256'];assert same(read(r['file']),expected[r['file']]),r['file']
# Strict old geometry/settings/identities are required when the only change is a tag or column.
for name in ['settings-only.kcad','created.kcad','undo.kcad']:
    got=read(name);assert got['entities'][:4]==before['entities'];assert got['uids'][:4]==before['uids']
for name in ['column.kcad','redo.kcad','invalid.kcad','recovered.kcad','cancel.kcad','reopened.kcad']:
    got=read(name);assert got['entities'][0]==before['entities'][0];assert got['entities'][2:4]==before['entities'][2:4]
    assert got['uids']==created['uids']
events=json.loads((B/'gui-events.json').read_text())
for e in events:
    assert sha(B/e['file'])==e['sha256'];build=json.loads((B/e['build']).read_text());assert sha(Path(build['app'])/build['executable'])==build['sha256']
for file,digest in json.loads((B/'after-binary.json').read_text())['sourceFiles'].items():assert sha(R/file)==digest,file
for r in json.loads((B/'test-runs.json').read_text()):assert sha(B/r['log'])==r['sha256']
for name,theme,size in [('dark13-profile.json','dark',13),('light18-profile.json','light',18)]:
    user=json.loads((B/name).read_text())['user'];assert user['appearance.theme']==theme;assert user['appearance.textSize']==size
print(f'PASS: {len(records)} full native GUI saves, unchanged old geometry/settings/UUIDs, independent inner-face/column areas, {len(events)} raw screenshots and binary/source/log hashes.')
