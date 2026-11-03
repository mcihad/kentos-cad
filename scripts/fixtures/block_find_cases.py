#!/usr/bin/env python3
"""Independent CAD-ADV-07 command expectations: translated lines, strict data and read-only refusals."""
from pathlib import Path
import argparse
import json
from copy import deepcopy

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Check reproducibility without writing fixtures")
    cli = parser.parse_args()
    def uid(n):
        return f"$uidOf:{n}"

    entities = [dict(kind="line", id=i, layerId="a", attrs=({"TYPE": "B"} if i == 3 else {}),
                     a=dict(x=x, y=0), b=dict(x=x+2, y=0)) for i, x in enumerate([0, 10, 20], 1)]
    entities.append(dict(kind="point", id=4, layerId="a", attrs={}, p=dict(x=30, y=0)))
    for ident, x, reverse, closed, elevation in [(5,40,False,True,False),(6,50,True,True,False),
                                                 (7,60,False,False,False),(8,70,False,True,True)]:
        pts=[dict(x=x,y=0),dict(x=x+2,y=0),dict(x=x+1.5,y=1)]
        if reverse:
            pts=[pts[1],pts[0],pts[2]]
        entity=dict(kind="polyline",id=ident,layerId="a",attrs={},pts=pts,closed=closed)
        if elevation:
            entity['zs']=[0.,0.,1.]
        entities.append(entity)
    setup = dict(format="kentos.document", version=1, name="Tekrar arama", settings=dict(
        srid=0, lengthDecimals=3, areaDecimals=2, areaUnit="m2", angleUnit="deg", plotScale=50, workspace="cad"),
        origin=dict(x=0, y=0), layers=[dict(id="a", name="A", type="layer", visible=True, locked=False,
        expanded=True, style=dict(color="fg", lineType="continuous", lineWeight=0.18), children=[])],
        activeLayer="a", entities=entities, styles=dict(items=[], categories=[]))

    def output(numbers, truncated=False):
        return dict(source=[uid(1)], matches=[dict(members=[uid(i)], origin=dict(x=0, y=0),
            at=dict(x=(i-1)*10, y=0), rotation=0, scale=1, mirror=False,
            bounds=dict(minX=(i-1)*10, minY=0, maxX=(i-1)*10+2, maxY=0), overlaps=False) for i in numbers],
            truncated=truncated, comparisons="$any", related={}, revision="$any")

    cases = []

    def case(name, args, result, op="execute"):
        cases.append(dict(name=name, steps=[dict(op=op, input=args, result=result,
            expect=dict(ids=list(range(1,9)), canUndo=False, dirty=False, revision="same"))]))

    for op in ["plan", "execute"]:
        case(f"{op}: tam geometri ve nesne verisi, kaynak değişmez", dict(source=[uid(1)]),
             dict(status="completed", output=output([1, 2]), warnings=[]), op)
    for name, args in [
        ("yinelenen kaynak kimliği bir kez alınır", dict(source=[uid(1), uid(1)])),
        ("yalnız öteleme araması", dict(source=[uid(1)], rotations=False)),
        ("aynalama ve ölçek varsayılanı açıkça kapalı", dict(source=[uid(1)], mirrors=False, scales=False)),
        ("mutlak tolerans geometriyi değiştirmez", dict(source=[uid(1)], tolerance=0.0005)),
    ]:
        case(name, args, dict(status="completed", output=output([1, 2]), warnings=[]))
    case("boş kapsam bütün çizim anlamına gelmez", dict(source=[uid(1)], uids=[]),
         dict(status="completed", output=output([]), warnings=[]))
    case("kaynak kapsamın dışında olsa da yalnız hedef aranır", dict(source=[uid(1)], uids=[uid(2)]),
         dict(status="completed", output=output([2]), warnings=[]))
    case("görünüş ve veriyi yok sayma açık ve raporlu", dict(source=[uid(1)], geometryOnly=True),
         dict(status="completed", output=output([1, 2, 3]), warnings=[dict(code="recognition_geometry_only",
         message="Yalnız geometri karşılaştırıldı; katman, görünüş ve öznitelik farkları eşleşmeye engel sayılmadı.", path="geometryOnly")]))
    case("sonuç sınırı eksik taramayı saklamaz", dict(source=[uid(1)], limit=1),
         dict(status="completed", output=output([1], True), warnings=[dict(code="recognition_truncated",
         message="Arama sınıra ulaştı; bulunanlar kısmi sonuçtur. Kapsamı daraltıp yeniden arayın.", path="uids")]))

    def refusal(name, args, code, path, status="failed", revision=False):
        error = dict(code=code, message="$any", path=path)
        if revision:
            error["revision"] = "$any"
        case(name, args, dict(status=status, error=error))

    refusal("boş kaynak", dict(source=[]), "no_entities", "source")
    refusal("büyük kaynak seçimden önce reddedilir", dict(source=[uid(1)]*129), "recognition_source_limit", "source")
    refusal("desteklenmeyen kaynak öğesi atlanmaz", dict(source=[uid(4)]), "recognition_unsupported", "source[0]")
    refusal("bozuk kaynak kimliği", dict(source=["bad"]), "invalid_uid", "source[0]")
    refusal("bozuk kapsam kimliği", dict(source=[uid(1)], uids=["bad"]), "invalid_uid", "uids[0]")
    refusal("olmayan kaynak", dict(source=["01925f3e-7c1a-7d2b-9e4f-000000000999"]), "entity_not_found", "source[0]")
    refusal("sıfır tolerans", dict(source=[uid(1)], tolerance=0), "invalid_tolerance", "tolerance")
    refusal("büyük tolerans", dict(source=[uid(1)], tolerance=1), "invalid_tolerance", "tolerance")
    refusal("sıfır sonuç sınırı", dict(source=[uid(1)], limit=0), "invalid_limit", "limit")
    refusal("bozuk revizyon", dict(source=[uid(1)], expectedRevision="bad"), "invalid_revision", "expectedRevision")
    refusal("eski revizyon", dict(source=[uid(1)], expectedRevision="999"), "revision_conflict", "expectedRevision", "conflict", True)
    for op in ['plan','execute']:
        result=dict(source=[uid(5)],matches=[dict(members=[uid(i)],origin=dict(x=40,y=0),at=dict(x=x,y=0),
            rotation=0,scale=1,mirror=False,bounds=dict(minX=x,minY=0,maxX=x+2,maxY=1),overlaps=False)
            for i,x in [(5,40),(6,50)]],truncated=False,comparisons='$any',related={},revision='$any')
        case(f'{op}: kapalı çoklu çizginin başlangıcı ve yönü farklı; açık ve Z verili benzerler eş değil',
            dict(source=[uid(5)]),dict(status='completed',output=result,warnings=[]),op)
    refusal('Z verili kaynak geometri seçeneğinde de atlanmaz',dict(source=[uid(8)],geometryOnly=True),
            'recognition_unsupported','source[0]')
    refusal('karma kaynakta Z verili ikinci üye bütün sorguyu reddettirir',dict(source=[uid(1),uid(8)]),
            'recognition_unsupported','source[1]')
    def semantic_case(name,raw,source,numbers,**flags):
        matches=[dict(members=[uid(i)],origin=raw['entities'][source-1]['p'],at=raw['entities'][i-1]['p'],rotation=0,scale=1,mirror=False,bounds='$any',overlaps=False) for i in numbers]
        warnings=[dict(code='recognition_geometry_only',message='$any',path=path) for key,path in [('textVariants','textVariants'),('attributeVariants','attributeVariants')] if flags.get(key)]
        cases.append(dict(name=name,setup=raw,steps=[dict(op='execute',input=dict(source=[uid(source)],**flags),
            result=dict(status='completed',output=dict(source=[uid(source)],matches=matches,truncated=False,comparisons='$any',related={},revision='$any'),warnings=warnings),
            expect=dict(ids=[e['id'] for e in raw['entities']],canUndo=False,dirty=False,revision='same'))]))
    texts=deepcopy(setup);texts['entities']=[dict(kind='text',id=i+1,layerId='a',attrs={},p=dict(x=i*10,y=0),text=word,height=.2,rotation=0) for i,word in enumerate(['A01','A02','A01','A03'])]
    semantic_case('yazı değerleri varsayılan olarak aynıdır',texts,1,[1,3])
    semantic_case('yazı çeşitleri açık seçenekle bulunur değerler değişmez',texts,1,[1,2,3,4],textVariants=True)
    styled=deepcopy(texts);styled['entities'][-1].update(bold=True,font='arimo')
    semantic_case('yazı çeşitleri biçim farkını yok saymaz',styled,1,[1,2,3],textVariants=True)
    tagged=deepcopy(setup);block='01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001'
    tagged['blocks']=[dict(id=block,name='Etiketli parça',base=dict(x=0,y=0),entities=[dict(kind='line',id=1,layerId='a',attrs={},a=dict(x=0,y=0),b=dict(x=1,y=0))],
        attributes=[dict(tag='CODE',value='A',p=dict(x=0,y=.2),height=.2,rotation=0)])]
    tagged['entities']=[dict(kind='insert',id=i+1,layerId='a',attrs=dict(CODE=code,DATA=data),block=block,p=dict(x=i*10,y=0),scale=1,rotation=0)
        for i,(code,data) in enumerate([('A','same'),('B','same'),('C','different')])]
    semantic_case('blok öznitelik değeri varsayılan olarak korunur',tagged,1,[1])
    semantic_case('yalnız tanımlı blok etiketlerinin değerleri çeşit olabilir diğer veri korunur',tagged,1,[1,2],attributeVariants=True)
    target = ROOT / "fixtures/commands/v1/cad.blocks.find.json"
    expected = json.dumps(dict(format="kentos.command-cases", version=1, command="cad.blocks.find",
        commandVersion=1, title="Tekrarlanan bileşenleri bul", note="Written by scripts/fixtures/block_find_cases.py; independent expectations, no KentOS imports.",
        setup=setup, cases=cases), ensure_ascii=False, indent=2)+"\n"
    if cli.check:
        if target.read_text() != expected:
            raise SystemExit(f"out of date: {target.relative_to(ROOT)}")
        print(f"cad.blocks.find: {len(cases)} independent cases are current")
    else:
        target.write_text(expected)


if __name__ == "__main__":
    main()
