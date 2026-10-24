#!/usr/bin/env python3
"""Independent rational straight-partition reference; no KentOS code or executable.
Source faces come from justification + offset; a clear gap includes both half
thicknesses, while an axis gap does not. Cardinal and 3–4–5 frames use exact Fractions.
"""
import argparse
import copy
import json
from fractions import Fraction as F
from pathlib import Path
from walls_rectangle_cases import SETUP

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/cad.walls.partition.json'
def point(x,y): return {'x':float(x),'y':float(y)}
def build():
    cases=[]
    for index in range(72):
        ux,uy=[(1,0),(0,1),(-1,0),(0,-1),(F(3,5),F(4,5)),(F(-4,5),F(3,5))][index%6]
        nx,ny=-uy,ux
        justify=['left','center','right'][(index//6)%3]
        side=-1 if (index//18)%2 else 1
        axis=bool(index//36)
        width=F(15 if index%2 else 25,100)
        origin=(500000,4400000) if index%3==0 else (0,0)
        x,y=map(F,origin);offset=F(12,100) if index%2 else F(0)
        source={'kind':'wall','id':1,'layerId':'base','attrs':{},'pts':[point(x,y),point(x+8*ux,y+8*uy)],
                'width':0.2,'justify':justify}
        if offset:source['offset']=float(offset)
        setup=copy.deepcopy(SETUP);setup['entities']=[source]
        setup['settings']['drawingUnit']=['m','mm','cm','in','ft'][index%5]
        left=offset+{'left':F(0),'center':F(1,10),'right':F(1,5)}[justify]
        center=left-F(1,10)
        axis_distance=F(3) if axis else F(3)+F(1,10)+width/2
        clear=axis_distance-F(1,10)-width/2
        distance=center+side*axis_distance
        pts=[point(x+nx*distance,y+ny*distance),point(x+8*ux+nx*distance,y+8*uy+ny*distance)]
        inp={'source':{'wall':'$uid:source','edge':0},'side':'left'if side==1 else'right',
             'measure':'axis'if axis else'clear','distance':3,
             'wall':{'layerId':'duvar','walls':[{'pts':[],'closed':False,'width':float(width)}]}}
        created={'kind':'wall','id':2,'layerId':'duvar','attrs':{},'pts':pts,'width':float(width),'justify':'center'}
        before={'ids':[1],'entities':{'1':source},'canUndo':False,'revision':'same'}
        steps=[{'op':'captureUid','id':1,'as':'source'}]
        if index%4==0:
            face_offset=left if side==1 else left-F(1,5)
            face=[point(x+nx*face_offset,y+ny*face_offset),point(x+8*ux+nx*face_offset,y+8*uy+ny*face_offset)]
            fr=point(x+4*ux+nx*face_offset,y+4*uy+ny*face_offset)
            to=point(F(fr['x'])+nx*side*clear,F(fr['y'])+ny*side*clear)
            planned={**created,'id':0}
            steps.append({'op':'plan','input':inp,'within':1e-8 if origin!=(0,0) else 1e-12,'result':{'status':'completed','output':{
                'wall':{'walls':[planned],'revision':'$current'},'sourceFace':face,'dimension':[fr,to],
                'clearDistance':float(clear),'axisDistance':float(axis_distance)},'warnings':[]},'expect':before})
        steps.append({'op':'execute','input':inp,'within':1e-8 if origin!=(0,0) else 1e-12,'result':{'status':'completed','output':{
            'uids':['$any'],'ids':[2],'revision':'$current'},'warnings':[]},'expect':{'ids':[1,2],
            'entities':{'1':source,'2':created},'canUndo':True,'revision':'changed'}})
        if index==0:
            steps.append({'op':'undo','returns':'Duvar','expect':{'ids':[1],'entities':{'1':source},'canUndo':False,'canRedo':True,'revision':'changed'}})
            steps.append({'op':'redo','returns':'Duvar','expect':{'ids':[1,2],'entities':{'1':source,'2':created},'revision':'changed'}})
        cases.append({'name':f'Rational {index}: {justify}, side {side}, axis {axis}, unit {setup["settings"]["drawingUnit"]}', 'setup':setup,'steps':steps})
    hidden=copy.deepcopy(cases[1]);hidden['name']='Hidden destination warns; source is preserved'
    hidden['steps'][1]['input']['wall']['layerId']='gizli'
    hidden['steps'][1]['result']['warnings']=[{'code':'layer_hidden','message':'“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.','path':'wall.layerId'}]
    hidden['steps'][1]['expect']['entities']['2']['layerId']='gizli';cases.append(hidden)
    locked=copy.deepcopy(cases[1]);locked['name']='Locked source is read, never edited'
    next(l for l in locked['setup']['layers'] if l['id']=='base')['locked']=True;cases.append(locked)
    validate=copy.deepcopy(cases[1]);validate['name']='Validate makes no entity, revision or undo'
    step=validate['steps'][1];step['op']='validate';step['result']['output']=None
    step['expect']={'ids':[1],'entities':{'1':validate['setup']['entities'][0]},'canUndo':False,'revision':'same'};cases.append(validate)
    base=copy.deepcopy(cases[0]);inp=copy.deepcopy(base['steps'][-3]['input'])
    for value in [0,-1]:
        bad=copy.deepcopy(inp);bad['distance']=value
        cases.append({'name':f'Nonpositive distance {value} is atomic','setup':base['setup'],'steps':[
            {'op':'captureUid','id':1,'as':'source'}, {'op':'execute','input':bad,
             'result':{'status':'failed','error':{'code':'invalid_wall','message':'Uzaklık sıfırdan büyük sonlu bir uzunluk olmalı.','path':'distance'}},
             'expect':{'ids':[1],'canUndo':False,'revision':'same'}}]})
    # Refusal codes and atomicity are independent assertions; inherited prose may evolve.
    for field,value,code,status in [
        ('uid','bad','invalid_uid','failed'),
        ('uid','01900000-0000-7000-8000-000000009999','entity_not_found','failed'),
        ('edge',99,'invalid_wall','failed'),
        ('layer','kilitli','layer_locked','failed'),
        ('layer','yok','layer_not_found','failed'),
        ('layer','grup','not_a_layer','failed'),
        ('weight',-1,'invalid_line_weight','failed'),
        ('style','yok-style','style_not_found','failed'),
        ('level','yok-level','level_not_found','failed'),
        ('revision','bad','invalid_revision','failed'),
        ('revision','999','revision_conflict','conflict'),
        ('finite',0,'not_finite','failed'),
        ('line',0,'not_a_wall','failed'),
    ]:
        bad=copy.deepcopy(inp);setup=copy.deepcopy(base['setup']);extra={}
        if field=='uid':bad['source']['wall']=value
        elif field=='edge':bad['source']['edge']=value
        elif field=='layer':bad['wall']['layerId']=value
        elif field=='weight':bad['wall']['lineWeight']=value
        elif field=='style':bad['wall']['walls'][0]['style']=value
        elif field=='level':bad['wall']['walls'][0]['arch']={'level':value}
        elif field=='revision':bad['wall']['expectedRevision']=value
        elif field=='finite':extra['nonFinite']={'wall.walls[0].width':'NaN'}
        elif field=='line':setup['entities']=[{'kind':'line','id':1,'layerId':'base','attrs':{},'a':point(0,0),'b':point(8,0)}]
        cases.append({'name':f'Atomic refusal {field}: {value}','setup':setup,'steps':[
            {'op':'captureUid','id':1,'as':'source'}, {'op':'execute','input':bad,**extra,
             'result':{'status':status,'error':{'code':code,'message':'$any','path':'$any', **({'revision':'$current'} if status=='conflict' else {})}},
             'expect':{'ids':[1],'canUndo':False,'revision':'same'}}]})
    return {'format':'kentos.command-cases','version':1,'command':'cad.walls.partition','commandVersion':1,
            'title':'Independent rational measured partitions','setup':cases[0]['setup'],'cases':cases}
def main():
    p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args()
    value=build();text=json.dumps(value,ensure_ascii=False,indent=2)+'\n'
    if a.check:
        if not OUT.exists()or OUT.read_text()!=text:raise SystemExit('Partition fixtures are stale')
    else:OUT.write_text(text)
    print(f'cad.walls.partition: {len(value["cases"])} independent cases')
if __name__=='__main__':main()
