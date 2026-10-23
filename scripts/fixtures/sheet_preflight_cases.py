#!/usr/bin/env python3
"""Independent physical print preflight inputs: model metres, viewport ratio and final paper factor."""
import argparse,copy,json,struct,hashlib,base64
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'fixtures/sheet/v1/native-preflight.json'
def make():
 base=json.loads((ROOT/'fixtures/sheet/v1/delivery.json').read_text());book=copy.deepcopy(base['book']);book['sheets']=book['sheets'][:1];book['sheets'][0]['items'][0]['kind']['viewLocked']=True
 cases=[]
 def add(name,**kw):cases.append(dict(name=name,**kw))
 add('locked-visible-map',expect=[],absent=['viewport_scale_unlocked','plot_provider_unsupported'])
 add('unlocked-map',patch={'viewLocked':False},expect=['viewport_scale_unlocked'])
 add('item-lock-also-protects-scale',patch={'viewLocked':False},item={'locked':True},expect=[],absent=['viewport_scale_unlocked'])
 add('hidden-map-not-a-warning',patch={'viewLocked':False},item={'hidden':True},expect=[],absent=['viewport_scale_unlocked'])
 add('unprinted-map-not-a-warning',patch={'viewLocked':False},item={'printable':False},expect=[],absent=['viewport_scale_unlocked'])
 width,height=struct.unpack('>II',(ROOT/'fixtures/interaction/v1/logo.png').read_bytes()[16:24])
 for unit in ['mm','cm','m']:
  for ppi,scale,factor in [(149,50,1),(150,50,1),(151,50,1),(74.5,100,2),(300,25,.5)]:
   # Full-source pixels divided by final physical inches, independently of the drawing display unit.
   w=width*25.4/ppi*scale/1000/factor;h=height*25.4/ppi*scale/1000/factor
   entity=dict(kind='image',id=20,layerId='cizim',attrs={},file='$work/logo.png',p=dict(x=0,y=0),width=w,height=h,rotation=0)
   add(f'image-{unit}-{ppi}ppi-scale{scale}-factor{factor}',unit=unit,scale=scale,factor=factor,entity=entity,files=[dict(fixture='interaction/v1/logo.png',to='logo.png')],expect=['low_resolution'] if ppi<150 else [],absent=[] if ppi<150 else ['low_resolution'],ppi=ppi)
 add('missing-visible-image',entity=dict(kind='image',id=20,layerId='cizim',attrs={},file='$work/gone.png',p=dict(x=0,y=0),width=1,height=1,rotation=0),expect=['missing_reference'])
 for kind in ['raster','cloud']:
  original=next(c for c in base['cases'] if any(e['kind']==('raster' if kind=='raster' else 'pointcloud') for e in c['entities']))
  entity=copy.deepcopy(original['entities'][0]);entity['layerId']='resources';outside=copy.deepcopy(entity)
  view={}
  if kind=='raster':entity['affine'][0]=0;entity['affine'][3]=0
  else:view['center']=dict(x=(entity['bounds'][0]+entity['bounds'][3])/2,y=(entity['bounds'][1]+entity['bounds'][4])/2)
  add(f'{kind}-visible-refused',entity=entity,files=original['files'],resourceLayerVisible=True,expect=['plot_provider_unsupported'],**view)
  add(f'{kind}-hidden-omission-is-intentional',entity=entity,files=original['files'],expect=[],absent=['plot_provider_unsupported'],**view)
  add(f'{kind}-outside-viewport-not-printed',entity=outside,files=original['files'],resourceLayerVisible=True,expect=[],absent=['plot_provider_unsupported'])
 add('visible-map-service-refused',service=True,resourceLayerVisible=True,expect=['plot_provider_unsupported'])
 add('hidden-map-service-not-printed',service=True,expect=[],absent=['plot_provider_unsupported'])
 add('off-page-paper-item',extraItem=dict(id='off',name='Kâğıt dışında',frame=dict(left=500000,top=10000,width=10000,height=10000),kind=dict(type='text',content='Taşma',style=dict(size=3500))),expect=['off_page'])
 add('paper-text-does-not-fit',extraItem=dict(id='small',name='Dar yazı',frame=dict(left=10000,top=10000,width=1000,height=1000),kind=dict(type='text',content='Bu yazı bu kutuya sığamaz',style=dict(size=9000))),expect=['text_overflow'])
 pngdata=(ROOT/'fixtures/interaction/v1/logo.png').read_bytes()
 for name,w,h,fit,factor,dpi in [('stretched-tall',10000,200000,'stretch',1,None),('stretched-wide',300000,10000,'stretch',1,None),('contained',10000,200000,'contain',1,None),('covered',10000,200000,'cover',1,None),('original-low',10000,10000,'original',1,96),('original-high',10000,10000,'original',1,320),('original-enlarged',10000,10000,'original',3,320)]:
  if fit=='stretch':iw,ih=w,h
  elif fit=='original':iw,ih=round(width*25400/dpi),round(height*25400/dpi)
  else:
   scale=(min if fit=='contain' else max)(w/width,h/height);iw,ih=round(width*scale),round(height*scale)
  ppi=min(width/(iw/25400*factor),height/(ih/25400*factor))
  meta=dict(name='Logo',sha256=hashlib.sha256(pngdata).hexdigest(),kind='png',width=width,height=height,bytes=len(pngdata))
  if dpi:meta['dpi']=dpi
  add('paper-image-'+name,png=True,assetMeta=meta,pictureFrame=dict(left=10000,top=10000,width=w,height=h),fit=fit,factor=factor,expect=['low_resolution'] if ppi<150 else [],absent=[] if ppi<150 else ['low_resolution'])
 add('missing-model-annotation-link',entity=dict(kind='text',id=20,layerId='cizim',attrs={},p=dict(x=0,y=0),text='Son değer',height=.1,rotation=0,labelOf='00000000-0000-5000-8000-000000000099',labelScale=50),expect=['broken_annotation'])
 add('substituted-model-font',fontMissing=True,entity=dict(kind='text',id=20,layerId='cizim',attrs={},p=dict(x=0,y=0),text='Özgün',height=.1,rotation=0,textStyle='font-style'),expect=['missing_font'])
 add('missing-paper-font',extraItem=dict(id='font',name='Özgün font',frame=dict(left=10000,top=10000,width=100000,height=10000),kind=dict(type='text',content='Türkçe ğüş',style=dict(size=3500,font='missing-office-font'))),expect=['missing_font'])
 png=base64.b64encode((ROOT/'fixtures/interaction/v1/logo.png').read_bytes()).decode()
 for name,body,bad in [('external-image','<image href="gone.png" width="10" height="10"/>',True),('external-css','<rect width="10" height="10" fill="url(gone.svg#paint)"/>',True),('internal-use','<defs><rect id="r" width="10" height="10" fill="red"/></defs><use href="#r"/>',False),('embedded-image',f'<image href="data:image/png;base64,{png}" width="10" height="10"/>',False)]:
  svg=f'<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">{body}</svg>'
  add('svg-'+name,svg=svg,svgMeta=dict(name='SVG',sha256=hashlib.sha256(svg.encode()).hexdigest(),kind='svg',width=10,height=10,bytes=len(svg.encode())),expect=['plot_asset_unreadable'] if bad else [],absent=[] if bad else ['plot_asset_unreadable'])
 for name,input,code in [('empty-selection',dict(sheets=[]),'no_sheets'),('unknown-page',dict(sheets=['gone']),'unknown_sheet'),('repeated-page',dict(sheets=['ground','ground']),'invalid_input'),('dpi-too-low',dict(dpi=35),'invalid_input'),('dpi-too-high',dict(dpi=1201),'invalid_input')]:add(name,input=input,code=code,expect=[])
 add('no-book',noBook=True,code='no_sheets',expect=[])
 return dict(format='kentos.command-cases',version=1,command='file.preflight.sheets',commandVersion=1,host='headless-file',title='Native physical print preflight',reference='Independent Python physical inches formula and explicit resource/visibility policy.',model=base['model'],book=book,cases=cases)
def main():
 p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args();text=json.dumps(make(),ensure_ascii=False,indent=2)+'\n'
 if a.check:assert OUT.read_text()==text,'native preflight fixture is stale'
 else:OUT.write_text(text)
 command=ROOT/'fixtures/commands/v1/file.preflight.sheets.json'
 if a.check:assert command.read_text()==text,'preflight command fixture is stale'
 else:command.write_text(text)
if __name__=='__main__':main()
