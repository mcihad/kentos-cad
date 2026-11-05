#!/usr/bin/env python3
"""The shared cases of cad.standards.conflicts (docs/specs/drawing-standard-package.md, “Conflicts”), written from the
specification without KentOS code.

    python3 scripts/fixtures/standards_conflicts_cases.py           # writes fixtures/commands/v1/cad.standards.conflicts.json
    python3 scripts/fixtures/standards_conflicts_cases.py --check   # writes nothing; compares

The bound standards are compared in their order. A later standard whose global rules (naming, fonts, lineTypes,
byLayer, in that order) differ from the first's is a `rules` conflict named “Genel kurallar”. A layer, text style or
dimension style rule whose name (Turkish case folded, trimmed) an earlier standard already defined is compared with
that first definition: a layer by color (case aside), lineType, lineWeight (1e-9), plot and from; a text style by font
and height (1e-9, absent apart from present); a dimension style by height (1e-9). A resource line type is compared by
its whole JSON but its name, numbers within 1e-9 (kind `lineTypeDefinition`). Each conflict names the first and the later standard and the
values of exactly the differing fields (null when absent). Order: per later standard, rules, layers, text styles,
dimension styles, then line types. The query writes nothing; with no bound standard the list is empty.
"""

import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.standards.conflicts.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
FOLD = str.maketrans({"I": "ı", "İ": "i"})


def fold(name):
    return name.strip().translate(FOLD).lower()


def standard(name, layers=(), texts=(), dims=(), naming=("A-*",), fonts=("barlow",), line_types=("continuous",),
             by_layer=True):
    return {"name": name, "layers": list(layers), "naming": list(naming), "fonts": list(fonts),
            "textStyles": list(texts), "dimensionStyles": list(dims), "lineTypes": list(line_types), "byLayer": by_layer}


def package(id_, std, line_types=None):
    p = {"format": "kentos.drawing-standard", "version": 2 if line_types is not None else 1, "id": id_,
         "revision": "2026.1", "standard": std}
    if line_types is not None:
        p["resources"] = {"lineTypes": line_types}
    return p


def layer(name, color="ink", line_type="continuous", weight=0.25, plot=True, from_=None):
    out = {"name": name, "color": color, "lineType": line_type, "lineWeight": weight}
    if not plot:
        out["plot"] = False
    if from_ is not None:
        out["from"] = from_
    return out


def eq(a, b):
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        return abs(a - b) <= 1e-9
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(eq(x, y) for x, y in zip(a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(eq(a[k], b[k]) for k in a)
    return a == b


def conflicts(packages):
    out = []
    first_pkg = packages[0] if packages else None
    seen = {"layer": {}, "textStyle": {}, "dimensionStyle": {}, "lineType": {}}
    for p in packages:
        s = p["standard"]
        if first_pkg and p["id"] != first_pkg["id"]:
            f = first_pkg["standard"]
            fields = [k for k in ("naming", "fonts", "lineTypes", "byLayer") if f.get(k) != s.get(k)]
            if fields:
                out.append({"kind": "rules", "name": "Genel kurallar", "firstId": first_pkg["id"], "laterId": p["id"],
                            "fields": fields, "firstValues": {k: f.get(k) for k in fields}, "laterValues": {k: s.get(k) for k in fields}})
        for l in s.get("layers", []):
            k = fold(l["name"])
            if k in seen["layer"]:
                fid, fl = seen["layer"][k]
                look = lambda x: {"color": x["color"], "lineType": x["lineType"], "lineWeight": x["lineWeight"],
                                  "plot": x.get("plot", True), "from": x.get("from")}
                a, b = look(fl), look(l)
                fields = []
                if a["color"].lower() != b["color"].lower():
                    fields.append("color")
                for key in ("lineType",):
                    if a[key] != b[key]:
                        fields.append(key)
                if abs(a["lineWeight"] - b["lineWeight"]) > 1e-9:
                    fields.append("lineWeight")
                for key in ("plot", "from"):
                    if a[key] != b[key]:
                        fields.append(key)
                if fields:
                    out.append({"kind": "layer", "name": l["name"], "firstId": fid, "laterId": p["id"], "fields": fields,
                                "firstValues": {x: a[x] for x in fields}, "laterValues": {x: b[x] for x in fields}})
            else:
                seen["layer"][k] = (p["id"], l)
        for t in s.get("textStyles", []):
            k = fold(t["name"])
            if k in seen["textStyle"]:
                fid, ft = seen["textStyle"][k]
                fields = []
                if ft["font"] != t["font"]:
                    fields.append("font")
                ha, hb = ft.get("height"), t.get("height")
                if (ha is None) != (hb is None) or (ha is not None and abs(ha - hb) > 1e-9):
                    fields.append("height")
                if fields:
                    out.append({"kind": "textStyle", "name": t["name"], "firstId": fid, "laterId": p["id"], "fields": fields,
                                "firstValues": {x: ft.get(x) for x in fields}, "laterValues": {x: t.get(x) for x in fields}})
            else:
                seen["textStyle"][k] = (p["id"], t)
        for d in s.get("dimensionStyles", []):
            k = fold(d["name"])
            if k in seen["dimensionStyle"]:
                fid, fd = seen["dimensionStyle"][k]
                if abs(fd["height"] - d["height"]) > 1e-9:
                    out.append({"kind": "dimensionStyle", "name": d["name"], "firstId": fid, "laterId": p["id"],
                                "fields": ["height"], "firstValues": {"height": fd["height"]}, "laterValues": {"height": d["height"]}})
            else:
                seen["dimensionStyle"][k] = (p["id"], d)
        for t in (p.get("resources") or {}).get("lineTypes", []):
            k = fold(t["name"])
            if k in seen["lineType"]:
                fid, ft = seen["lineType"][k]
                a = {x: y for x, y in ft.items() if x != "name"}
                b = {x: y for x, y in t.items() if x != "name"}
                fields = sorted(x for x in set(a) | set(b) if not (x in a and x in b and eq(a[x], b[x])))
                if fields:
                    out.append({"kind": "lineTypeDefinition", "name": t["name"], "firstId": fid, "laterId": p["id"], "fields": fields,
                                "firstValues": {x: a.get(x) for x in fields}, "laterValues": {x: b.get(x) for x in fields}})
            else:
                seen["lineType"][k] = (p["id"], t)
    return out


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Standart farkları",
            "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 100, "workspace": "cad", "drawingUnit": "m"},
            "origin": {"x": 0, "y": 0},
            "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                        "style": STYLE, "children": []}],
            "activeLayer": "0", "entities": [], "styles": {"items": [], "categories": []}}


A = standard("Ofis A", layers=[layer("A-DUVAR", weight=0.5, from_="duvar*"), layer("A-KAPI", color="#FF0000")],
             texts=[{"name": "A-Yazı", "font": "barlow", "height": 2.5}], dims=[{"name": "A-Ölçü", "height": 2.5}])
LT_A = [{"name": "GAZ", "elements": [{"length": 2.0}, {"length": -1.0}], "unit": "paper"}]


def case(name, packages):
    docs = [{"text": json.dumps(p, ensure_ascii=False)} for p in packages]
    return {"name": name, "steps": [
        {"op": "execute", "command": "cad.settings.set", "input": {"standardDocs": docs}, "result": "$any"},
        {"op": "execute", "input": {}, "within": 1e-9,
         "result": {"status": "completed", "output": {"conflicts": conflicts(packages), "revision": "$current"}, "warnings": []},
         "expect": {"revision": "same"}}]}


def variant(**kw):
    s = json.loads(json.dumps(A))
    for k, v in kw.items():
        s[k] = v
    return s


def cases():
    a = package("ofis-a", A)
    out = [
        {"name": "Bağlı standart yokken boş liste", "steps": [{"op": "execute", "input": {},
            "result": {"status": "completed", "output": {"conflicts": [], "revision": "$current"}, "warnings": []},
            "expect": {"revision": "same", "canUndo": False}}]},
        case("Tek standart: fark yok", [a]),
        case("Aynı içerikli ikinci standart: fark yok", [a, package("ofis-b", A)]),
        case("Adlandırma kuralı farklı", [a, package("ofis-b", variant(naming=["B-*"]))]),
        case("Yazı tipleri farklı", [a, package("ofis-b", variant(fonts=["arimo"]))]),
        case("İzinli çizgi tipleri farklı", [a, package("ofis-b", variant(lineTypes=["continuous", "dashed"]))]),
        case("Katmana göre görünüş farklı", [a, package("ofis-b", variant(byLayer=False))]),
        case("Bütün genel kurallar farklı", [a, package("ofis-b", variant(naming=[], fonts=["arimo"], lineTypes=["dotted"], byLayer=False))]),
        case("Katman rengi farklı; harf büyüklüğü sayılmaz", [a, package("ofis-b", variant(layers=[
            layer("A-DUVAR", weight=0.5, from_="duvar*"), layer("a-kapi", color="#00FF00")]))]),
        case("Renk yalnız harf büyüklüğünde farklı: fark yok", [a, package("ofis-b", variant(layers=[layer("A-KAPI", color="#ff0000")]))]),
        case("Katman kalınlığı 1e-9 altında: fark yok", [a, package("ofis-b", variant(layers=[layer("A-DUVAR", weight=0.5000000001, from_="duvar*")]))]),
        case("Katman kalınlığı ve yazdırma farklı", [a, package("ofis-b", variant(layers=[layer("A-DUVAR", weight=0.35, plot=False, from_="duvar*")]))]),
        case("Katmanın kaynak deseni farklı ya da yok", [a, package("ofis-b", variant(layers=[layer("A-DUVAR", weight=0.5)]))]),
        case("Katman çizgi tipi farklı", [a, package("ofis-b", variant(layers=[layer("A-KAPI", color="#FF0000", line_type="dashed")]))]),
        case("Yazı stilinin yazı tipi ve yüksekliği farklı", [a, package("ofis-b", variant(textStyles=[{"name": "A-YAZI", "font": "arimo", "height": 3.0}]))]),
        case("Yazı yüksekliği birinde yok", [a, package("ofis-b", variant(textStyles=[{"name": "A-Yazı", "font": "barlow"}]))]),
        case("Ölçü yüksekliği farklı", [a, package("ofis-b", variant(dimensionStyles=[{"name": "A-Ölçü", "height": 3.5}]))]),
        case("Üç standart: ikinci ve üçüncü ilkle karşılaştırılır", [a, package("ofis-b", variant(fonts=["arimo"])),
            package("ofis-c", variant(dimensionStyles=[{"name": "A-Ölçü", "height": 2.0}]))]),
        case("İkinci standardın yeni katmanı üçüncüyle karşılaştırılır", [a, package("ofis-b", variant(layers=[layer("B-EK")])),
            package("ofis-c", variant(layers=[layer("B-EK", color="#00FF00")]))]),
        case("Çizgi tipi kaynağı farklı", [package("ofis-a", A, LT_A), package("ofis-b", A, [
            {"name": "gaz", "elements": [{"length": 3.0}, {"length": -1.0}], "unit": "paper"}])]),
        case("Çizgi tipi kaynağının birimi farklı", [package("ofis-a", A, LT_A), package("ofis-b", A, [
            {"name": "GAZ", "elements": [{"length": 2.0}, {"length": -1.0}], "unit": "drawing"}])]),
        case("Çizgi tipi 1e-9 altında aynı", [package("ofis-a", A, LT_A), package("ofis-b", A, [
            {"name": "GAZ", "elements": [{"length": 2.0000000001}, {"length": -1.0}], "unit": "paper"}])]),
        case("Kurallar, katman ve stiller birlikte: sırasıyla", [a, package("ofis-b", variant(
            byLayer=False, layers=[layer("A-KAPI", color="#0000FF")], textStyles=[{"name": "A-Yazı", "font": "arimo", "height": 2.5}],
            dimensionStyles=[{"name": "A-Ölçü", "height": 5.0}]))]),
    ]
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.standards.conflicts", "commandVersion": 1,
           "title": "Bağlı standartlardaki farklı tanımları göster",
           "note": "Bağımsız başvuru scripts/fixtures/standards_conflicts_cases.py: kurallar belgeden elle yazılır; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0




# ── cad.standards.resolve (the same packages) ───────────────────────────

RESOLVE_OUT = ROOT / "fixtures/commands/v1/cad.standards.resolve.json"


def resolved(packages):
    """The bound standards as one: the first's global rules and name; each named layer, text and dimension style rule
    from the first standard that has its name (Turkish case folded); each resource line type from the first that has
    it (case aside); every one's origin."""
    first = packages[0]
    o = lambda p: {"id": p["id"], "revision": p["revision"]}
    s = json.loads(json.dumps(first["standard"]))
    prov = {"rules": o(first), "layers": {}, "textStyles": {}, "dimensionStyles": {}, "lineTypes": {}, "hatchPatterns": {},
            "blocks": {}, "sheetTemplates": {}, "leaderStyles": {}, "textStyleDefinitions": {}, "dimensionStyleDefinitions": {}}
    for key, pk in (("layers", "layers"), ("textStyles", "textStyles"), ("dimensionStyles", "dimensionStyles")):
        names = set()
        for item in s.get(key, []):
            names.add(fold(item["name"]))
            prov[pk][item["name"]] = o(first)
        for p in packages[1:]:
            for item in p["standard"].get(key, []):
                if fold(item["name"]) not in names:
                    names.add(fold(item["name"]))
                    prov[pk][item["name"]] = o(p)
                    s.setdefault(key, []).append(item)
    resources = {}
    seen = set()
    for p in packages:
        for t in (p.get("resources") or {}).get("lineTypes", []):
            if t["name"].strip().lower() not in seen:
                seen.add(t["name"].strip().lower())
                prov["lineTypes"][t["name"]] = o(p)
                resources.setdefault("lineTypes", []).append(t)
    s["name"] = "Projeye bağlı standartlar"
    s["description"] = ("Genel kurallar ilk kaynaktan, her adlı tanım ilk bulunduğu kaynaktan alınır. "
                        f"İlk kaynak: {first['id']}.")
    for k in ("naming", "fonts", "textStyles", "dimensionStyles", "lineTypes"):
        if not s.get(k):
            s.pop(k, None)
    if not s.get("byLayer"):
        s.pop("byLayer", None)
    return {"standard": s, "resources": resources, "sources": [p["id"] for p in packages], "provenance": prov,
            "revision": "$current"}


def resolve_case(name, packages):
    docs = [{"text": json.dumps(p, ensure_ascii=False)} for p in packages]
    return {"name": name, "steps": [
        {"op": "execute", "command": "cad.settings.set", "input": {"standardDocs": docs}, "result": "$any"},
        {"op": "execute", "input": {}, "within": 1e-9,
         "result": {"status": "completed", "output": resolved(packages), "warnings": []}, "expect": {"revision": "same"}}]}


def resolve_cases():
    a = package("ofis-a", A)
    plain = [c for c in cases()[1:]]
    out = [{"name": "Bağlı standart yokken: ret", "steps": [{"op": "execute", "input": {},
        "result": {"status": "failed", "error": {"code": "no_standard", "message": "Projeye bağlı standart yok.", "path": "standardDocs"}},
        "expect": {"revision": "same", "canUndo": False}}]}]
    for c in plain:
        docs = c["steps"][0]["input"]["standardDocs"]
        packages = [json.loads(d["text"]) for d in docs]
        out.append(resolve_case(c["name"].replace("fark yok", "tek tanım").replace("farklı", "farklıysa ilk kaynak kazanır"), packages))
    out.append(resolve_case("İkinci standardın yeni katmanı ve stili eklenir, kaynağı ikinci", [a, package("ofis-b", variant(
        layers=[layer("B-YENI")], textStyles=[{"name": "B-Not", "font": "arimo"}], dimensionStyles=[{"name": "B-Ölçü", "height": 1.8}]))]))
    return out


def write_resolve(check):
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.standards.resolve", "commandVersion": 1,
           "title": "Bağlı standartları birleştir",
           "note": "Bağımsız başvuru scripts/fixtures/standards_conflicts_cases.py: kurallar belgeden elle yazılır; KentOS kodu yok.",
           "setup": setup(), "cases": resolve_cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if check:
        return RESOLVE_OUT.exists() and RESOLVE_OUT.read_text("utf-8") == text
    RESOLVE_OUT.write_text(text, "utf-8")
    print(f"yazıldı: {RESOLVE_OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return True


if __name__ == "__main__":
    code = main()
    if "--check" in sys.argv[1:]:
        if not write_resolve(True):
            print(f"{RESOLVE_OUT.relative_to(ROOT)} güncel değil.")
            code = 1
    else:
        write_resolve(False)
    sys.exit(code)
