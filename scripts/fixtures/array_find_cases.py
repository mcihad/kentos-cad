#!/usr/bin/env python3
"""Independent read-only line/grid array protocol cases. No KentOS runtime imports."""
import argparse
from copy import deepcopy
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def main():
    cli = argparse.ArgumentParser(description=__doc__)
    cli.add_argument("--check", action="store_true")
    args = cli.parse_args()
    setup = json.loads((ROOT / "fixtures/commands/v1/cad.arrays.adopt.json").read_text())["setup"]
    setup["entities"] = [dict(kind="insert", id=k+1, layerId="a", attrs={},
        block=setup["blocks"][0]["id"], p=dict(x=x,y=y),scale=1,rotation=0)
        for k,(x,y) in enumerate([(0,0),(2,0),(6,0),(8,0),(10,0),(14,0),(2.5,.5)])]
    setup["entities"].append(dict(kind="line",id=9,layerId="a",attrs={},a=dict(x=0,y=0),b=dict(x=20,y=0)))
    ids = [1,2,3,4,5,6,7,9]
    held = dict(ids=ids,canUndo=False,dirty=False,revision="same",arrays=[])
    uid = lambda i:f"$uidOf:{i}"
    items = [dict(uid=uid(i)) if i else {} for i in [1,2,None,3,4,5,None,6]]
    candidate = dict(adoption=dict(source=uid(1),layout=dict(kind="line",count=8,dx=2,dy=0),
        items=items,tolerance=.000001,expectedRevision="$any"),present=6,missing=2,deviation=0,overlaps=False)
    result = dict(matches=[candidate],eligible="$any",excluded="$any",truncated=False,comparisons="$any",revision="$any")
    cases=[]

    def success(name, input_, output, op="execute"):
        cases.append(dict(name=name,steps=[dict(op=op,input=input_,result=dict(status="completed",output=output,warnings=[]),expect=held)]))

    success("validate: yazısız kaynak ve kapsam denetimi",dict(source=uid(1)),None,"validate")
    for op in ["plan","execute"]:
        success(f"{op}: iki eksik yer, eşleşmeyen öğe ve aynı kimlikler",dict(source=uid(1)),result,op)
    success("iç kaynak sıra ucuna yeniden eşlenir",dict(source=uid(4)),result)
    success("ters kapsam ve yinelenen UUID sonucu değiştirmez",dict(source=uid(1),uids=[uid(i) for i in reversed(ids)]+[uid(1)]),result)
    empty=dict(result,matches=[])
    success("boş kapsam yalnız kaynağı arar",dict(source=uid(1),uids=[]),empty)
    success("iki nesne örüntü sayılmaz",dict(source=uid(1),uids=[uid(2)]),empty)
    success("bir adaylık sınır gerçek tek sonucu kısmi göstermez",dict(source=uid(1),limit=1),result)
    crossing=deepcopy(setup)
    crossing["entities"]=[dict(kind="insert",id=k+1,layerId="a",attrs={},
        block=setup["blocks"][0]["id"],p=dict(x=x,y=y),scale=1,rotation=0)
        for k,(x,y) in enumerate([(0,0),(2,0),(4,0),(0,3),(0,6)])]
    cases.append(dict(name="kesişen iki adaydan biriyle sınırlı arama kısmi uyarısı verir",setup=crossing,steps=[dict(
        op="execute",input=dict(source=uid(1),limit=1),
        result=dict(status="completed",output=dict(result,matches="$any",truncated=True),warnings=[dict(
            code="recognition_truncated",message="$any",path="uids")]),
        expect=dict(held,ids=[1,2,3,4,5]))]))

    success("line açık seçim geriye uyumlu aynı sonucu verir",dict(source=uid(1),kind="line"),result)
    success("grid seçimi doğrusal sırayı grid gibi sunmaz",dict(source=uid(1),kind="grid"),empty)
    def grid_case(name,coords,source,layout,slots):
        grid=deepcopy(setup)
        grid["entities"]=[dict(kind="insert",id=k+1,layerId="a",attrs={},
            block=setup["blocks"][0]["id"],p=dict(x=x,y=y),scale=1,rotation=0)
            for k,(x,y) in enumerate(coords)]
        n=sum(i is not None for i in slots)
        candidate=dict(adoption=dict(source=uid(slots[0]),layout=layout,
            items=[dict(uid=uid(i)) if i else {} for i in slots],tolerance=.000001,expectedRevision="$any"),
            present=n,missing=len(slots)-n,deviation=0,overlaps=False)
        expected=dict(result,matches=[candidate])
        cases.append(dict(name=name,setup=grid,steps=[dict(op="execute",input=dict(source=uid(source),kind="grid"),
            result=dict(status="completed",output=expected,warnings=[]),
            expect=dict(held,ids=list(range(1,len(coords)+1))))]))
    grid_case("grid iki boş yer ve eşleşmeyen blok ile yazısız önerilir",
        [(0,0),(2,0),(0,3),(2,3),(4,3),(0,6),(4,6),(1.3,.7)],4,
        dict(kind="grid",rows=3,cols=3,dx=2,dy=3),[1,2,None,3,4,5,6,None,7])
    grid_case("grid boş başlangıcı doldurmaz dolu köşeden ters sütunla başlar",
        [(2,0),(4,0),(0,3),(2,3),(4,3)],4,
        dict(kind="grid",rows=2,cols=3,dx=-2,dy=3),[2,1,None,5,4,3])

    def precision_case(name, coords, source, layout):
        raw=deepcopy(setup)
        raw["entities"]=[dict(kind="insert",id=k+1,layerId="a",attrs={},
            block=setup["blocks"][0]["id"],p=dict(x=x,y=y),scale=1,rotation=0)
            for k,(x,y) in enumerate(coords)]
        n=len(coords)
        expected=dict(result,matches=[dict(adoption=dict(source=uid(1),layout=layout,
            items=[dict(uid=uid(i)) for i in range(1,n+1)],tolerance=.000001,expectedRevision="$any"),
            present=n,missing=0,deviation="$any",overlaps=False)])
        input_=dict(source=uid(source))
        if layout["kind"]=="grid":input_["kind"]="grid"
        cases.append(dict(name=name,setup=raw,steps=[dict(op="execute",input=input_,
            result=dict(status="completed",output=expected,warnings=[]),
            expect=dict(held,ids=list(range(1,n+1))))]))
    precision_case("büyük koordinatta 3 mm sıra iç kaynaktan eksiksiz bulunur",
        [(4e9+k*.003,1e6) for k in range(11)],6,
        dict(kind="line",count=11,dx="$any",dy=0))
    precision_case("büyük koordinatta grid yuvarlanan tek aralığı çoğaltmaz",
        [(4e9+c*.03,4e9+r*.004) for r in range(3) for c in range(4)],6,
        dict(kind="grid",rows=3,cols=4,dx="$any",dy="$any"))

    def polar_case(name, count, missing, rotate, source=0, unmatched=False):
        raw=deepcopy(setup)
        slots=[k for k in range(count) if k not in missing]
        ids={k:i+1 for i,k in enumerate(slots)}
        raw["entities"]=[dict(kind="insert",id=ids[k],layerId="a",attrs={},
            block=setup["blocks"][0]["id"],p=dict(x=20+8*math.cos(2*math.pi*k/count),y=-15+8*math.sin(2*math.pi*k/count)),
            scale=1,rotation=.3+(2*math.pi*k/count if rotate else 0)) for k in slots]
        candidate=dict(adoption=dict(source=uid(ids[source]),layout=dict(kind="polar",center=dict(x="$any",y="$any"),count=count,fill=360,rotate=rotate),
            items=[dict(uid=uid(ids[k])) if k in ids else {} for k in [(source+j)%count for j in range(count)]],tolerance=.000001,expectedRevision="$any"),
            present=len(slots),missing=count-len(slots),deviation="$any",overlaps=False)
        if unmatched:
            raw["entities"].append(dict(kind="insert",id=len(slots)+1,layerId="a",attrs={},block=setup["blocks"][0]["id"],p=dict(x=22.5,y=-14.5),scale=1,rotation=.3))
        expected=dict(result,matches=[candidate])
        if len(slots)<4:expected=dict(result,matches=[])
        cases.append(dict(name=name,setup=raw,steps=[dict(op="execute",input=dict(source=uid(ids[source]),kind="polar"),
            result=dict(status="completed",output=expected,warnings=[]),expect=dict(held,ids=list(range(1,len(raw["entities"])+1))))]))
    polar_case("kutupsal: iki eksik yer ve yönü sabit öğeler",8,[2,6],False,unmatched=True)
    polar_case("kutupsal: iç kaynak ve merkez çevresinde dönen öğeler",8,[2,6],True,4)
    polar_case("kutupsal: yay verisi açık tam çember seçimiyle boş yerler bırakır",12,list(range(4,12)),False)
    polar_case("kutupsal: üç nokta niyet çıkarmak için yeterli değildir",4,[3],False)
    success("kutupsal seçimi doğrusal sırayı çember gibi sunmaz",dict(source=uid(1),kind="polar"),empty)

    path_candidate=deepcopy(candidate)
    path_candidate['adoption']['layout']=dict(kind='path',path=uid(9),count=8,spacing=2,align=False)
    # candidate above is the original straight row; the guide deliberately extends past the last root.
    success("yol: son dolu yerden sonrası varsayılmaz",dict(source=uid(1),kind='path',path=uid(9)),dict(result,matches=[path_candidate]))

    def path_case(name,align,closed=False,bulged=False):
        raw=deepcopy(setup)
        if bulged:
            count=8; spacing=math.pi/4; missing={2,6}
            points=[(2*math.cos(k*spacing/2),2*math.sin(k*spacing/2)) for k in range(count)]
            guide=dict(kind='arc',id=9,layerId='a',attrs={},c=dict(x=0,y=0),r=2,a0=0,a1=math.pi)
            angles=[k*spacing/2 for k in range(count)]
        elif closed:
            count=14;spacing=2;missing={2,6}
            points=[(k*2,0) if k<3 else (6,k*2-6) if k<7 else (20-k*2,8) if k<10 else (0,28-k*2) for k in range(count)]
            angles=[0 if k<3 else math.pi/2 if k<7 else math.pi if k<10 else -math.pi/2 for k in range(count)]
            guide=dict(kind='polyline',id=9,layerId='a',attrs={},pts=[dict(x=0,y=0),dict(x=6,y=0),dict(x=6,y=8),dict(x=0,y=8)],closed=True)
        else:
            count=8;spacing=2;missing={2,6}
            points=[(k*2,0) if k<3 else (6,k*2-6) for k in range(count)]
            angles=[0 if k<3 else math.pi/2 for k in range(count)]
            guide=dict(kind='polyline',id=9,layerId='a',attrs={},pts=[dict(x=0,y=0),dict(x=6,y=0),dict(x=6,y=8)])
        occupied=[k for k in range(count) if k not in missing]; ids={k:j+1 for j,k in enumerate(occupied)}
        guide['id']=len(occupied)+2
        raw['entities']=[dict(kind='insert',id=ids[k],layerId='a',attrs={},block=setup['blocks'][0]['id'],
            p=dict(x=points[k][0],y=points[k][1]),scale=1,rotation=.2+(angles[k] if align else 0)) for k in occupied]
        raw['entities'].append(dict(kind='insert',id=len(occupied)+1,layerId='a',attrs={},block=setup['blocks'][0]['id'],p=dict(x=7,y=3),scale=1,rotation=.2))
        raw['entities'].append(guide)
        expected=dict(result,matches=[dict(adoption=dict(source=uid(1),layout=dict(kind='path',path=uid(guide['id']),count=count,spacing='$any',align=align),
            items=[dict(uid=uid(ids[k])) if k in ids else {} for k in range(count)],tolerance=.000001,expectedRevision='$any'),present=len(occupied),missing=2,deviation='$any',overlaps=False)])
        cases.append(dict(name=name,setup=raw,steps=[dict(op='execute',input=dict(source=uid(1),kind='path',path=uid(guide['id'])),
            result=dict(status='completed',output=expected,warnings=[]),expect=dict(held,ids=list(range(1,len(raw['entities'])+1))))]))
    path_case('yol: köşeli kılavuz ve yönü sabit öğeler',False)
    path_case('yol: köşede teğet hizalama ve iki boş yer',True)
    path_case('yol: kapalı kılavuzun dördüncü kenarı ve yön',True,closed=True)
    path_case('yol: analitik yay ve dönen öğeler',True,bulged=True)

    def refuse(name, input_, code, path, conflict=False, raw=None, numbers=None):
        err=dict(code=code,message="$any",path=path)
        if conflict:err["revision"]="$any"
        step=dict(op="execute",input=input_,result=dict(status="conflict" if conflict else "failed",error=err),expect=held)
        if numbers:step["nonFinite"]=numbers
        case=dict(name=name,steps=[step])
        if raw:case["setup"]=raw
        cases.append(case)

    refuse("geçersiz kaynak UUID",dict(source="bad"),"invalid_uid","source")
    refuse("yol kılavuzu zorunludur",dict(source=uid(1),kind='path'),'invalid_path','path')
    refuse("kılavuz başka düzende sessizce yok sayılmaz",dict(source=uid(1),path=uid(9)),'invalid_path','path')
    refuse("blok kılavuz sayılamaz",dict(source=uid(1),kind='path',path=uid(2)),'invalid_path','path')
    refuse("kayıtlı başlangıçta olmayan kaynak taşınmaz",dict(source=uid(2),kind='path',path=uid(9)),'recognition_unsupported','source')
    refuse("olmayan kaynak",dict(source="01925f3e-7c1a-7d2b-9e4f-000000000999"),"entity_not_found","source")
    refuse("kaynak blok değil",dict(source=uid(9)),"array_adoption_refused","source")
    refuse("bozuk kapsam kimliği",dict(source=uid(1),uids=["bad"]),"invalid_uid","uids[0]")
    refuse("olmayan kapsam nesnesi",dict(source=uid(1),uids=["01925f3e-7c1a-7d2b-9e4f-000000000999"]),"entity_not_found","uids[0]")
    refuse("kapsam sınırı",dict(source=uid(1),uids=[uid(1)]*2049),"recognition_scope_limit","uids")
    for value in [0,-1,.00101]:
        refuse(f"geçersiz tolerans {value}",dict(source=uid(1),tolerance=value),"invalid_tolerance","tolerance")
    for value in ["NaN","Infinity","-Infinity"]:
        refuse(f"sonlu olmayan tolerans {value}",dict(source=uid(1),tolerance=1),"invalid_tolerance","tolerance",numbers={"tolerance":value})
    for value in [0,101]:
        refuse(f"geçersiz aday sınırı {value}",dict(source=uid(1),limit=value),"invalid_limit","limit")
    refuse("bozuk revizyon",dict(source=uid(1),expectedRevision="bad"),"invalid_revision","expectedRevision")
    refuse("eski revizyon",dict(source=uid(1),expectedRevision="999"),"revision_conflict","expectedRevision",conflict=True)
    locked=deepcopy(setup);locked["layers"][0]["locked"]=True
    refuse("kilitli kaynak",dict(source=uid(1)),"layer_locked","source",raw=locked)
    data=deepcopy(setup);data["entities"][0]["attrs"]={"CODE":"A"}
    refuse("kaynak verisi atlanamaz",dict(source=uid(1)),"array_adoption_refused","source",raw=data)
    result=dict(format="kentos.command-cases",version=1,command="cad.arrays.find",commandVersion=1,
        title="Blok dizi örüntülerini bul",note="ADR0874. Independent expectations; geometry, ambiguity, cancellation and adoption roundtrip in native acceptance.",
        setup=setup,cases=cases)
    target=ROOT/"fixtures/commands/v1/cad.arrays.find.json"
    text=json.dumps(result,ensure_ascii=False,indent=2)+"\n"
    if args.check:
        if target.read_text()!=text:raise SystemExit("array find cases are stale")
    else:target.write_text(text)
    print(f"cad.arrays.find: {len(cases)} independent cases")


if __name__=="__main__":main()
