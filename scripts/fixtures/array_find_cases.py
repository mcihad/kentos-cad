#!/usr/bin/env python3
"""Independent read-only linear-array protocol cases. No KentOS runtime imports."""
import argparse
from copy import deepcopy
import json
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

    def refuse(name, input_, code, path, conflict=False, raw=None, numbers=None):
        err=dict(code=code,message="$any",path=path)
        if conflict:err["revision"]="$any"
        step=dict(op="execute",input=input_,result=dict(status="conflict" if conflict else "failed",error=err),expect=held)
        if numbers:step["nonFinite"]=numbers
        case=dict(name=name,steps=[step])
        if raw:case["setup"]=raw
        cases.append(case)

    refuse("geçersiz kaynak UUID",dict(source="bad"),"invalid_uid","source")
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
        title="Doğrusal blok dizilerini bul",note="ADR0874. Independent expectations; geometry, ambiguity, cancellation and adoption roundtrip in native acceptance.",
        setup=setup,cases=cases)
    target=ROOT/"fixtures/commands/v1/cad.arrays.find.json"
    text=json.dumps(result,ensure_ascii=False,indent=2)+"\n"
    if args.check:
        if target.read_text()!=text:raise SystemExit("array find cases are stale")
    else:target.write_text(text)
    print(f"cad.arrays.find: {len(cases)} independent cases")


if __name__=="__main__":main()
