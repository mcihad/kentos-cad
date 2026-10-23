"""Independent annotation representation, layer freeze and CAD appearance combinations."""
import copy,json,sys
from pathlib import Path
from sheets_edit_command_cases import sheet,SETUP
cases=[]
for physical in [.5,20,37.5,50,100]:
 for mode,color,weight,pattern in [('auto','#D51E23',500,'dashed'),('half','#1747AA',700,'dashdot'),('missing','#874DB1',350,'dotted'),('frozen','#006F44',250,'continuous')]:
  model=copy.deepcopy(SETUP);model['settings']['plotScale']=50
  model['layers']=[{'id':id,'name':id,'type':'layer','visible':True,'locked':False,'expanded':True,'style':{'color':'#111111','lineType':'continuous','lineWeight':.25},'children':[]} for id in ['geom','anno']];model['activeLayer']='geom'
  annotation=physical if mode in ['auto','frozen'] else physical/2 if mode=='half' else physical/4
  scales=sorted(set([physical,physical/2])-set([50]))
  model['entities']=[{'kind':'line','id':1,'layerId':'geom','attrs':{},'a':{'x':-2.5,'y':-.02*physical},'b':{'x':2.5,'y':-.02*physical}},
   {'kind':'line','id':2,'layerId':'geom','attrs':{},'color':'#00AA00','a':{'x':-2.5,'y':.02*physical},'b':{'x':2.5,'y':.02*physical}},
   {'kind':'text','id':3,'layerId':'anno','attrs':{},'p':{'x':0.,'y':0.},'text':'ANNO','height':.125,'rotation':0.,'annotative':{'at':50.,'scales':[{'scale':v} for v in scales]}}]
  page=sheet('page','A-101',1);i=page['items'][0];i['id']='viewport';i['kind']['view']['center']={'x':0.,'y':0.};i['kind']['view']['ratio']={'paper':1.,'model':physical};i['kind']['viewLocked']=True
  op={'op':'setViewport','id':'viewport','annotation':{'type':'auto'} if mode in ['auto','frozen'] else {'type':'fixed','scale':annotation},'layers':{'type':'all'} if mode!='half' else {'type':'list','layers':['geom','anno']},'layerOverrides':[{'layer':'geom','color':color,'lineWeight':weight,'lineType':pattern},{'layer':'anno','frozen':mode=='frozen','color':color}]}
  cases.append({'name':f'camera {physical} {mode}','model':model,'sheet':page,'operation':op,'expected':{'physical':physical,'annotation':annotation,'visible':mode in ['auto','half'],'heightMetres':.125*annotation/50,'heightPaperMm':2.5*annotation/physical,'color':color,'weightUm':weight,'dash':pattern!='continuous','annoFrozen':mode=='frozen'}})
path=Path(__file__).resolve().parents[2]/'fixtures/sheet/v1/viewport-annotations.json';text=json.dumps({'version':1,'cases':cases},ensure_ascii=False,indent=2)+'\n'
if '--check' in sys.argv:assert path.read_text()==text
else:path.write_text(text)
