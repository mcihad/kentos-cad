"""Independent delivery inventory. Files, font digests and expected refusals come from inputs, not KentOS output."""
import copy,hashlib,json,struct,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'fixtures/sheet/v1/delivery.json'
base=json.loads((ROOT/'fixtures/commands/v1/file.publish.sheets.json').read_text());MODEL=copy.deepcopy(base['setup']);MODEL['settings']['srid']=5256;BOOK=base['book']
layer=copy.deepcopy(MODEL['layers'][0]);layer.update(id='resources',name='Kaynaklar',visible=False);MODEL['layers'].append(layer)
FONTDIR=ROOT/'apps/desktop/assets/fonts/drawing'
fonts=sorted(hashlib.sha256(p.read_bytes()).hexdigest() for p in FONTDIR.glob('*.ttf'))
licenses=sorted(hashlib.sha256(p.read_bytes()).hexdigest() for p in FONTDIR.glob('*-OFL.txt'))
profile=copy.deepcopy(json.loads((ROOT/'fixtures/sheet/v1/plot-profiles.json').read_text())['cases'][0]['exchange']['profile'])
profile['assignments']=[]
def image(path,i=100):return dict(kind='image',id=i,layerId='resources',attrs={},p=dict(x=1000,y=1000),width=1,height=1,rotation=0,file=path)
def raster(path=None,url=None):
 e=dict(kind='raster',id=100,layerId='resources',attrs={},affine=[1000,1,0,1000,0,-1],width=4,height=4,bands=3,sample='u8',srid=0,style=dict(render='rgb',bands=[1,2,3]))
 e['file' if path else 'url']=path or url;return e
def f(to,fixture=None,text=None,source_model=False):return {k:v for k,v in dict(to=to,fixture=fixture,text=text,sourceModel=source_model or None).items() if v is not None}
cases=[]
def add(name,files=None,entities=None,deps=None,ready=True,**extra):
 cases.append(dict(name=name,files=files or [],entities=entities or [],deps=sorted(deps or []),ready=ready,**extra))
logo=f('assets/logo.png','interaction/v1/logo.png')
add('source drawing bundled fonts licenses and revision manifest')
add('absolute linked image',[logo],[image('$work/assets/logo.png')],['veri/logo.png'])
add('relative linked image',[logo],[image('assets/logo.png')],['veri/logo.png'])
add('two image links share one copy',[logo],[image('assets/logo.png'),image('assets/logo.png',101)],['veri/logo.png'])
add('equal leaf names get separate copies',[logo,f('other/logo.png','raster/v1/files/rgb.png')],[image('assets/logo.png'),image('other/logo.png',101)],['veri/logo.png','veri/logo-2.png'])
add('portable case collision',[f('assets/Logo.PNG','interaction/v1/logo.png'),f('other/logo.png','raster/v1/files/rgb.png')],[image('assets/Logo.PNG'),image('other/logo.png',101)],['veri/Logo.PNG','veri/logo-2.png'])
add('raster brings world and auxiliary files',[f('rgb.png','raster/v1/files/rgb.png'),f('rgb.pgw','raster/v1/files/rgb.pgw'),f('rgb.png.aux.xml',text='<PAMDataset/>')],[raster('rgb.png')],['veri/rgb.png','veri/rgb.pgw','veri/rgb.png.aux.xml'])
table=dict(kind='table',id=100,layerId='resources',attrs={},p=dict(x=1000,y=1000),rotation=0,height=.1,rows=[.2],columns=[1.,1.],cells=[['A','10']],source=dict(kind='file',name='alan.csv',path='alan.csv'))
add('linked table source',[f('alan.csv',text='ad,alan\nA,10\n')],[table],['veri/alan.csv'])
line=dict(kind='line',id=100,layerId='resources',attrs={},a=dict(x=1000,y=1000),b=dict(x=1001,y=1000),attachments=[dict(name='Tutanak',file='tutanak.pdf')])
add('attachment is carried',[f('tutanak.pdf',text='%PDF-1.4\n% source attachment\n')],[line],['veri/tutanak.pdf'])
# Counts/bounds of the actual LAS input, read directly from its standard header.
las=(ROOT/'fixtures/pointcloud/v1/files/tile-a.las').read_bytes();n=struct.unpack_from('<I',las,107)[0];b=struct.unpack_from('<6d',las,179);bounds=[b[1],b[3],b[5],b[0],b[2],b[4]]
cloud=dict(kind='pointcloud',id=100,layerId='resources',attrs={},sources=[dict(file='cloud.las',format='las',count=n,bounds=bounds)],bounds=bounds,count=n,srid=0,style=dict(render='rgb',size=3))
add('local pointcloud source',[f('cloud.las','pointcloud/v1/files/tile-a.las')],[cloud],['veri/cloud.las'])
add('project data profile and mapping',[f('profiller/park-bahce.profil.json','profiles/v1/files/park-bahce.profil.json'),f('profiller/sivas-bel.eslem.json','profiles/v1/files/sivas-bel.eslem.json')],deps=['profiller/park-bahce.profil.json','profiller/sivas-bel.eslem.json'])
add('bound GeoPackage stays portable',[f('kadastro.gpkg','geopackage/v1/qgis.gpkg')],deps=['veri/kadastro.gpkg'],commands=[dict(command='cad.sources.edit',input=dict(edit=dict(kind='link',path='$work/kadastro.gpkg',table='parsel')))])
add('external CAD reference source',[f('plan.kcad',source_model=True)],deps=['veri/plan.kcad'],commands=[dict(command='cad.xrefs.edit',input=dict(drawing='$work/source.kcad',operation=dict(kind='attach',path='$work/plan.kcad',layerId='resources')))])
add('library plot profile',profile='library',plotFiles=1)
add('captured profile differs from edited library',profile='snapshot',plotFiles=2)
add('page setup also carries its captured profile',profile='setup',plotFiles=2)
add('missing hidden source still blocks a complete package',entities=[image('missing.png')],ready=False,issue='package_incomplete')
add('remote raster must be downloaded explicitly',entities=[raster(url='https://invalid.example/source.tif')],ready=False,issue='package_incomplete')
add('substitute font is not an exact source delivery',fontMissing=True,ready=False,issue='package_incomplete')
add('hidden map service is not a complete offline source',service=True,ready=False,issue='package_incomplete')
add('nonportable resource name is reported',[f('bad:name.png','interaction/v1/logo.png')],[image('bad:name.png')],['veri/bad:name.png'],ready=False,issue='package_incomplete')
add('reserved resource leaf is reported',[f('CON.png','interaction/v1/logo.png')],[image('CON.png')],['veri/CON.png'],ready=False,issue='package_incomplete')
add('explicit PDF only does not claim a resource package',entities=[image('missing.png')],resources=False)
add('empty target is accepted',target='empty')
add('nonempty target is preserved',target='nonempty',code='file_exists',ready=False)
for kind in ['pdf','font','license','drawing','profile','pageSetups','manifest','dependency','missing','symlink']:
 add('recorded delivery refuses tampered '+kind,[logo],[image('assets/logo.png')],['veri/logo.png'],profile='snapshot',plotFiles=2,tamper=kind)
fx=dict(format='kentos.sheet-delivery/1',model=MODEL,book=BOOK,profile=profile,fontSha256=fonts,licenseSha256=licenses,cases=cases)
text=json.dumps(fx,ensure_ascii=False,indent=1)+'\n'
if '--check' in sys.argv:assert OUT.read_text()==text
else:OUT.write_text(text);print(len(cases),'delivery cases')
