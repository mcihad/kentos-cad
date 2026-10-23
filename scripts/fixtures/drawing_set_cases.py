"""Independent drawing-set reference: identities, subset prefixes, literal titles and ordered index rows."""
import copy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'fixtures/sheet/v1/drawing-set.json'
pub=json.loads((ROOT/'fixtures/commands/v1/file.publish.sheets.json').read_text())
BOOK=pub['book'];MODEL=pub['setup']
def info(number='',title='',revision='',status='',subset=None):
 return dict(number=number,title=title,revision=revision,status=status,subset=subset or [])
def set_(sid,**kwargs):return dict(op='setDrawingInfo',id=sid,info=info(**kwargs))
def rows(book,subset=None,views=False):
 out=[]
 for s in book['sheets']:
  i=s.get('drawingInfo') or info();path=i['subset']
  if path[:len(subset or [])]!=(subset or []):continue
  row=[i['number'],i['title'] or s['name'],i['revision'],i['status'],' / '.join(path)]
  if not views:out.append(row+['']);continue
  items=next((m['items'] for m in book.get('masters',[]) if m['id']==s.get('master')),[])+s['items']
  for it in items:
   if it.get('hidden') or not it.get('printable',True) or it['kind']['type']!='map' or it['kind'].get('overviewOf'):continue
   src=it['kind'].get('source',{'type':'drawing'})
   if src['type']=='drawing' and not it['kind']['view'].get('center'):continue
   view=src.get('view',{}).get('name',it['name'])
   if src['type']=='viewLink':view='Güncel plan' if src['view']['id']=='live' else view+' (kaynak eksik)'
   out.append(row+[view])
 return out
cases=[]
def case(name,ops,*,code=None,book=None,subset=None,views=False):
 book=copy.deepcopy(book or BOOK);expected=copy.deepcopy(book)
 if not code:
  expanded=[]
  for op in ops:
   expanded.extend([dict(op='setDrawingInfo',**e) for e in op['entries']] if op['op']=='setDrawingSet' else [op])
  for op in expanded:
   s=next(s for s in expected['sheets'] if s['id']==op.get('id'))
   if op['op']=='setDrawingInfo':
    if op['info'] is None:s.pop('drawingInfo',None)
    else:s['drawingInfo']=copy.deepcopy(op['info'])
   elif op['op']=='moveSheet':expected['sheets'].remove(s);expected['sheets'].insert(op['to'],s)
   elif op['op']=='duplicateSheet':
    c=copy.deepcopy(s);c['id']=op['newId'];c['name']=op['name'];c.get('drawingInfo',{})['number']=''
    for it,new in zip(c['items'],op['itemIds']):it['id']=new
    expected['sheets'].insert(expected['sheets'].index(s)+1,c)
 cases.append(dict(name=name,book=book,ops=ops,code=code,index=dict(subset=subset or [],views=views),rows=rows(expected,subset,views) if not code else None,metadata={s['id']:s.get('drawingInfo') for s in expected['sheets']} if not code else None))
case('all identity fields',[set_('ground',number='A-101',title='Zemin kat',revision='R02',status='Kontrol',subset=['Mimari','Katlar'])])
case('empty title follows tab',[set_('ground',number='A-102')])
case('literal title is not expression code',[set_('ground',number='TR-İ-01',title='[% 1 + 1 %]',revision='A',status='Taslak')])
case('a subset includes descendants',[set_('ground',number='A-101',subset=['Mimari','Katlar']),set_('section',number='A-201',subset=['Mimari','Kesitler']),set_('detail',number='S-01',subset=['Statik'])],subset=['Mimari'])
case('a child subset excludes siblings',[set_('ground',subset=['Mimari','Katlar']),set_('section',subset=['Mimari','Kesitler'])],subset=['Mimari','Katlar'])
case('prefix is a segment not a text prefix',[set_('ground',subset=['MimariX'])],subset=['Mimari'])
case('duplicate numbers refuse the entire operation',[set_('ground',number='A-01'),set_('section',number='a-01')],code='duplicate_sheet_number')
case('draft blank numbers may repeat',[set_('ground'),set_('section')])
case('field limits are inclusive',[set_('ground',number='N'*64,title='T'*240,revision='R'*64,status='S'*120,subset=['D'*120]*12)])
for key,length in [('number',65),('title',241),('revision',65),('status',121)]:case('too long '+key,[set_('ground',**{key:'x'*length})],code='bad_drawing_info')
for name,kwargs in [('leading space',dict(number=' A')),('trailing space',dict(title='Plan ')),('control text',dict(status='Yayın\n')),('too many subset levels',dict(subset=['A']*13)),('empty subset level',dict(subset=['A',''])),('slash in a level',dict(subset=['A/B'])),('backslash in a level',dict(subset=['A\\B'])),('whitespace level',dict(subset=[' A']))]:case(name,[set_('ground',**kwargs)],code='bad_drawing_info')
b=copy.deepcopy(BOOK);b['sheets'][0]['drawingInfo']=info(number='A-01',title='Eski',revision='A')
case('metadata can be removed',[dict(op='setDrawingInfo',id='ground',info=None)],book=b)
case('revision and title can change',[set_('ground',number='A-01',title='Yeni',revision='B',status='Yayın')],book=b)
case('a duplicate is a new unnumbered drawing',[dict(op='duplicateSheet',id='ground',newId='copy',name='Kopya',itemIds=['copied-'+str(i) for i in range(len(b['sheets'][0]['items']))])],book=b)
case('book order is index order',[dict(op='moveSheet',id='detail',to=0),set_('detail',number='D-01')])
case('view index follows physical viewports',[set_('ground',number='A-01')],views=True)
b=copy.deepcopy(BOOK);b['sheets'][0]['items'][0]['kind']['source']=dict(type='viewLink',view=dict(id='live',name='Eski plan',center=dict(x=2.5,y=0),width=5,height=3,twist=0))
case('named view index follows a renamed source',[],book=b,views=True)
b=copy.deepcopy(b);b['sheets'][0]['items'][0]['kind']['source']['type']='viewSnapshot'
case('snapshot view index keeps captured name',[],book=b,views=True)
b=copy.deepcopy(BOOK);b['sheets'][0]['items'][0]['kind']['view']['center']=None;b['sheets'][1]['items'][0]['hidden']=True
case('unplaced and hidden viewports are absent',[],book=b,views=True)
b=copy.deepcopy(BOOK);b['sheets'][0]['drawingInfo']=info(number='A-01');b['sheets'][1]['drawingInfo']=info(number='A-02')
case('atomic batch exchanges two existing numbers',[dict(op='setDrawingSet',entries=[dict(id='ground',info=info(number='A-02')),dict(id='section',info=info(number='A-01'))])],book=b)
case('duplicate ids in batch are refused',[dict(op='setDrawingSet',entries=[dict(id='ground',info=info(number='A-01')),dict(id='ground',info=info(number='A-02'))])],code='duplicate_id')
case('missing second id leaves first unchanged',[dict(op='setDrawingSet',entries=[dict(id='ground',info=info(number='A-01')),dict(id='gone',info=info(number='A-02'))])],code='unknown_sheet')
case('atomic batch removes metadata',[dict(op='setDrawingSet',entries=[dict(id='ground',info=None),dict(id='section',info=None)])],book=b)
b=copy.deepcopy(BOOK);it=copy.deepcopy(b['sheets'][0]['items'][0]);it['id']='master-map';it['name']='Ortak görünüm'
b['masters']=[dict(id='master',name='Ortak',page=copy.deepcopy(b['sheets'][0]['page']),items=[it])]
b['sheets'][0]['master']='master';b['sheets'][1]['master']='master'
case('master view has a separate row on every owner page',[],book=b,views=True)
b=copy.deepcopy(BOOK);b['sheets'][0]['items'][0]['printable']=False
case('nonprintable view does not enter the index',[],book=b,views=True)
fx=dict(format='kentos.sheet-drawing-set/1',model=MODEL,book=BOOK,namedViews=[dict(id='live',name='Güncel plan',center=dict(x=2.5,y=0),width=5,height=3,twist=0)],cases=cases)
text=json.dumps(fx,ensure_ascii=False,indent=1)+'\n'
if '--check' in sys.argv:assert OUT.read_text()==text
else:OUT.write_text(text);print(len(cases),'drawing-set cases')
