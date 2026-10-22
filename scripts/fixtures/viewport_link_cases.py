"""Named view links and independent current cameras: no KentOS calls."""
import copy,json,math,sys
from pathlib import Path
from sheets_edit_command_cases import sheet
out=[]
for scale in [.5,20,37.5,50,100]:
 for rotation in [0,30,90,137]:
  s=sheet('page','A-101',50);i=s['items'][0];i['id']='viewport';i['padding']=2000;i['kind']['labelBand']=3000
  captured={'id':'detail-a','name':'Detay A','center':{'x':2.5,'y':1.5},'width':5.,'height':3.,'twist':0.}
  current={'id':'detail-a','name':'Detay A güncel','center':{'x':500002.5,'y':4500001.5},'width':scale*.38,'height':scale*.11,'twist':math.radians(rotation)}
  unrelated={'id':'other','name':'Detay A','center':{'x':0.,'y':0.},'width':100.,'height':100.,'twist':0.}
  out.append({'name':f'live {scale} rotation {rotation}','sheet':s,'operation':{'op':'setViewport','id':'viewport','source':{'type':'viewLink','view':captured}},'views':[unrelated,current],'expected':{'center':current['center'],'scale':scale,'rotation':rotation*1000}})
p=Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/viewport-links.json';text=json.dumps({'version':1,'cases':out},ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
