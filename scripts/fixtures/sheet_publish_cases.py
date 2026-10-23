"""Independent ordered publication/naming cases; never imports KentOS or computes expected values from its output."""
import copy,json,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'scripts/fixtures'))
from sheets_edit_command_cases import SETUP,sheet
IDS=['ground','section','detail'];NAMES=['Kat planı','Kesit','Detay']
book=dict(schema='kentos.sheet/1',sheets=[sheet(i,n,50) for i,n in zip(IDS,NAMES)],masters=[],assets=[],variables=[])
for number,s in enumerate(book['sheets'],1):
 s['items'].append(dict(id=s['id']+'-title',name='Başlık',frame=dict(left=30000,top=260000,width=200000,height=10000),kind=dict(type='text',content='[% @pafta_adi %]',style=dict(size=3500))))
 s['items'].append(dict(id=s['id']+'-marker',name='Sıra kontrolü',frame=dict(left=30000,top=245000,width=number*10000,height=1000),kind=dict(type='line',points=[[0,0],[1000000,0]],stroke=dict(width=250,color='#aa22cc'))))
cases=[]
def good(name,inp,files,ids=None,prepare=None):
 chosen=ids or IDS
 expected=dict(ready=True,sheets=chosen,files=[dict(name=n,sheets=s) for n,s in files])
 cases.append(dict(name=name,prepare=prepare or {},steps=[dict(op='plan',input={'path':'$out',**inp},result=dict(status='completed',output=expected),published=False),dict(op='execute',input={'path':'$out',**inp},result=dict(status='completed',output=expected),published=True)]))
def bad(name,inp,code,prepare=None,status='failed'):
 cases.append(dict(name=name,prepare=prepare or {},steps=[dict(op='execute',input={'path':'$out',**inp},result=dict(status=status,error=dict(code=code)),published=False)]))
good('all, one PDF',{},[('paftalar.pdf',IDS)])
good('all, separate',dict(mode='separate'),[(f'{n:03}-{v}.pdf',[i]) for n,(i,v) in enumerate(zip(IDS,NAMES),1)])
rev=['detail','ground']
good('explicit reverse order single',dict(sheets=rev,name='Kontrol'),[('Kontrol.pdf',rev)],rev)
good('explicit reverse order separate',dict(sheets=rev,mode='separate'),[('001-Detay.pdf',['detail']),('002-Kat planı.pdf',['ground'])],rev)
good('sheet name pattern',dict(mode='separate',naming='{sheet}'),[(v+'.pdf',[i]) for i,v in zip(IDS,NAMES)])
good('stable source id pattern',dict(mode='separate',naming='{id}'),[(i+'.pdf',[i]) for i in IDS])
good('sanitized title collision',dict(mode='separate',naming='{sheet}'),[('Kat_Plan.pdf',['ground']),('Kat_Plan-2.pdf',['section']),('Detay.pdf',['detail'])],prepare=dict(names={'ground':'Kat/Plan','section':'Kat:Plan'}))
good('constant name collision',dict(mode='separate',naming='pafta'),[(f'pafta{"" if n==0 else "-"+str(n+1)}.pdf',[i]) for n,i in enumerate(IDS)])
good('portable reserved name',dict(name='CON'),[('_CON.pdf',IDS)])
good('title is a leaf, never a path',dict(mode='separate',naming='{sheet}'),[('Yatak_Oda_01.pdf',['ground']),('Kesit.pdf',['section']),('Detay.pdf',['detail'])],prepare=dict(names={'ground':'Yatak/Oda:01'}))
bad('explicit no selection',dict(sheets=[]),'no_sheets')
bad('unknown selected sheet',dict(sheets=['gone']),'unknown_sheet')
bad('duplicate selected sheet',dict(sheets=['ground','ground']),'invalid_input')
bad('unknown name placeholder',dict(mode='separate',naming='{wrong}'),'invalid_input')
bad('relative destination',dict(path='relative-folder'),'invalid_file_path')
bad('existing nonempty folder survives',{},'file_exists',dict(target='nonempty'))
bad('preview pin moved',dict(pin=dict(revision='0',sha256='0'*64)),'pin_mismatch',status='conflict')
bad('no sheet book',{},'no_sheets',dict(noBook=True))
for label,prepare,code in [('missing named view after a good page',dict(missing=['section']),'missing_named_view'),('unplaced model frame after a good page',dict(unplaced=['section']),'map_unplaced')]:
 cases.append(dict(name=label,prepare=prepare,steps=[dict(op='plan',input=dict(path='$out',mode='separate'),result=dict(status='completed',output=dict(ready=False)),issue=code,published=False),dict(op='execute',input=dict(path='$out',mode='separate'),result=dict(status='failed',error=dict(code='publish_blocked')),published=False)]))
bad('invalid requested dpi',dict(dpi=0),'invalid_input')
bad('empty name pattern',dict(mode='separate',naming=''),'invalid_input')
good('empty folder may become a complete set',{},[('paftalar.pdf',IDS)],prepare=dict(target='empty'))
good('single name cannot traverse',dict(name='../Kontrol'),[('_Kontrol.pdf',IDS)])
good('an explicit resolution keeps source page settings',dict(dpi=150,mode='separate'),[(f'{n:03}-{v}.pdf',[i]) for n,(i,v) in enumerate(zip(IDS,NAMES),1)])
good('broken unselected page does not poison selected output',dict(sheets=rev),[('paftalar.pdf',rev)],rev,dict(missing=['section']))
# Long names collide after byte truncation; each still gets a unique portable leaf.
good('long pattern has bounded unique names',dict(mode='separate',naming='x'*300),[('x'*200+suffix+'.pdf',[i]) for i,suffix in zip(IDS,['','-2','-3'])])
cases.append(dict(name='missing linked image is reported for every affected sheet',prepare=dict(imageMissing=True),steps=[dict(op='plan',input=dict(path='$out',mode='separate'),result=dict(status='completed',output=dict(ready=False)),issue='plot_asset_unreadable',published=False),dict(op='execute',input=dict(path='$out',mode='separate'),result=dict(status='failed',error=dict(code='publish_blocked')),published=False)]))
fixture=dict(format='kentos.command-cases',version=1,command='file.publish.sheets',commandVersion=1,host='headless-file',title='Ordered atomic sheet publication',note='Native filesystem host cases; Rust headless and the typed Python file runner use the same cases. Selection and leaf names are independently specified here.',setup=copy.deepcopy(SETUP),book=book,markers=dict(zip(IDS,[10,20,30])),cases=cases)
p=ROOT/'fixtures/commands/v1/file.publish.sheets.json';text=json.dumps(fixture,ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
