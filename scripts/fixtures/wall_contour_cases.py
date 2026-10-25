#!/usr/bin/env python3
"""Opt-in contour cases: independent winding, exact circles and old wall reference.
No KentOS import. Every case has its own setup; the old conversion cases stay unchanged.
"""
import copy
import math
from wall_command_cases import SETUP, P, base, build, is_refusal, refusal, uid_of

def orient(r, ccw):
    r=copy.deepcopy(r)
    pts=r['pts']; o=pts[0]
    area=sum((a['x']-o['x'])*(b['y']-o['y'])-(a['y']-o['y'])*(b['x']-o['x']) for a,b in zip(pts,pts[1:]+pts[:1]))/2
    if 'bulges' in r: # the two exact semicircles used here
        area=math.pi*9*(1 if r['bulges'][0]>0 else -1)
    if (area>0)!=ccw:
        n=len(pts)
        r['pts']=[pts[0]]+list(reversed(pts[1:]))
        if 'bulges' in r:r['bulges']=[-r['bulges'][(n-1-i)%n] for i in range(n)]
    return r

def planned(name, source, justify='center', keep=True, width=.2, extra=None):
    setup=copy.deepcopy(SETUP);setup['entities']=[source];setup['settings']['drawingUnit']='m'
    inp={'uids':[uid_of(1)],'mode':'contours','width':width,'justify':justify,'keepSources':keep}
    inp.update(extra or {})
    if source['kind']=='circle':
        c=source['c'];r=source['r']
        rings=[{'pts':[P(c['x']+r,c['y']),P(c['x']-r,c['y'])],'bulges':[1.,1.]}]
    else:
        rings=[orient({k:copy.deepcopy(source[k]) for k in ['pts','bulges'] if k in source},True)]
        rings += [orient(h,False) for h in source.get('holes',[])]
    walls=[]
    for r in rings:
        w=build(setup['settings'],dict(r,closed=True,width=width,justify=justify),'uids[0]',[])
        if is_refusal(w):raise AssertionError(w)
        walls.append(dict(kind='wall',**{k:copy.deepcopy(v)for k,v in source.items()if k in ['layerId','attrs','color','lineWeight','label']},id=0,**w))
    out={'walls':walls,'revision':'$current'}
    if not keep:out['removed']=[uid_of(1)]
    before={'ids':[1],'canUndo':False,'canRedo':False,'dirty':False,'revision':'same'}
    return {'name':name,'setup':setup,'steps':[{'op':'plan','input':inp,'result':{'status':'completed','output':out,'warnings':[]},'expect':before,'within':1e-10}]}

def cases():
    result=[]
    shapes=[('L oda',[P(0,0),P(6,0),P(6,2),P(2,2),P(2,5),P(0,5)],[]),
            ('Avlulu oda',[P(0,0),P(10,0),P(10,8),P(0,8)],[{'pts':[P(4,3),P(6,3),P(6,5),P(4,5)]}])]
    for title,pts,holes in shapes:
        for reverse in [False,True]:
            source={'kind':'polygon',**base(1,'cizim',color='#123456',lineWeight=.35,label='Oda konturu'),'pts':copy.deepcopy(pts)}
            source['attrs']={'office':'A'}
            if holes:source['holes']=copy.deepcopy(holes)
            if reverse:
                source['pts'][1:]=reversed(source['pts'][1:])
                for h in source.get('holes',[]):h['pts'][1:]=reversed(h['pts'][1:])
                # Independent large-coordinate input, output remains true-size SI.
                for r in [source]+source.get('holes',[]):
                    for p in r['pts']:p['x']+=500000;p['y']+=4400000
            for reference in ['left','right','center']:
                for keep in [False,True]:
                    result.append(planned(f'Kapalı kontur: {title}, ters={reverse}, referans={reference}, kaynak koru={keep}',source,reference,keep))
    for reference in ['left','right','center']:
        source={'kind':'circle',**base(1,'cizim'),'c':P(0,0),'r':3.}
        result.append(planned(f'Kapalı kontur: tam daire iki analitik yay, {reference}',source,reference))
    poly={'kind':'polyline',**base(1,'cizim'),'pts':[P(0,0),P(4,0),P(4,3),P(0,3)],'closed':True}
    result.append(planned('Kapalı kontur: kapalı çoklu çizgi ve kaynak silme planı',poly,'left',False))
    for title, source, message in [
        ('açık çoklu çizgi',dict(poly,closed=False),'Kapalı kontur için kapalı çoklu çizgi, alan veya daire verin; açık ya da başka türde kaynak varsa hiçbir duvar yazılmaz.'),
        ('sıfır alan',dict(poly,kind='polygon',pts=[P(0,0),P(1,0),P(2,0)]),'Konturun sonlu ve sıfırdan farklı bir alanı olmalı.'),
        ('dışarıdaki delik',dict(poly,kind='polygon',holes=[{'pts':[P(10,10),P(12,10),P(12,12),P(10,12)]}]),'Kontur duvara çevrilemedi: Delik dış halkanın dışına taşıyor. Kaynağı düzeltin; hiçbir duvar yazılmadı.'),
    ]:
        setup=copy.deepcopy(SETUP);setup['entities']=[source]
        result.append({'name':f'Kapalı kontur: {title} atomik reddedilir','setup':setup,'steps':[{'op':'execute','input':{'uids':[uid_of(1)],'mode':'contours'},'result':refusal('invalid_wall',message,'uids[0]'),'expect':{'ids':[1],'canUndo':False,'canRedo':False,'dirty':False,'revision':'same'}}]})
    return result
