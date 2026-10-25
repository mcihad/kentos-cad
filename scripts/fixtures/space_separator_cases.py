#!/usr/bin/env python3
"""ADR 0650 independent virtual partitions: Decimal rectangles, no KentOS code.
A 10x8 metre wall-axis rectangle of .2 m thickness and a 2x2 m column.
A zero-thickness separator at x=5 removes no material. For offset d, the
left width is 5+d/2, the height 8+d; the column removes exactly 4 m².
Only setup metadata is borrowed from the other independent reference.
"""
from copy import deepcopy
from decimal import Decimal as D
from pathlib import Path
import argparse,json
from space_scope_cases import setup,LABEL,DELTA
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'fixtures/commands/v1/cad.spaces.separators.json'
def completed(output):return dict(status='completed',output=output,warnings=[])
def case(unit,method,far,reverse):
    doc,seed=setup(unit,far);doc['name']='Bağımsız sıfır kalınlıklı mahal ayırıcı'
    doc['entities']=[doc['entities'][0],dict(doc['entities'][2],id=2)]
    x,y=(D(500000),D(4400000)) if far else (D(0),D(0))
    p=lambda a,b:dict(x=float(x+D(a)),y=float(y+D(b)))
    pts=[p(5,0),p(5,8)]
    if reverse:pts.reverse()
    args=dict(operation=dict(kind='create',layerId='cizim',paths=[dict(pts=pts)]))
    base=dict(kind='polyline',id=4,layerId='cizim',attrs={},lineType='dashed',pts=pts)
    role=dict(base,spaceSeparator={})
    room=dict(kind='space',id=3,layerId='mahal',attrs={},parts='$any',seed=seed,columns='all')
    if method!='net':room['method']=method
    def state(with_role):
        return dict(ids=[1,2,3,4],entities={'3':room,'4':role if with_role else base},canUndo=True,dirty=True)
    def quantities(half):
        width=D(5 if half else 10)
        by={m:float((width+d/(2 if half else 1))*(D(8)+d)-4) for m,d in DELTA.items()}
        d=DELTA[method]
        row=dict(uid='$uidOf:3',method=method,columns='all',area=by[method],
            perimeter=float(2*(width+d/(2 if half else 1)+D(8)+d)+8),byMethod=by,
            rule=f'alan: halkaların alanı, delikler düşülür ({LABEL[method]}); hacim: alan × yükseklik; kolon alanı: Her yöntemde')
        return dict(op='execute',command='cad.spaces.quantities',input={},within=1e-6,result=completed(dict(spaces=[row],walls='$any',openings=[],totals='$any')),expect=dict(revision='same'))
    changed=completed(dict(uids=['$any'],ids=[4],revision='$current'))
    return dict(name=f'{unit}/{method}/{"far" if far else "local"}/{"reverse" if reverse else "forward"}',setup=doc,steps=[
        dict(op='execute',command='cad.spaces.create',input=dict(layerId='mahal',spaces=[dict(seed=seed,method=method,columns='all')]),result=completed(dict(uids=['$any'],ids=[3],revision='$current')),expect=dict(ids=[1,2,3],entities={'3':room})),
        quantities(False),
        dict(op='plan',input=args,result=completed(dict(entities=[dict(role,id=0)],revision='$current')),expect=dict(ids=[1,2,3],revision='same')),
        dict(op='execute',input=args,result=changed,expect=dict(state(True),revision='changed')),
        quantities(True),
        dict(op='undo',returns='Mahal ayırıcı',expect=dict(ids=[1,2,3],entities={'3':room},revision='changed')),
        quantities(False),
        dict(op='redo',returns='Mahal ayırıcı',expect=dict(state(True),revision='changed')),
        quantities(True),
        dict(op='plan',input=dict(operation=dict(kind='clear',uids=['$uidOf:4'])),result=completed(dict(entities=[base],revision='$current')),expect=dict(state(True),revision='same')),
        dict(op='execute',input=dict(operation=dict(kind='clear',uids=['$uidOf:4'])),result=completed(dict(uids=['$uidOf:4'],ids=[4],revision='$current')),expect=dict(state(False),revision='changed')),
        quantities(False),
        dict(op='execute',input=dict(operation=dict(kind='set',uids=['$uidOf:4'])),result=completed(dict(uids=['$uidOf:4'],ids=[4],revision='$current')),expect=dict(state(True),revision='changed')),
        quantities(True),
        dict(op='execute',input=dict(operation=dict(kind='set',uids=['$uidOf:4'])),result=completed(dict(uids=['$uidOf:4'],ids=[4],revision='$current')),expect=dict(state(True),revision='same')),
        dict(op='undo',returns='Mahal ayırıcı',expect=dict(state(False),revision='changed')),
        quantities(False),
    ])
def refusal_cases():
    doc,_=setup('m',False);doc['entities']=[dict(doc['entities'][0],justify='center'),dict(doc['entities'][2],id=2)]
    path=dict(pts=[dict(x=5.,y=0.),dict(x=5.,y=8.)])
    basic=dict(operation=dict(kind='create',layerId='cizim',paths=[path]))
    out=[]
    def refused(name,args,code,field,**extra):
        status='conflict' if code=='revision_conflict' else 'failed'
        step=dict(op='execute',input=args,result=dict(status=status,error=dict(code=code,message='$any',path=field)),
            expect=dict(ids=[1,2],entities={'1':doc['entities'][0],'2':doc['entities'][1]},canUndo=False,canRedo=False,dirty=False,revision='same'))
        if status=='conflict':step['result']['error']['revision']='$current'
        step.update(extra);out.append(dict(name=name,setup=deepcopy(doc),steps=[step]))
    def altered(**kwargs):
        p=deepcopy(basic);p['operation'].update(kwargs);return p
    def changed_path(**kwargs):return altered(paths=[dict(path,**kwargs)])
    refused('Boş ayırıcı listesi',altered(paths=[]),'invalid_space_separator','operation.paths')
    refused('Tek nokta',changed_path(pts=[dict(x=0.,y=0.)]),'too_few_points','operation.paths[0].pts')
    refused('Çakışık uçlar',changed_path(pts=[dict(x=1.,y=2.)]*2),'invalid_space_separator','operation.paths[0]')
    refused('Yay sayısı',changed_path(bulges=[0.,0.]),'bulge_count','operation.paths[0].bulges')
    refused('Sonlu olmayan nokta',basic,'not_finite','operation.paths[0].pts[0].x',nonFinite={'operation.paths[0].pts[0].x':'NaN'})
    refused('Sonlu olmayan yay',changed_path(bulges=[0.]),'not_finite','operation.paths[0].bulges[0]',nonFinite={'operation.paths[0].bulges[0]':'Infinity'})
    refused('Geçersiz çizgi kalınlığı',altered(lineWeight=-1.),'invalid_line_weight','operation.lineWeight')
    refused('Olmayan kat',changed_path(arch=dict(level='yok')),'level_not_found','operation.paths[0].arch.level')
    for layer,code in [('yok','layer_not_found'),('pafta','not_a_layer'),('kilitli','layer_locked')]:
        refused('Hedef katman '+code,altered(layerId=layer),code,'operation.layerId')
    refused('Geçersiz sürüm',dict(basic,expectedRevision='01'),'invalid_revision','expectedRevision')
    refused('Eski sürüm',dict(basic,expectedRevision='9999'),'revision_conflict','expectedRevision')
    for kind in ['set','clear']:
        refused(kind+' boş seçim',dict(operation=dict(kind=kind,uids=[])),'no_entities','operation.uids')
        refused(kind+' geçersiz kimlik',dict(operation=dict(kind=kind,uids=['bad'])),'invalid_uid','operation.uids[0]')
        refused(kind+' kayıp nesne',dict(operation=dict(kind=kind,uids=['00000000-0000-0000-0000-000000000001'])),'entity_not_found','operation.uids[0]')
        refused(kind+' kolon reddi',dict(operation=dict(kind=kind,uids=['$uidOf:2'])),'invalid_space_separator','operation.uids[0]')
    role=dict(kind='polyline',id=3,layerId='cizim',attrs={},spaceSeparator={},pts=deepcopy(path['pts']))
    guarded=deepcopy(doc);guarded['entities'].append(role)
    expected=dict(ids=[1,2,3],entities={'1':doc['entities'][0],'2':doc['entities'][1],'3':role},canUndo=False,canRedo=False,dirty=False,revision='same')
    steps=[]
    for command,args,field in [
        ('cad.entities.edit',dict(operation='properties',changes=[dict(kind='update',uid='$uidOf:3',geometry=dict(kind='polyline',pts=path['pts'],widths=[[1.,1.],[0.,0.]]))]),'changes'),
        # With a finite positive factor this small, binary64 addition rounds
        # both endpoints to the centre; a semantic boundary must not collapse.
        ('cad.entities.transform',dict(uids=['$uidOf:3'],transform=dict(kind='scale',center=dict(x=100.,y=100.),factor=1e-30)),'transform'),
    ]:
        steps.append(dict(op='execute',command=command,input=args,
            result=dict(status='failed',error=dict(code='invalid_space_separator',message='$any',path=field)),expect=deepcopy(expected)))
    out.append(dict(name='Genel geometri düzenlemesi rolü veya sınırı kaybedemez',setup=guarded,steps=steps))
    hidden=altered(layerId='gizli')
    planned=dict(kind='polyline',id=0,layerId='gizli',attrs={},lineType='dashed',spaceSeparator={},pts=deepcopy(path['pts']))
    out.append(dict(name='Gizli hedef katmanı açık uyarı verir',setup=deepcopy(doc),steps=[dict(op='plan',input=hidden,
        result=dict(status='completed',output=dict(entities=[planned],revision='$current'),warnings=[dict(code='layer_hidden',message='$any',path='operation.layerId')]),expect=dict(ids=[1,2],revision='same'))]))
    # An existing semantic tool must report its role constraint before execution.
    guarded=deepcopy(doc)
    constrained=dict(kind='polyline',id=3,layerId='cizim',attrs={},spaceSeparator={},pts=[dict(x=0.,y=0.),dict(x=4.,y=0.),dict(x=4.,y=4.)])
    guarded['entities'].append(constrained)
    expected=dict(ids=[1,2,3],entities={'3':constrained},canUndo=False,canRedo=False,dirty=False,revision='same')
    for command,args,code,path_field in [
        ('cad.curves.edit',dict(uid='$uidOf:3',edit=dict(kind='toSpline',degree=2,through=False)),'edit_not_applicable','edit'),
        ('cad.entities.vertex',dict(uid='$uidOf:3',action='width',place=dict(kind='mid',part=0,ring=0,index=0),widths=[1.,1.]),'invalid_polyline','uid'),
    ]:
        out.append(dict(name=f'{command}: ayırıcı kısıtı validate/plan/execute aynı ret',setup=deepcopy(guarded),steps=[dict(op=op,command=command,input=args,result=dict(status='failed',error=dict(code=code,message='$any',path=path_field)),expect=deepcopy(expected))for op in ('validate','plan','execute')]))
    return out

def stale_boundary_cases():
    """Independent 4×3 rectangle: zero-width = 12; .2 wall net = 10.64.
    Deleting the sole source retains that boundary; an old area must be marked.
    No column/scope/source opt-in is present, including the legacy wall case.
    """
    out=[]
    reason='Mahali duvarlar artık kapatmıyor; sınırı son hâliyle kaldı. Duvarları kapatın ya da mahali yeniden bulun (Mahali düzenle › Duvarlardan bul).'
    rule='alan: halkaların alanı, delikler düşülür (Net (iç yüzler)); hacim: alan × yükseklik'
    for unit in ['mm','cm','m','in','ft']:
        for far in [False,True]:
            for virtual in [False,True]:
                doc,_=setup(unit,far);x,y=(D(500000),D(4400000))if far else(D(0),D(0))
                pt=lambda a,b:dict(x=float(x+D(a)),y=float(y+D(b)))
                source=dict(kind='polyline'if virtual else'wall',id=1,layerId='cizim',attrs={},closed=True,pts=[pt(0,0),pt(4,0),pt(4,3),pt(0,3)])
                source.update(dict(spaceSeparator={})if virtual else dict(width=.2,justify='center'))
                doc['entities']=[source]
                by={method:float((D(4)+(D(0)if virtual else delta))*(D(3)+(D(0)if virtual else delta)))for method,delta in DELTA.items()}
                perimeter=float(2*((D(4)+(D(0)if virtual else DELTA['net']))+(D(3)+(D(0)if virtual else DELTA['net']))))
                def query(stale):
                    row=dict(uid='$uidOf:2',method='net',area=by['net'],perimeter=perimeter,byMethod={}if stale else by,rule=rule+('; son geçerli sınır: '+reason if stale else''))
                    if stale:row['boundaryBreak']='lostEnclosure'
                    return dict(op='execute',command='cad.spaces.quantities',input={},within=1e-6,result=completed(dict(spaces=[row],openings=[],walls='$any',totals='$any')),expect=dict(revision='same'))
                out.append(dict(name=f'Kopuk sınır raporu/{unit}/{"far"if far else"local"}/{"virtual"if virtual else"legacy-wall"}',setup=doc,steps=[
                    dict(op='execute',command='cad.spaces.create',input=dict(layerId='mahal',spaces=[dict(seed=pt(1,1))]),result=completed(dict(ids=[2],uids=['$any'],revision='$current')),expect=dict(ids=[1,2])),
                    query(False),
                    dict(op='execute',command='cad.entities.delete',input=dict(uids=['$uidOf:1']),result=completed('$any'),expect=dict(ids=[2])),
                    query(True),
                    dict(op='undo',expect=dict(ids=[1,2],entities={'1':source})),
                    query(False),
                ]))
    return out

def document():
    cases=[case(u,m,f,r) for u in ['mm','cm','m','in','ft'] for m in DELTA for f in [False,True] for r in [False,True]]
    cases.extend(refusal_cases())
    cases.extend(stale_boundary_cases())
    return dict(format='kentos.command-cases',version=1,command='cad.spaces.separators',commandVersion=1,
        title='Sıfır kalınlıklı mahal ayırıcı: alan korunumu, üç yöntem/beş birim/iki başlangıç/ters yön, rol/undo',
        note=__doc__.strip(),setup=deepcopy(cases[0]['setup']),cases=cases)
if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('--check',action='store_true');args=parser.parse_args()
    data=json.dumps(document(),ensure_ascii=False,indent=2)+'\n'
    if args.check:assert OUT.read_text()==data
    else:OUT.write_text(data)
    print(len(document()['cases']),'independent virtual room separator cases')
