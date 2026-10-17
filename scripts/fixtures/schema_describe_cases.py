"""Writes fixtures/commands/v1/cad.schema.describe.json: the shared cases of `cad.schema.describe` v1 (docs/adr/0443 §1),
their expectations built here without KentOS code from the ADR's rule: the project's layers (tree order, groups left
out), forms, value lists and relations (the project's orders), each as a summary (id, name, layer, child, count, latest
release), its definition only with `expand`; `kinds`, `layer` (itself, its forms, the lists its fields link, the
relations it is a side of) and `query` (the id or name holding the words, Turkish folded) narrow; a page is `limit`
long (1–100, 20 when none), `next` the offset of the next page as a decimal text, `total` all found.

    python3 scripts/fixtures/schema_describe_cases.py           # writes the file
    python3 scripts/fixtures/schema_describe_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import domains_cases as dc  # noqa: E402  (layers, fields and value lists written without KentOS code)
import forms_cases as fc  # noqa: E402  (a form's release and digest)

OUT = ROOT / "fixtures/commands/v1/cad.schema.describe.json"

FIELD = lambda i, f: {"type": "field", "id": i, "field": f}  # noqa: E731
BINA_V1 = {"revision": 1, "items": [FIELD("malzeme", "Malzeme"), FIELD("kat", "kat-sayisi")]}
BINA_DRAFT = {"revision": 2, "items": [FIELD("malzeme", "Malzeme"), FIELD("kat", "kat-sayisi"),
                                       {"type": "group", "id": "ek", "label": "Ek bilgi",
                                        "items": [FIELD("not", "Not"), FIELD("etiket", "Etiket")]}]}
FORMS = [
    {"id": "bina-denetim", "name": "Bina denetimi", "layer": "bina", "draft": copy.deepcopy(BINA_DRAFT),
     "releases": [fc.release_of(BINA_V1, 1), fc.release_of(BINA_DRAFT, 2)]},
    {"id": "depo-kayit", "name": "Depo kaydı", "layer": "depo", "draft": {"revision": 1, "items": [FIELD("m", "Malzeme")]}},
    {"id": "yol-olcu", "name": "Yol ölçüsü", "layer": "yol", "draft": {"revision": 1, "items": [FIELD("g", "Genişlik")]},
     "releases": [fc.release_of({"revision": 1, "items": [FIELD("g", "Genişlik")]}, 1)], "enforce": True},
]
RELATIONS = [
    {"id": "bina-depo", "name": "Binanın depoları", "parent": "bina", "child": "depo", "field": "Malzeme"},
    {"id": "yol-bina", "name": "Yolun binaları", "parent": "yol", "child": "bina", "field": "Not", "onDelete": "restrict"},
]
LAYERS = dc.LAYERS
DOMAINS = dc.DOMAINS


def setup():
    s = dc.setup()
    s["name"] = "Şema"
    s["settings"]["forms"] = copy.deepcopy(FORMS)
    s["settings"]["relations"] = copy.deepcopy(RELATIONS)
    return s


IDS = [i for i, _, _ in dc.ENTITIES]
UNCHANGED = {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same"}


def leaves(nodes):
    for n in nodes:
        if n["type"] == "layer":
            yield n
        yield from leaves(n.get("children", []))


def fold(text):
    """Lower case the Turkish way: I ı, İ i; a letter whose lower case is more than one keeps itself."""
    out = []
    for c in text:
        if c == "I":
            out.append("ı")
        elif c == "İ":
            out.append("i")
        else:
            low = c.lower()
            out.append(low if len(low) == 1 else c)
    return "".join(out)


def count_items(items):
    return sum(1 + count_items(i.get("items", [])) for i in items)


def found(kinds=(), query=None, layer=None, expand=False):
    want = lambda k: not kinds or k in kinds  # noqa: E731
    words = fold(query) if query is not None and query.strip() else None
    hit = lambda i, n: words is None or words in fold(i) or words in fold(n)  # noqa: E731
    out = []
    if want("layers"):
        for n in leaves(LAYERS):
            if (layer and n["id"] != layer) or not hit(n["id"], n["name"]):
                continue
            item = {"kind": "layers", "id": n["id"], "name": n["name"], "count": len(n.get("fields", []))}
            if expand:
                item["detail"] = copy.deepcopy(n.get("fields", []))
            out.append(item)
    if want("forms"):
        for f in FORMS:
            if (layer and f["layer"] != layer) or not hit(f["id"], f["name"]):
                continue
            item = {"kind": "forms", "id": f["id"], "name": f["name"], "layer": f["layer"],
                    "count": count_items(f["draft"]["items"])}
            if f.get("releases"):
                item["version"] = f["releases"][-1]["version"]
            if expand:
                item["detail"] = {"draft": copy.deepcopy(f["draft"]),
                                  "releases": [{"version": r["version"], "digest": r["digest"]}
                                               for r in f.get("releases", [])],
                                  "enforce": bool(f.get("enforce"))}
            out.append(item)
    if want("domains"):
        linked = None
        if layer:
            node = next(n for n in leaves(LAYERS) if n["id"] == layer)
            linked = [x["domain"] for x in node.get("fields", []) if "domain" in x]
        for d in DOMAINS:
            if (linked is not None and d["id"] not in linked) or not hit(d["id"], d["name"]):
                continue
            item = {"kind": "domains", "id": d["id"], "name": d["name"], "count": len(d.get("values", []))}
            if expand:
                item["detail"] = copy.deepcopy(d)
            out.append(item)
    if want("relations"):
        for r in RELATIONS:
            if (layer and layer not in (r["parent"], r["child"])) or not hit(r["id"], r["name"]):
                continue
            item = {"kind": "relations", "id": r["id"], "name": r["name"], "layer": r["parent"], "child": r["child"]}
            if expand:
                item["detail"] = copy.deepcopy(r)
            out.append(item)
    return out


def ok(name, inp, op="execute"):
    limit = inp.get("limit", 20)
    offset = int(inp.get("cursor", "0"))
    all_ = found(inp.get("kinds", ()), inp.get("query"), inp.get("layer"), inp.get("expand", False))
    end = min(offset + limit, len(all_))
    output = {"items": all_[offset:end], "total": len(all_)}
    if end < len(all_):
        output["next"] = str(end)
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "completed", "output": output,
                                                                        "warnings": []}, "expect": UNCHANGED}]}


def refused(name, inp, code, message, path, op="execute"):
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": UNCHANGED}]}


def cursor_text(c):
    return f"“{c}” bu listenin sayfa imleci değil; bir önceki cevabın next'ini verin."


def cases():
    total = len(found())
    return [
        ok("bütün şema: katmanlar, formlar, listeler, ilişkiler bu sırayla, özetleriyle", {}),
        ok("yalnız katmanlar: gruplar yok, alan sayılarıyla", {"kinds": ["layers"]}),
        ok("türlerin verilen sırası listenin sırasını değiştirmez", {"kinds": ["relations", "forms"]}),
        ok("formlar: katmanı, öğe sayısı iç içe öğelerle, son yayın", {"kinds": ["forms"]}),
        ok("bir katmanın sahip oldukları", {"layer": "bina"}),
        ok("ilişkinin çocuk tarafı da katmanındır", {"layer": "depo"}),
        ok("kilitli katman da listelenir", {"layer": "eski"}),
        ok("sözcük adda, Türkçe katlanarak (I ı)", {"query": "YAPI"}),
        ok("sözcük kimlikte", {"query": "denetim"}),
        ok("büyük İ küçük i olur", {"query": "İLİŞKİ"}),
        ok("boş sözcük süzmez", {"query": "  ", "kinds": ["domains"]}),
        ok("bulunmayan sözcük: boş sayfa", {"query": "kanalizasyon"}),
        ok("tanımlarıyla: alanlar, form gövdesi ve yayınların özetleri, liste, ilişki", {"layer": "bina", "expand": True}),
        ok("formun tanımı: kurallar uygulanıyor", {"kinds": ["forms"], "query": "yol", "expand": True}),
        ok("ilk sayfa ve sonrakinin imleci", {"limit": 3}),
        ok("ikinci sayfa", {"limit": 3, "cursor": "3"}),
        ok("son sayfa: imleç yok", {"limit": 5, "cursor": "10"}),
        ok("listenin sonundaki imleç: boş sayfa", {"limit": 5, "cursor": str(total)}),
        ok("en büyük sayfa", {"limit": 100}),
        ok("plan aynı cevabı verir", {"kinds": ["domains", "relations"], "limit": 2}, op="plan"),
        ok("bir katmanın listeleri: alanlarının bağladıkları", {"kinds": ["domains"], "layer": "yol"}),
        refused("sayfa sıfır olamaz", {"limit": 0}, "invalid_count",
                "Bir sayfada 1 ile 100 arasında öğe listelenir; 0 verildi.", "limit"),
        refused("sayfa en çok 100", {"limit": 101}, "invalid_count",
                "Bir sayfada 1 ile 100 arasında öğe listelenir; 101 verildi.", "limit"),
        refused("imleç sayı değil", {"cursor": "abc"}, "invalid_cursor", cursor_text("abc"), "cursor"),
        refused("imleç başında sıfırla", {"cursor": "03"}, "invalid_cursor", cursor_text("03"), "cursor"),
        refused("imleç listenin ötesinde", {"cursor": str(total + 1)}, "invalid_cursor",
                f"“{total + 1}” bu listenin sayfa imleci değil: liste {total} öğe; bir önceki cevabın next'ini verin.",
                "cursor"),
        refused("grup katman değildir", {"layer": "grup"}, "layer_not_found",
                "“grup” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"),
        refused("olmayan katman, planda da", {"layer": "su"}, "layer_not_found",
                "“su” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer", op="plan"),
        refused("sayfa imlecinden önce denetlenir", {"limit": 0, "cursor": "x"}, "invalid_count",
                "Bir sayfada 1 ile 100 arasında öğe listelenir; 0 verildi.", "limit"),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.schema.describe", "commandVersion": 1,
            "title": "Şemayı keşfet",
            "note": "ADR 0443 §1: projenin veri şeması sayfa sayfa. Written by scripts/fixtures/schema_describe_cases.py "
                    "with the layers of domains_cases.py, without KentOS code; do not edit by hand.",
            "setup": setup(), "cases": cases()}


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    n = len(json.loads(text)["cases"])
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({n} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({n} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
