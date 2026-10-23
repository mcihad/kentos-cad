"""Independent centre/diameter construction of annular hatches (ADR 0622 §9).

No KentOS code is loaded. Circles use the 60-digit Decimal Taylor reference
from shape_command_cases; exact boundaries are two semicircles. Checks,
undo/redo, plans and attributes run through both native and Python fixtures.
"""
from __future__ import annotations
import copy
import json
import sys
from pathlib import Path
from shape_command_cases import (SETUP, IDS, NEXT, UNCHANGED, P, PI, sin_, cos_,
                                 polygon_refusals, refused, not_finite_point, HIDDEN)
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/commands/v1/cad.donut.create.json'


def base(**changes):
    return {'layerId':'yapi', 'center':P(20,20), 'inner':2, 'outer':6, **changes}


def entity(inp, id_=NEXT):
    x, y = inp['center']['x'], inp['center']['y']
    # Stored hatch display rings follow the documented 5° boundary tessellation (72 segments).
    # Exact semicircle boundaries, rather than these samples, remain the measurement authority.
    def circle(d):
        # A canonical hatch derives its circle from the stored (rounded) semicircle endpoints.
        east, west = x + d / 2, x - d / 2
        center, r = (east + west) / 2, (east - west) / 2
        return [{'x':center + r*cos_(float(2*PI)*i/72), 'y':y + r*sin_(float(2*PI)*i/72)} for i in range(72)]
    def exact(d):
        return {'pts':[{'x':x+d/2,'y':y},{'x':x-d/2,'y':y}], 'bulges':[1,1]}
    e = {'kind':'hatch', 'id':id_, 'layerId':inp['layerId'], 'attrs':inp.get('attrs',{}),
         'ring':circle(inp['outer']), 'pattern':{'type':'solid','angle':0,'spacing':1},
         'boundary':[exact(inp['outer'])]}
    if inp['inner'] > 0:
        e['holes'] = [circle(inp['inner'])]
        e['boundary'].append(exact(inp['inner']))
    for k in ('color','label','symbol','lineWeight'):
        if k in inp: e[k] = inp[k]
    if 'lineType' in inp: e['lineType'] = inp['lineType']
    return e


def completed():
    return {'status':'completed', 'output':{'created':[f'$uidOf:{NEXT}'],'ids':[NEXT],'revision':'$current'},'warnings':[]}


def written(name, inp):
    done = completed()
    if inp['layerId']=='gizli':
        done['warnings']=[{'code':'layer_hidden','message':HIDDEN,'path':'layerId'}]
    return {'name':name,'steps':[
        {'op':'execute','input':inp,'result':done,'expect':{'ids':IDS+[NEXT],'entities':{str(NEXT):entity(inp)},'canUndo':True,'dirty':True}},
        {'op':'undo','returns':'Ekle','expect':{'ids':IDS,'canRedo':True,'canUndo':False}},
        {'op':'redo','returns':'Ekle','expect':{'ids':IDS+[NEXT],'entities':{str(NEXT):entity(inp)}}}]}


def cases():
    out = [written('Halka: kesin yay sınırı ve delik',base()),
           written('Disk: iç çap sıfır',base(inner=0)),
           written('Küçük halka',base(inner=0.002,outer=0.006)),
           written('Büyük halka',base(inner=20,outer=600)),
           written('Renk, öznitelik ve etiket',base(color='#E5484D',attrs={'Tür':'Kuyu'},label='H1')),
           written('Kalınlık ve çizgi tipi',base(lineWeight=0.5,lineType='dashed')),
           written('Gizli katman uyarısı',base(layerId='gizli')),
           {'name':'Plan yazmaz; halka geometriyle gelir','steps':[{'op':'plan','input':base(),
              'result':{'status':'completed','output':{'entities':[entity(base(),0)],'revision':'$current'},'warnings':[]},'expect':UNCHANGED}]},
           {'name':'Doğrulama yazmaz','steps':[{'op':'validate','input':base(),
              'result':{'status':'completed','output':None,'warnings':[]},'expect':UNCHANGED}]}]
    for name,inp,code,msg,path,nf in [
        ('Merkezin doğusu sonlu değil',base(),'not_finite',not_finite_point('Merkezin','x'),'center.x',{'center.x':'NaN'}),
        ('Merkezin kuzeyi sonlu değil',base(),'not_finite',not_finite_point('Merkezin','y'),'center.y',{'center.y':'Infinity'}),
        ('İç çap sonlu değil',base(),'not_finite','İç çap sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir çap verin.','inner',{'inner':'NaN'}),
        ('Dış çap sonlu değil',base(),'not_finite','Dış çap sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir çap verin.','outer',{'outer':'-Infinity'}),
        ('Eksi iç çap',base(inner=-1),'invalid_distance','İç çap sıfır ya da pozitif olmalı.','inner',None),
        ('Dış çap sıfır',base(inner=0,outer=0),'invalid_distance','Dış çap iç çaptan büyük olmalı.','outer',None),
        ('Eşit çaplar',base(inner=6),'invalid_distance','Dış çap iç çaptan büyük olmalı.','outer',None),
        ('Dış çap içten küçük',base(inner=8),'invalid_distance','Dış çap iç çaptan büyük olmalı.','outer',None),
    ]: out.append(refused(name,inp,code,msg,path,nf))
    borrowed = polygon_refusals(base())
    for c in borrowed:
        for st in c['steps']:
            if st.get('result',{}).get('status')=='completed': st['result']=completed()
    out.extend(borrowed)
    # Independent layer-field definitions keep the same base objects and test public input paths.
    for code, field, attrs, message, path in [
        ('invalid_attribute', {'name':'Durum','kind':'text','values':[{'code':'M','label':'Mevcut'}]},
         {'Durum':'X'}, '“Durum” alanı listedeki değerlerden birini ister; “X” listede yok.', 'attrs.Durum'),
        ('attribute_required', {'name':'Ada','kind':'text','required':True},
         {'Ada':' '}, '“Ada” alanı zorunlu; boş bırakılamaz.', 'attrs.Ada'),
    ]:
        setup=copy.deepcopy(SETUP)
        next(n for n in setup['layers'] if n['id']=='yapi')['fields']=[field]
        c=refused(f'Katman alan kuralı: {code}',base(attrs=attrs),code,message,path)
        c['setup']=setup
        out.append(c)
    assert len(out)>=20
    return out


def main():
    d={'format':'kentos.command-cases','version':1,'command':'cad.donut.create','commandVersion':1,
       'title':'Halka oluştur','note':__doc__,'setup':SETUP,'cases':cases()}
    text=json.dumps(d,ensure_ascii=False,indent=2)+'\n'
    if '--check' in sys.argv:
        if not OUT.exists() or OUT.read_text()!=text: raise SystemExit('Halka durumları güncel değil')
        print(f'Halka durumları güncel: {len(d["cases"])} durum')
    else:
        OUT.write_text(text);print(f'Halka: {len(d["cases"])} durum yazıldı')
if __name__=='__main__': main()
