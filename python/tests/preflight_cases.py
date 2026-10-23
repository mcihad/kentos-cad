"""Independent paper/model physical preflight through typed Python; the query never writes an output."""
import copy,json,tempfile,base64
from pathlib import Path
import kentos.cad as cad
from test_sheet_delivery import resolve
ROOT=Path(__file__).resolve().parents[2]
def source(fx,c,root):
 for f in c.get('files',[]):
  path=root/f['to'];path.parent.mkdir(parents=True,exist_ok=True)
  path.write_bytes((ROOT/'fixtures'/f['fixture']).read_bytes() if 'fixture' in f else f['text'].encode())
 model=copy.deepcopy(fx['model'])
 if 'entity' in c:model['entities'].append(resolve(c['entity'],root))
 if 'unit' in c:model['settings']['drawingUnit']=c['unit']
 if c.get('resourceLayerVisible'):model['layers'][1]['visible']=True
 if c.get('service'):model['layers'][1]['service']=dict(kind='xyz',url='https://invalid.example/{z}/{x}/{y}.png')
 if c.get('fontMissing'):model['settings']['textStyles']=[dict(id='font-style',name='Özgün',font='barlow',fontFile='romans.shx')]
 doc=cad.Document.from_bytes(json.dumps(model).encode())
 if c.get('noBook'):return doc
 book=copy.deepcopy(fx['book']);sheet=book['sheets'][0];frame=sheet['items'][0]
 frame['kind'].update(c.get('patch',{}));frame.update(c.get('item',{}))
 if 'center' in c:frame['kind']['view']['center']=c['center']
 if 'scale' in c:frame['kind']['view']['scale']=c['scale']
 if 'factor' in c:sheet['plot']=dict(settings=dict(area=dict(type='window',rect=dict(left=0,top=0,width=420000,height=297000)),scale=dict(type='ratio',ratio=dict(paper=c['factor'],model=1))))
 if 'extraItem' in c:sheet['items'].append(c['extraItem'])
 pictures=[];ops=[]
 if c.get('png'):
  ops.append(dict(op='addAssets',assets=[c['assetMeta']]))
  pictures=[dict(sha256=c['assetMeta']['sha256'],bytes=base64.b64encode((ROOT/'fixtures/interaction/v1/logo.png').read_bytes()).decode())]
  sheet['items'].append(dict(id='png',name='PNG',frame=c['pictureFrame'],kind=dict(type='picture',asset=c['assetMeta']['sha256'],fit=c['fit'])))
 if 'svg' in c:
  ops.append(dict(op='addAssets',assets=[c['svgMeta']]))
  pictures=[dict(sha256=c['svgMeta']['sha256'],bytes=base64.b64encode(c['svg'].encode()).decode())]
  sheet['items'].append(dict(id='svg',name='SVG',frame=dict(left=10000,top=10000,width=10000,height=10000),kind=dict(type='picture',asset=c['svgMeta']['sha256'],fit='contain')))
 ops.append(dict(op='addSheet',sheet=sheet));wrapper=cad.command('cad.sheets.edit');wrapper.run(doc,wrapper.input_type.from_json(dict(ops=ops,pictures=pictures))).unwrap()
 return doc
def run_fixture(fx):
 assert fx['command']=='file.preflight.sheets' and len(fx['cases'])>=20
 wrapper=cad.command('file.preflight.sheets')
 for case in fx['cases']:
  try:
   with tempfile.TemporaryDirectory(prefix='kentos-preflight-python-') as name:
    root=Path(name);doc=source(fx,case,root);before=doc.to_bytes();files=sorted(p.relative_to(root).as_posix() for p in root.rglob('*'));inp=wrapper.input_type.from_json(dict(drawing=str(root/'source.kcad'),sheets=['ground'])|case.get('input',{}))
    first=None
    for op in ['plan','execute']:
     result=wrapper.run(doc,inp,op=op)
     if 'code' in case:assert result.error.code==case['code'];continue
     report=result.unwrap();codes=[i.code for i in report.issues]
     for code in case['expect']:assert code in codes,(code,codes)
     for code in case.get('absent',[]):assert code not in codes,(code,codes)
     assert report.ready==(not any(i.error for i in report.issues))
     if first is None:first=report.to_json()
     else:assert report.to_json()==first
    assert doc.to_bytes()==before
    assert sorted(p.relative_to(root).as_posix() for p in root.rglob('*'))==files
  except Exception as e:raise AssertionError(f"{fx['command']} › {case['name']}: {e}") from e
 return len(fx['cases'])
