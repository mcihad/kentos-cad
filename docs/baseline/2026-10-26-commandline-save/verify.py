"""Independent KCAD model/UUID/tag/schedule checks of real saved GUI actions.

Only the standalone KCAD decoder is imported; no KentOS command/geometry code.
"""
from pathlib import Path
from decimal import Decimal as D
import copy,hashlib,importlib.util,json,math
B=Path(__file__).resolve().parent;R=B.parents[2]
spec=importlib.util.spec_from_file_location('reader',R/'tools/kcad/kcad.py');reader=importlib.util.module_from_spec(spec);spec.loader.exec_module(reader)
def read(name):return reader.read((B/name).read_bytes())[1]
original=read('before.kcad');ids=json.loads((B/'ids.json').read_text());members=dict(zip(original['uids'],original['entities']))
assert len(original['entities'])==13
for uid in ids['rooms']:
 e=members[uid];pts=e['parts'][0]['outer']['pts'];area=abs(sum(D(str(a['x']))*D(str(b['y']))-D(str(b['x']))*D(str(a['y']))for a,b in zip(pts,pts[1:]+pts[:1])))/2
 assert abs(area-D('22.04'))<D('1e-10')

def numbered(name,values):
 actual=read(name);expected=copy.deepcopy(original);mapping=dict(zip(ids['rooms'],values))
 for e,uid in zip(expected['entities'],expected['uids']):
  if e['kind']=='space':e['number']=mapping[uid]
  elif e['kind']=='tag':e['text']=mapping[e['of']]+'\n'+e['text'].split('\n',1)[1]
  elif e['kind']=='table':
   for row,refs in zip(e['cells'][1:],e['source']['members']):row[0]=mapping[refs[0]]
   # Only the first column may resize for changed number glyph widths.
   table=next(v for v in actual['entities']if v['id']==e['id']);assert len(table['columns'])==len(e['columns']);assert table['columns'][1:]==e['columns'][1:]
   assert math.isfinite(table['columns'][0])and table['columns'][0]>0;e['columns'][0]=table['columns'][0]
 assert actual==expected,name
manifest=B/'models.json';assert manifest.exists(),'No real GUI model saves recorded yet'
records=json.loads(manifest.read_text())
for record in records:
 if record['numbers'] is None:assert read(record['file'])==original,record['file']
 else:numbered(record['file'],record['numbers'])
 assert hashlib.sha256((B/record['file']).read_bytes()).hexdigest()==record['sha256']
for e in json.loads((B/'gui-events.json').read_text()):
 assert hashlib.sha256((B/e['file']).read_bytes()).hexdigest()==e['sha256']
 build=json.loads((B/e['build']).read_text());assert hashlib.sha256((Path(build['app'])/build['executable']).read_bytes()).hexdigest()==build['sha256']
accepted='after2-binary.json'
binary=json.loads((B/accepted).read_text())
for f,h in binary['sourceFiles'].items():assert hashlib.sha256((R/f).read_bytes()).hexdigest()==h,f
assert hashlib.sha256((Path(binary['app'])/binary['executable']).read_bytes()).hexdigest()==binary['sha256']
print('PASS:',len(records),'GUI disk snapshots (including documented diagnostic existing-disk captures), exact UUID/geometry/settings/tag anchor/template/schedule preservation; room areas22.04m²; raw screenshot/binary hashes')
