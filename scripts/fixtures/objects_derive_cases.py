"""The shared cases of cad.objects.derive (docs/adr/0622 §9): Köşelere nokta, Orta hat and Toplu alan.

    python3 scripts/fixtures/objects_derive_cases.py           # writes the file
    python3 scripts/fixtures/objects_derive_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.objects.derive.json, without KentOS code, from the rules:

- Köşelere nokta (docs/adr/0152 §5): the objects in the drawing's order, each path's vertices in order (an area's outer
  ring as written); a place a vertex already gave, or a point holds, passed over (counted when a point holds it);
  names from the first by Yazı's Artır (the trailing number goes up: N1, N2 …), the code as `Kod`, the vertex's
  elevation as `z`; the next name after the last;
- Orta hat (docs/adr/0190): between two straight sides of matched vertices, the middles of the matched vertices, in
  the first side's direction, `matched`; a closed side, a step not above zero, a side without a route, sides of
  objects that do not join are refused;
- Toplu alan (docs/adr/0151): a region the line work closes is an area; its one label (a point's label, a text) its
  attribute; none or many leave it empty and are counted; a region an area already is is not written again.
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
POLYGON = json.loads((OUT / "cad.polygon.create.json").read_text(encoding="utf-8"))
X, Y = 487000, 4420000
FILE = OUT / "cad.objects.derive.json"


def W(p):
    return {"x": X + p[0], "y": Y + p[1]}


def line(id_, a, b, layer="yapi"):
    return {"kind": "line", "id": id_, "layerId": layer, "attrs": {}, "a": W(a), "b": W(b)}


def point(id_, p, label=None):
    e = {"kind": "point", "id": id_, "layerId": "yapi", "attrs": {}, "p": W(p)}
    if label:
        e["label"] = label
    return e


A = [(0, 0), (10, 0), (10, 10), (0, 10)]
SQUARE = [(100, 0), (110, 0), (110, 10), (100, 10)]
ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "yapi", "attrs": {}, "pts": [W(p) for p in A], "zs": [1, 2, 3, 4]},
    {"kind": "polyline", "id": 2, "layerId": "yapi", "attrs": {}, "pts": [W(p) for p in [(20, 0), (30, 0), (30, 10)]]},
    point(3, (30, 10)),
    line(4, (0, 10), (0, 20)),
    line(5, (0, 40), (20, 40)),
    line(6, (0, 44), (20, 44)),
    line(7, (20, 46), (0, 46)),
    line(8, (40, 0), (45, 0)),
    point(9, (40, 0)),
    point(10, (45, 0)),
] + [line(11 + i, SQUARE[i], SQUARE[(i + 1) % 4]) for i in range(4)] + [
    point(15, (105, 5), label="7"),
    point(16, (106, 5), label="8"),
    line(17, (50, 40), (60, 40)),
    line(18, (62, 40), (70, 40)),
]
SETUP = {**{k: v for k, v in POLYGON["setup"].items() if k != "entities"}, "entities": ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NEXT = max(IDS) + 1
UNCHANGED = {"ids": IDS, "dirty": False, "revision": "same"}


def U(i):
    return f"$uidOf:{i}"


def derive(work, layer="yapi", **extra):
    return {"layerId": layer, "work": work, **extra}


def out(n, skipped=0, many=0, existing=0, next_=None, method=None):
    o = {"ids": [NEXT + i for i in range(n)], "uids": [f"$uidOf:{NEXT + i}" for i in range(n)], "skipped": skipped,
         "many": many, "existing": existing}
    if next_ is not None:
        o["next"] = next_
    if method is not None:
        o["method"] = method
    o["revision"] = "$current"
    return o


def done(name, inp, output, undo, fields):
    n = len(output["ids"])
    step = {"op": "execute", "input": inp, "result": {"status": "completed", "output": output, "warnings": []},
            "expect": {"ids": IDS + [NEXT + i for i in range(n)], "fields": fields, "canUndo": True, "dirty": True}}
    back = {"op": "undo", "returns": undo, "expect": {"ids": IDS, "canUndo": False, "canRedo": True}}
    return {"name": name, "steps": [step, back]}


def refused(name, inp, code, message, path, non_finite=None):
    step = {"op": "execute", "input": inp}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


def names(first, n):
    """Yazı's Artır on a trailing number: N1, N2 …"""
    head = first.rstrip("0123456789")
    k = int(first[len(head):])
    return [f"{head}{k + i}" for i in range(n + 1)]


def vertex_cases():
    nm = names("N1", 4)
    return [
        done("Köşelere nokta: alanın köşeleri adları, kodu ve kotlarıyla; adım “Köşelere nokta”",
             derive({"kind": "vertexPoints", "ids": [U(1)], "name": "N1", "code": "KS"}),
             out(4, next_=nm[4]), "Köşelere nokta",
             {str(NEXT + i): {"kind": "point", "p": W(p), "z": i + 1, "label": nm[i], "attrs": {"Kod": "KS"}}
              for i, p in enumerate(A)}),
        done("Köşelere nokta: paylaşılan köşe bir nokta",
             derive({"kind": "vertexPoints", "ids": [U(4), U(1)]}),
             out(5), "Köşelere nokta",
             {str(NEXT + 4): {"p": W((0, 20))}}),
        done("Köşelere nokta: noktası olan köşe atlanır ve sayılır",
             derive({"kind": "vertexPoints", "ids": [U(2)], "name": "P10"}),
             out(2, skipped=1, next_="P12"), "Köşelere nokta",
             {str(NEXT): {"p": W((20, 0)), "label": "P10"}, str(NEXT + 1): {"p": W((30, 0)), "label": "P11"}}),
        refused("Köşelere nokta: bütün köşelerde nokta var", derive({"kind": "vertexPoints", "ids": [U(8)]}),
                "nothing_derived", "Köşelere nokta: yazılacak nokta yok; 2 köşede zaten nokta var.", "work.ids"),
        refused("Köşelere nokta: çizgi ya da alan yok", derive({"kind": "vertexPoints", "ids": [U(3)]}), "not_a_path",
                "Köşelere nokta: seçimde çizgi, çoklu çizgi ya da alan yok.", "work.ids"),
    ]


NO_ROUTE = ("Bu nesnenin üzerinde yürünecek tek bir yolu yok: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da tek "
            "parçalı alan seçin.")


def centerline_cases():
    axis = [(0, 42), (20, 42)]
    return [
        done("Orta hat: iki düz kenarın ekseni, kenar kenar; adım “Orta hat”",
             derive({"kind": "centerline", "first": [U(5)], "second": [U(6)], "step": 1}),
             out(1, method="matched"), "Orta hat",
             {str(NEXT): {"kind": "polyline", "pts": [W(p) for p in axis]}}),
        done("Orta hat: ters çizilmiş ikinci kenar",
             derive({"kind": "centerline", "first": [U(5)], "second": [U(7)], "step": 1}),
             out(1, method="matched"), "Orta hat",
             {str(NEXT): {"pts": [W((0, 43)), W((20, 43))]}}),
        refused("Orta hat: adım sıfır", derive({"kind": "centerline", "first": [U(5)], "second": [U(6)], "step": 0}),
                "invalid_distance", "Adım sıfırdan büyük bir uzunluk olmalı.", "work.step"),
        refused("Orta hat: kapalı kenar",
                derive({"kind": "centerline", "first": [U(1)], "second": [U(6)], "step": 1}), "not_open",
                "Kapalı yolun orta hattı çizilmez; iki açık kenar seçin.", "work.first"),
        refused("Orta hat: noktanın yolu yok",
                derive({"kind": "centerline", "first": [U(3)], "second": [U(6)], "step": 1}), "no_route", NO_ROUTE,
                "work.first"),
        refused("Orta hat: kenarın nesneleri birleşmiyor",
                derive({"kind": "centerline", "first": [U(17), U(18)], "second": [U(6)], "step": 1}),
                "side_not_joined", "Kenarın nesneleri uç uca tek bir yol olmuyor; uçları birleşen çizgileri verin.",
                "work.first"),
        refused("Orta hat: iki kenar aynı",
                derive({"kind": "centerline", "first": [U(5)], "second": [U(5)], "step": 1}), "invalid_input",
                "İkinci kenar birincisiyle aynı olamaz; öbür kenarı verin.", "work.second"),
        refused("Orta hat: adım sonlu değil",
                derive({"kind": "centerline", "first": [U(5)], "second": [U(6)], "step": 1}), "not_finite",
                "Adım sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir uzunluk verin.", "work.step",
                non_finite={"work.step": "NaN"}),
    ]


def polygonize_cases():
    sq_lines = [U(11 + i) for i in range(4)]
    area = {"kind": "polygon", "area": None}
    del area["area"]
    return [
        done("Toplu alan: dört çizginin kapattığı bölge, içindeki nokta etiketiyle; adım “Toplu alan”",
             derive({"kind": "polygonize", "lines": sq_lines, "labels": [U(15)], "attribute": "Ad"}),
             out(1), "Toplu alan",
             {str(NEXT): {"kind": "polygon", "attrs": {"Ad": "7"}}}),
        done("Toplu alan: iki etiketli bölgenin özniteliği boş kalır",
             derive({"kind": "polygonize", "lines": sq_lines, "labels": [U(15), U(16)], "attribute": "Ad"}),
             out(1, many=1), "Toplu alan", {str(NEXT): {"attrs": {}}}),
        done("Toplu alan: etiketsiz bölge sayılır",
             derive({"kind": "polygonize", "lines": sq_lines, "attribute": "Parsel"}),
             out(1, skipped=1), "Toplu alan", {str(NEXT): {"attrs": {}}}),
        refused("Toplu alan: bölge zaten alan",
                derive({"kind": "polygonize", "lines": [U(1)], "attribute": "Ad"}), "nothing_derived",
                "Toplu alan: yazılacak alan yok; hiçbir şey değişmedi.", "work.lines"),
        refused("Toplu alan: çizgi yok", derive({"kind": "polygonize", "lines": [U(3)], "attribute": "Ad"}),
                "not_a_path", "Toplu alan: bölge kapatacak çizgi yok; çizgi, çoklu çizgi, yay ya da alan seçin.",
                "work.lines"),
        refused("Toplu alan: öznitelik adı boş", derive({"kind": "polygonize", "lines": sq_lines, "attribute": " "}),
                "invalid_attribute", "Etiketlerin yazılacağı özniteliğin adını verin (ör. Ad).", "work.attribute"),
    ]


def hidden_case():
    out_ = done("Gizli katmana uyarıyla",
                derive({"kind": "centerline", "first": [U(5)], "second": [U(6)], "step": 1}, layer="gizli"),
                out(1, method="matched"), "Orta hat", {str(NEXT): {"layerId": "gizli"}})
    out_["steps"][0]["result"]["warnings"] = [
        {"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}]
    return out_


def general_cases():
    return [hidden_case(),
        refused("Kilitli katmana yazılmaz", derive({"kind": "vertexPoints", "ids": [U(1)]}, layer="kilitli"),
                "layer_locked",
                "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.",
                "layerId"),
        refused("Olmayan katman", derive({"kind": "vertexPoints", "ids": [U(1)]}, layer="yok"), "layer_not_found",
                "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"),
        refused("Katman değil grup", derive({"kind": "vertexPoints", "ids": [U(1)]}, layer="pafta"), "not_a_layer",
                "“Pafta” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"),
        refused("Çizgi kalınlığı aralık dışında",
                derive({"kind": "centerline", "first": [U(5)], "second": [U(6)], "step": 1}, lineWeight=150),
                "invalid_line_weight", "$any", "lineWeight"),
        refused("Orta hat: kenar boş", derive({"kind": "centerline", "first": [], "second": [U(6)], "step": 1}),
                "too_few_objects", "Kenarın en az bir nesnesini verin.", "work.first"),
        refused("Orta hat: adım çok küçük",
                derive({"kind": "centerline", "first": [U(5)], "second": [U(7)], "step": 0.00001}), "too_many_points",
                "Bu adımla çok nokta olur; adımı büyütün.", "work.step"),
        refused("Olmayan kimlik", derive({"kind": "vertexPoints", "ids": ["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"]}),
                "entity_not_found",
                "“01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin "
                "olabilir. Var olan bir nesnenin kimliğini verin.", "work.ids[0]"),
        refused("Kimlik değil", derive({"kind": "vertexPoints", "ids": ["alan"]}), "invalid_uid",
                "“alan” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).", "work.ids[0]"),
        refused("Sürüm yazısı geçersiz", derive({"kind": "vertexPoints", "ids": [U(1)]}, expectedRevision="012"),
                "invalid_revision",
                "Beklenen sürüm “012” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                "ya da komutun planından alın.", "expectedRevision"),
        {"name": "Plan: yazılmaz", "steps": [
            {"op": "plan", "input": derive({"kind": "vertexPoints", "ids": [U(1)]}),
             "result": {"status": "completed", "output": "$any", "warnings": []},
             "expect": UNCHANGED}]},
        {"name": "Sürüm değiştiyse hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "base"},
            {"op": "execute", "input": derive({"kind": "vertexPoints", "ids": [U(1)]}),
             "result": {"status": "completed", "output": "$any", "warnings": []}},
            {"op": "execute", "input": derive({"kind": "vertexPoints", "ids": [U(2)]}, expectedRevision="$base"),
             "result": {"status": "conflict", "error": {
                 "code": "revision_conflict",
                 "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin "
                            "şimdiki hâline göre yeniden hazırlayın.",
                 "path": "expectedRevision", "revision": "$current"}},
             "expect": {"revision": "same"}}]},
    ]


def build():
    cases = vertex_cases() + centerline_cases() + polygonize_cases() + general_cases()
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.objects.derive", "commandVersion": 1,
            "title": "Nesneden türet",
            "note": "Written by scripts/fixtures/objects_derive_cases.py from the derivations' rules (docs/adr/0622 §9, "
                    "0152, 0190, 0151); do not edit by hand.",
            "setup": SETUP, "cases": cases}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not FILE.exists() or FILE.read_text(encoding="utf-8") != text:
            print("cad.objects.derive durumları güncel değil; betiği --check'siz çalıştırın.")
            sys.exit(1)
        print(f"cad.objects.derive durumları güncel: {len(build()['cases'])} durum.")
        return
    FILE.write_text(text, encoding="utf-8")
    print(f"yazıldı: {FILE.relative_to(ROOT)} ({len(build()['cases'])} durum)")


if __name__ == "__main__":
    main()
