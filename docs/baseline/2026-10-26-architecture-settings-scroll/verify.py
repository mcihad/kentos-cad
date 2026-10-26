"""Independent expected models for actual property-panel GUI saves."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
B=Path(__file__).resolve().parent
R=B.parents[2]
spec=importlib.util.spec_from_file_location('reader',R/'tools/kcad/kcad.py')
reader=importlib.util.module_from_spec(spec)
spec.loader.exec_module(reader)
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def read(name):return reader.read((B/name).read_bytes())[1]
before=read('before.kcad')
assert len(before['entities'])==len(set(before['uids']))==14
renamed=copy.deepcopy(before)
room='01a17545-a419-75c4-b0da-5b7c30b39b76'
renamed['entities'][renamed['uids'].index(room)]['name']='Oturma'
for e in renamed['entities']:
    if e['kind']=='tag' and e['of']==room:e['text']=e['text'].replace('\nSalon\n','\nOturma\n')
    if e['kind']=='table':
        assert e['cells'][1][1]=='Salon'
        e['cells'][1][1]='Oturma'
expected={'settings-only.kcad':before,'renamed.kcad':renamed,'undo.kcad':before,
          'preview-cancel.kcad':before,'reopened.kcad':before}
records=json.loads((B/'models.json').read_text())
assert set(expected)=={r['file']for r in records}
for r in records:
    assert sha(B/r['file'])==r['sha256']
    assert read(r['file'])==expected[r['file']],r['file']
events=json.loads((B/'gui-events.json').read_text())
for e in events:
    assert sha(B/e['file'])==e['sha256']
    p=json.loads((B/e['build']).read_text())
    assert sha(Path(p['app'])/p['executable'])==p['sha256']
for file,digest in json.loads((B/'after-binary.json').read_text())['sourceFiles'].items():
    assert sha(R/file)==digest,file
for r in json.loads((B/'test-runs.json').read_text()):assert sha(B/r['log'])==r['sha256']
for name,theme,size in [('dark13-profile.json','dark',13),('light18-profile.json','light',18)]:
    p=json.loads((B/name).read_text())['user']
    assert p['appearance.theme']==theme and p['appearance.textSize']==size
print(f'PASS: {len(records)} exact actual GUI saves, all 14 entities/UUIDs/settings; '
      f'{len(events)} raw image hashes, immutable binaries, final sources and logs.')
