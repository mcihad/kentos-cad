"""Complete source delivery: the independent inventory and KCAD reader, typed API, portable links and tamper refusal."""
import copy,hashlib,json,os,shutil,sys,tempfile,unittest
from pathlib import Path
import kentos.cad as cad
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tools/kcad'))
import kcad
FX=json.loads((ROOT/'fixtures/sheet/v1/delivery.json').read_text())
def resolve(v,work):
 if isinstance(v,str):return v.replace('$work',str(work))
 if isinstance(v,list):return [resolve(x,work) for x in v]
 if isinstance(v,dict):return {k:resolve(x,work) for k,x in v.items()}
 return v

def source(c,work):
 work.mkdir()
 for f in c['files']:
  path=work/f['to'];path.parent.mkdir(parents=True,exist_ok=True)
  data=(ROOT/'fixtures'/f['fixture']).read_bytes() if 'fixture' in f else json.dumps(FX['model']).encode() if f.get('sourceModel') else f['text'].encode()
  path.write_bytes(data)
 model=copy.deepcopy(FX['model']);model['entities']+=resolve(c['entities'],work)
 if c.get('fontMissing'):model['settings']['textStyles']=[dict(id='font-style',name='Özgün',font='barlow',fontFile='romans.shx')]
 if c.get('service'):model['layers'][1]['service']=dict(kind='xyz',url='https://invalid.example/{z}/{x}/{y}.png')
 doc=cad.Document.from_bytes(json.dumps(model).encode());book=copy.deepcopy(FX['book'])
 if c.get('profile'):
  p=copy.deepcopy(FX['profile']);captured=copy.deepcopy(p);captured['name']='Önceki baskı';captured['entries'][0]['style']['lineWeight']=350
  book['plotProfiles']=[p]
  if c['profile']=='snapshot':book['sheets'][0]['plot']=dict(settings=dict(profile=p['id'],profileSnapshot=captured))
  if c['profile']=='setup':book['pageSetups']=[dict(id='office',name='Ofis',page=book['sheets'][0]['page'],settings=dict(profile=p['id'],profileSnapshot=captured))]
 ops=[dict(op='addSheet',sheet=s) for s in book['sheets']]
 if 'plotProfiles' in book:ops.append(dict(op='savePlotProfiles',profiles=book['plotProfiles']))
 if 'pageSetups' in book:ops.append(dict(op='savePageSetups',setups=book['pageSetups']))
 edit=cad.command('cad.sheets.edit');edit.run(doc,edit.input_type.from_json(dict(ops=ops))).unwrap()
 for command in c.get('commands',[]):
  wrapper=cad.command(command['command']);wrapper.run(doc,wrapper.input_type.from_json(resolve(command['input'],work))).unwrap()
 return doc

def inventory(c,target,report):
 for f in report['files']:
  data=(target/f['name']).read_bytes();assert len(data)==f['bytes'];assert hashlib.sha256(data).hexdigest()==f['sha256']
 if c.get('resources') is False:assert report.get('resources') is None;return
 resources=report['resources'];bykind={};total=0
 for f in resources['files']:
  data=(target/f['path']).read_bytes();total+=len(data);assert len(data)==f['bytes'];assert hashlib.sha256(data).hexdigest()==f['sha256'];bykind.setdefault(f['kind'],[]).append(f)
 assert total==resources['bytes']
 assert sorted(f['sha256'] for f in bykind['font'])==FX['fontSha256']
 assert sorted(f['sha256'] for f in bykind['license'])==FX['licenseSha256']
 assert sorted(f['path'].removeprefix('kaynaklar/') for f in bykind.get('dependency',[]))==c['deps']
 assert len(bykind.get('plotProfile',[]))==c.get('plotFiles',0)
 for f in bykind.get('plotProfile',[]):assert json.loads((target/f['path']).read_text())['schema']=='kentos.plot/1'
 manifest=json.loads((target/'yayin.json').read_text());assert manifest['source']==report['source'];assert manifest['resources']==resources
 _,saved=kcad.read((target/resources['project']).read_bytes());assert saved.get('sheets')

def local_links(saved,folder):
 def entities(values):
  for e in values:
   for path in [e.get('file'),e.get('source',{}).get('path'),*(s.get('file') for s in e.get('sources',[])),*(a.get('file') for a in e.get('attachments',[]))]:
    if path:assert not Path(path).is_absolute() and (folder/path).is_file(),path
 entities(saved['entities'])
 for block in saved.get('blocks',[]):
  if 'xref' in block:
   x=block['xref'];path=folder/x['relative'];assert path.is_file();assert hashlib.sha256(path.read_bytes()).hexdigest()==x['digest']
  else:entities(block.get('entities',[]))
 for s in saved['settings'].get('sources',[]):
  assert (folder/s['relative']).is_file();assert hashlib.sha256((folder/s['relative']).read_bytes()).hexdigest()==s['revision']

class SheetDelivery(unittest.TestCase):
 def test_independent_deliveries_relocation_and_tamper_through_typed_api(self):
  self.assertGreaterEqual(len(FX['cases']),20)
  for n,c in enumerate(FX['cases']):
   with self.subTest(c['name']),tempfile.TemporaryDirectory(prefix='kentos-delivery-python-') as name:
    root=Path(name);work=root/'work';target=root/'delivery';doc=source(c,work)
    if c.get('target') in ('empty','nonempty'):target.mkdir()
    if c.get('target')=='nonempty':(target/'keep.txt').write_bytes(b'keep')
    before=[r.entity.to_json() for r in doc.entities()];before_book=cad.sheets.set(doc,details=True).book.to_json()
    wrapper=cad.command('file.publish.sheets');inp=wrapper.input_type.from_json(dict(path=str(target),drawing=str(work/'source.kcad'),resources=c.get('resources',True)))
    plan=wrapper.run(doc,inp,op='plan')
    if c.get('code'):
     self.assertEqual(plan.error.code,c['code']);self.assertEqual((target/'keep.txt').read_bytes(),b'keep');continue
    self.assertEqual(plan.unwrap().ready,c['ready']);self.assertFalse((target/'yayin.json').exists())
    if not c['ready']:
     self.assertTrue(any(i.code==c['issue'] and i.error for i in plan.output.issues));failed=wrapper.run(doc,inp);self.assertFalse(failed.ok);self.assertFalse(target.exists())
    else:
     request=f'delivery-{n}';issued=wrapper.run(doc,inp,request_id=request).unwrap().to_json();inventory(c,target,issued)
     again=wrapper.run(doc,inp,request_id=request);self.assertTrue(again.replayed);self.assertTrue(again.ok)
     if kind:=c.get('tamper'):
      kinds=dict(font='font',license='license',drawing='drawing',profile='plotProfile',pageSetups='pageSetups',dependency='dependency',missing='font',symlink='font')
      rel=issued['files'][0]['name'] if kind=='pdf' else 'yayin.json' if kind=='manifest' else next(f['path'] for f in issued['resources']['files'] if f['kind']==kinds[kind])
      path=target/rel
      if kind=='missing':path.unlink()
      elif kind=='symlink':path.unlink();(root/'outside.ttf').write_bytes(b'outside');path.symlink_to(root/'outside.ttf')
      else:path.write_bytes(b'changed')
      result=wrapper.run(doc,inp,request_id=request);self.assertTrue(result.replayed);self.assertEqual(result.error.code,'integrity_failed')
     elif resources:=issued.get('resources'):
      moved=root/'moved';target.rename(moved);shutil.rmtree(work)
      _,saved=kcad.read((moved/resources['project']).read_bytes());local_links(saved,moved/'kaynaklar')
      opened=cad.Document.open(moved/resources['project']);self.assertEqual(opened.info().objects,doc.info().objects)
    self.assertEqual([r.entity.to_json() for r in doc.entities()],before);self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),before_book);self.assertFalse(list(root.glob('.kentos-yayin-*')))
