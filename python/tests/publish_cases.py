"""Filesystem-host fixture runner: typed publication, independent PDF operators and hashlib."""
import copy,hashlib,json,math,tempfile
from pathlib import Path
import kentos.cad as cad
from acceptance_files import pdf_pages

def subset(got,want):
 if isinstance(want,dict):
  for k,v in want.items():subset(got[k],v)
 elif isinstance(want,list):
  assert len(got)==len(want),(got,want)
  for a,b in zip(got,want):subset(a,b)
 else:assert got==want,(got,want)

def run_fixture(fx):
 assert fx['command']=='file.publish.sheets' and len(fx['cases'])>=20
 problems=[]
 for case in fx['cases']:
  try:
   with tempfile.TemporaryDirectory(prefix='kentos-publish-python-') as folder:
    root=Path(folder);target=root/'delivery';prep=case['prepare']
    if prep.get('target') in ('empty','nonempty'):target.mkdir()
    if prep.get('target')=='nonempty':(target/'keep.txt').write_bytes(b'keep')
    model=copy.deepcopy(fx['setup'])
    if prep.get('imageMissing'):model['entities'].append(dict(kind='image',id=90000,layerId='cizim',attrs={},file=str(root/'missing.png'),p=dict(x=0,y=1),width=1,height=1,rotation=0))
    doc=cad.Document.from_bytes(json.dumps(model).encode())
    if not prep.get('noBook'):
     book=copy.deepcopy(fx['book'])
     for sheet in book['sheets']:
      sid=sheet['id'];sheet['name']=prep.get('names',{}).get(sid,sheet['name'])
      if sid in prep.get('missing',[]):sheet['items'][0]['kind']['source']=dict(type='viewLink',view=dict(id='gone',name='Silinmiş',center=dict(x=2.5,y=0),width=5,height=3,twist=0))
      if sid in prep.get('unplaced',[]):sheet['items'][0]['kind']['view']['center']=None
     cad.sheets.edit(doc,ops=[cad.AddSheetPaperOp(sheet=cad.PaperSheet.from_json(s)) for s in book['sheets']])
    wrapper=cad.command(fx['command'])
    before=[r.entity.to_json() for r in doc.entities()];before_book=cad.sheets.set(doc,details=True).book
    before_book=before_book.to_json() if before_book else None
    for step in case['steps']:
     inp=copy.deepcopy(step['input'])
     if inp['path']=='$out':inp['path']=str(target)
     result=wrapper.run(doc,wrapper.input_type.from_json(inp),op=step['op'])
     got=dict(status=result.status)
     if result.error:got['error']=dict(code=result.error.code)
     if result.output:got['output']=result.output.to_json()
     subset(got,step['result'])
     if 'issue' in step:assert any(i['code']==step['issue'] and i['error'] for i in got['output']['issues']),got
     assert (target/'yayin.json').exists()==step['published']
     if step['published']:
      manifest=json.loads((target/'yayin.json').read_text());assert manifest['sheets']==got['output']['sheets']
      assert hashlib.sha256((target/'yayin.json').read_bytes()).hexdigest()==got['output']['sha256']
      for f in got['output']['files']:
       data=(target/f['name']).read_bytes();assert len(data)==f['bytes'];assert hashlib.sha256(data).hexdigest()==f['sha256']
       pages=pdf_pages(data);assert len(pages)==len(f['sheets'])
       for page,sid in zip(pages,f['sheets']):
        for length in [100,fx['markers'][sid]]:assert any(abs(math.hypot(p[2]-p[0],p[3]-p[1])-length)<.002 for p in page['segments']),(sid,length)
     if prep.get('target')=='nonempty':assert (target/'keep.txt').read_bytes()==b'keep'
     assert not list(root.glob('.kentos-yayin-*'))
     assert [r.entity.to_json() for r in doc.entities()]==before
     kept=cad.sheets.set(doc,details=True).book;assert (kept.to_json() if kept else None)==before_book
  except Exception as error:problems.append(f"{case['name']}: {type(error).__name__}: {error}")
 assert not problems,'\n'.join(problems)
 return len(fx['cases'])
