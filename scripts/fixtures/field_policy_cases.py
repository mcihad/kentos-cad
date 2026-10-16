"""Writes fixtures/commands/v1/cad.entities.edit.policies.json: the shared cases of the layer fields' split and merge
policies (docs/adr/0419) through `cad.entities.edit`, their expectations built here from the ADR's rules without KentOS
code: which changes are a split object's pieces (an update, a kept-data replacement or new piece of it) and which a
merge's one piece and its contributors; a share is each piece's floor of total × weight / weights (areas, else lengths)
in the field's last digit, the rest one unit each to the largest remainders, the earlier on a tie; a sum exact; the least
and greatest by their numbers. The desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/field_policy_cases.py           # writes the file
    python3 scripts/fixtures/field_policy_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.entities.edit.policies.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

PARSEL_FIELDS = [
    {"name": "Ada", "kind": "text"},
    {"name": "Pay", "kind": "integer", "split": "ratio", "merge": "sum"},
    {"name": "Alan", "kind": "decimal", "scale": 2, "split": "ratio", "merge": "sum"},
    {"name": "Not", "kind": "text", "split": "clear", "merge": "clear"},
    {"name": "Tür", "kind": "text", "default": "bölünmüş", "split": "default", "merge": "first"},
    {"name": "Kot", "kind": "decimal", "scale": 1, "split": "copy", "merge": "max"},
    {"name": "En", "kind": "integer", "merge": "min"},
]
YOL_FIELDS = [{"name": "Ad", "kind": "text"}, {"name": "Uzunluk", "kind": "decimal", "scale": 1, "split": "ratio", "merge": "sum"}]


def node(id_, name, fields):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": copy.deepcopy(STYLE), "children": [], "fields": fields}


X, Y = 500000, 4400000


def rect(x0, x1, y0=0, y1=10):
    return [{"x": X + x0, "y": Y + y0}, {"x": X + x1, "y": Y + y0}, {"x": X + x1, "y": Y + y1}, {"x": X + x0, "y": Y + y1}]


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "pts": rect(0, 20),
     "attrs": {"Ada": "101", "Pay": "1000", "Alan": "200.00", "Not": "eski", "Tür": "arsa", "Kot": "12.5", "En": "3"}},
    {"kind": "polygon", "id": 2, "layerId": "parsel", "pts": rect(20, 40),
     "attrs": {"Ada": "101", "Pay": "500", "Alan": "199.99", "Kot": "15.0", "En": "7"}},
    {"kind": "polygon", "id": 3, "layerId": "parsel", "pts": rect(40, 70), "attrs": {"Pay": "1001", "Alan": "0.05"}},
    {"kind": "polygon", "id": 4, "layerId": "parsel", "pts": rect(70, 80), "attrs": {"Pay": "-7", "Alan": "-0.03"}},
    {"kind": "line", "id": 5, "layerId": "yol", "a": {"x": X, "y": Y - 10}, "b": {"x": X + 30, "y": Y - 10},
     "attrs": {"Ad": "Cadde", "Uzunluk": "10.0"}},
    {"kind": "line", "id": 6, "layerId": "yol", "a": {"x": X + 30, "y": Y - 10}, "b": {"x": X + 40, "y": Y - 10},
     "attrs": {"Ad": "Sokak", "Uzunluk": "10.5"}},
    {"kind": "polygon", "id": 7, "layerId": "parsel", "pts": rect(80, 90), "attrs": {}},
]
IDS = [e["id"] for e in ENTITIES]
FIELDS = {"parsel": PARSEL_FIELDS, "yol": YOL_FIELDS}


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Alanların bölme ve birleştirme kuralı",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
            "origin": {"x": X, "y": Y}, "layers": [node("parsel", "Parsel", PARSEL_FIELDS), node("yol", "Yol", YOL_FIELDS)],
            "activeLayer": "parsel", "entities": copy.deepcopy(ENTITIES), "styles": {"items": [], "categories": []}}


# ── The rules (docs/adr/0419) ─────────────────────────────────────────────


def units(f, v):
    scale = 0 if f["kind"] == "integer" else f.get("scale")
    if scale is None or v is None:
        return None
    minus = v.startswith("-")
    rest = v[1:] if minus else v
    whole, _, frac = rest.partition(".")
    if not whole.isdigit() or (frac and not frac.isdigit()) or len(frac) > scale:
        return None
    n = int(whole + frac) * 10 ** (scale - len(frac))
    return -n if minus else n


def text_of(f, n):
    scale = f.get("scale", 0) if f["kind"] == "decimal" else 0
    digits = str(abs(n))
    sign = "-" if n < 0 else ""
    if scale == 0:
        return sign + digits
    padded = digits.rjust(scale + 1, "0")
    return f"{sign}{padded[:-scale]}.{padded[-scale:]}"


def share(total, weights):
    if not all(math.isfinite(w) and w >= 0 for w in weights) or sum(weights) <= 0:
        weights = [1.0] * len(weights)
    s = sum(weights)
    whole = abs(total)
    out, rest = [], []
    for i, w in enumerate(weights):
        q = float(whole) * w / s
        fl = math.floor(q)
        out.append(min(int(fl), whole))
        rest.append((q - fl, i))
    rest.sort(key=lambda r: (-r[0], r[1]))
    left = whole - sum(out)
    k = 0
    while left > 0:
        out[rest[k % len(rest)][1]] += 1
        left -= 1
        k += 1
    return [-u if total < 0 else u for u in out]


def split_values(f, value, weights):
    p = f.get("split")
    if p is None or p == "copy":
        return None
    if p == "clear":
        return [None] * len(weights)
    if p == "default":
        return [f.get("default")] * len(weights)
    total = units(f, value)
    if total is None:
        return None
    return [text_of(f, u) for u in share(total, weights)]


def merge_value(f, values):
    p = f.get("merge")
    given = [v for v in values if v]
    if p is None or p == "first":
        return "keep"
    if p == "clear":
        return None
    if p == "sum":
        us = [units(f, v) for v in given]
        if any(u is None for u in us):
            return "keep"
        return text_of(f, sum(us)) if given else None
    best = None
    for v in given:
        try:
            x = float(v)
        except ValueError:
            continue
        if best is None or (x < best[1] if p == "min" else x > best[1]):
            best = (v, x)
    return best[0] if best else None


def area(pts):
    s = 0.0
    for i in range(len(pts)):
        a, b = pts[i], pts[(i + 1) % len(pts)]
        s += a["x"] * b["y"] - b["x"] * a["y"]
    return abs(s) / 2


def weight(g):
    if g["kind"] == "polygon":
        a = area(g["pts"])
        if a > 0:
            return a
    if g["kind"] == "line":
        return math.hypot(g["b"]["x"] - g["a"]["x"], g["b"]["y"] - g["a"]["y"])
    return 0.0


def put(attrs, name, v):
    if v:
        attrs[name] = v
    else:
        attrs.pop(name, None)


def edited(operation, changes):
    """The objects the edit writes, by slot: (expected entities, ids)."""
    es = {e["id"]: copy.deepcopy(e) for e in ENTITIES}
    nxt = max(es) + 1
    pieces, gone, out_changed, out_created = [], [], [], []
    for ch in changes:
        src = int(ch.get("uid", ch.get("from", "")).split(":")[1])
        base = es[src]
        if ch["kind"] in ("update", "replace"):
            keep = ch["kind"] == "update" or ch.get("keepData")
            e = {"kind": ch["geometry"]["kind"], "id": src, "layerId": base["layerId"],
                 "attrs": copy.deepcopy(base["attrs"]) if keep else {}, **{k: v for k, v in ch["geometry"].items() if k != "kind"}}
            out_changed.append(e)
            if keep:
                pieces.append((e, src))
        elif ch["kind"] == "add":
            e = {"kind": ch["geometry"]["kind"], "id": nxt, "layerId": base["layerId"],
                 "attrs": copy.deepcopy(base["attrs"]) if ch.get("keepData") else {},
                 **{k: v for k, v in ch["geometry"].items() if k != "kind"}}
            nxt += 1
            out_created.append(e)
            if ch.get("keepData"):
                pieces.append((e, src))
        else:
            gone.append(src)
    splits = operation in ("areaSplit", "split", "break", "partsSplit")
    merges = operation in ("areaUnion", "join", "partsJoin")
    if splits:
        for src in dict.fromkeys(s for _, s in pieces):
            at = [e for e, s in pieces if s == src]
            if len(at) < 2:
                continue
            ws = [weight(e) for e in at]
            for f in FIELDS[es[src]["layerId"]]:
                vals = split_values(f, ENTITIES_BY[src]["attrs"].get(f["name"]), ws)
                if vals is None:
                    continue
                for e, v in zip(at, vals):
                    put(e["attrs"], f["name"], v)
    if merges and gone and len(pieces) == 1:
        e, src = pieces[0]
        for f in FIELDS[es[src]["layerId"]]:
            vals = [ENTITIES_BY[src]["attrs"].get(f["name"])] + [ENTITIES_BY[g]["attrs"].get(f["name"]) for g in gone if g != src]
            v = merge_value(f, vals)
            if v != "keep":
                put(e["attrs"], f["name"], v)
    ids = sorted((set(IDS) - set(gone)) | {e["id"] for e in out_created})
    return {str(e["id"]): e for e in out_changed + out_created}, ids, len(out_created)


ENTITIES_BY = {e["id"]: e for e in ENTITIES}


def uid(slot):
    return f"$uidOf:{slot}"


def poly(pts):
    return {"kind": "polygon", "pts": pts}


def line(x0, x1):
    return {"kind": "line", "a": {"x": X + x0, "y": Y - 10}, "b": {"x": X + x1, "y": Y - 10}}


def case(name, operation, changes):
    entities, ids, made = edited(operation, changes)
    named = lambda c: "$uid:s" + c["uid"].split(":")[1]  # noqa: E731
    changed = [named(c) for c in changes if c["kind"] in ("update", "replace")]
    removed = [named(c) for c in changes if c["kind"] == "remove"]
    slots = sorted({int(c.get("uid", c.get("from", "")).split(":")[1]) for c in changes})
    first_new = max(IDS) + 1
    created = [uid(first_new + i) for i in range(made)]
    result = {"status": "completed", "output": {"changed": changed, "created": created, "removed": removed, "revision": "$current"},
              "warnings": []}
    return {"name": name, "steps": [{"op": "captureUid", "id": s, "as": f"s{s}"} for s in slots] + [
        {"op": "execute", "input": {"operation": operation, "changes": changes}, "result": result,
         "expect": {"ids": ids, "entities": entities, "canUndo": True, "revision": "changed"}},
        {"op": "undo", "expect": {"ids": IDS, "entities": {str(k): v for k, v in
                                                          ((int(i), ENTITIES_BY.get(int(i))) for i in entities) if v}}},
    ]}


def rep(slot, geometry, keep=True):
    c = {"kind": "replace", "uid": uid(slot), "geometry": geometry}
    if keep is not None:
        c["keepData"] = keep
    return c


def add(slot, geometry, keep=True):
    c = {"kind": "add", "from": uid(slot), "geometry": geometry}
    if keep is not None:
        c["keepData"] = keep
    return c


def remove(slot):
    return {"kind": "remove", "uid": uid(slot)}


def cases():
    return [
        case("Alan böl ikiye: pay ve alan yarı yarıya, not boşalır, tür varsayılanını alır, ada ve kot kalır", "areaSplit",
             [rep(1, poly(rect(0, 10))), add(1, poly(rect(10, 20)))]),
        case("Alan böl üçe, eşit olmayan alanlar: 100, 60, 40", "areaSplit",
             [rep(1, poly(rect(0, 10))), add(1, poly(rect(10, 16))), add(1, poly(rect(16, 20)))]),
        case("eşit üç parça: 1001'in kalanı ilk parçaya; 0.05'in kalanları en büyük kalanlara", "areaSplit",
             [rep(3, poly(rect(40, 50))), add(3, poly(rect(50, 60))), add(3, poly(rect(60, 70)))]),
        case("eksi değer: -7 ikiye -4 ve -3", "areaSplit", [rep(4, poly(rect(70, 75))), add(4, poly(rect(75, 80)))]),
        case("yeni parça önce: ağırlıklar parçaların yazılış sırasıyla", "areaSplit",
             [add(1, poly(rect(0, 5))), rep(1, poly(rect(5, 20)))]),
        case("verisini taşımayan parçaya kural uygulanmaz", "areaSplit",
             [rep(1, poly(rect(0, 10))), add(1, poly(rect(10, 20)), keep=False)]),
        case("tek parça: bölünme yok, değerler kalır", "areaSplit", [rep(1, poly(rect(0, 18)))]),
        case("boş öznitelikler boş kalır, tür varsayılanını alır", "areaSplit",
             [rep(7, poly(rect(80, 85))), add(7, poly(rect(85, 90)))]),
        case("iki nesne birlikte bölünür: her biri kendi parçalarıyla", "areaSplit",
             [rep(1, poly(rect(0, 10))), add(1, poly(rect(10, 20))), rep(2, poly(rect(20, 25))), add(2, poly(rect(25, 40)))]),
        case("Parçala: çizgi uzunluklarıyla 10 ve 20", "split", [rep(5, line(0, 10)), add(5, line(10, 30))]),
        case("Parçala: 10.5 üçte bir ve üçte iki, kalan büyük kalana", "split",
             [rep(6, line(30, 33.3333)), add(6, line(33.3333, 40))]),
        case("Alan birleştir: pay ve alan toplanır, not boşalır, tür ilkinin, kot en büyüğü, en en küçüğü", "areaUnion",
             [remove(1), remove(2), add(1, poly(rect(0, 40)))]),
        case("Alan birleştir ikinciden: ilkinin değeri ikincininki", "areaUnion",
             [remove(1), remove(2), add(2, poly(rect(0, 40)))]),
        case("Alan birleştir üç parsel: toplamlar", "areaUnion",
             [remove(1), remove(2), remove(3), add(3, poly(rect(0, 70)))]),
        case("eksi toplam", "areaUnion", [remove(3), remove(4), add(4, poly(rect(40, 80)))]),
        case("boş değerler toplama girmez; hiç değer yoksa boş kalır", "areaUnion",
             [remove(7), remove(4), add(7, poly(rect(70, 90)))]),
        case("Birleştir: yerinde kalan çizgi uzunlukları toplar", "join", [rep(5, line(0, 40)), remove(6)]),
        case("iki parça kalırsa birleştirme kuralı uygulanmaz", "areaUnion",
             [remove(1), remove(2), add(1, poly(rect(0, 20))), add(2, poly(rect(20, 40)))]),
        case("bölme ya da birleştirme olmayan iş kuralları uygulamaz", "areaSubtract",
             [rep(1, poly(rect(0, 15)))]),
        case("Parçalara ayır: yerinde kalan ve yeni parça paylaşır", "partsSplit",
             [{"kind": "update", "uid": uid(2), "geometry": poly(rect(20, 30))}, add(2, poly(rect(30, 40)))]),
        case("Parçaları birleştir: yerinde kalan silinenlerle toplar", "partsJoin",
             [{"kind": "update", "uid": uid(1), "geometry": poly(rect(0, 40))}, remove(2)]),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.edit", "commandVersion": 1,
            "title": "Alanların bölme ve birleştirme kuralı",
            "note": "Written by scripts/fixtures/field_policy_cases.py from docs/adr/0419; do not edit by hand.",
            "setup": setup(), "cases": cases()}


def main() -> int:
    # The reference's arithmetic on the ADR's examples.
    assert share(100, [1.0, 1.0, 1.0]) == [34, 33, 33] and share(-10, [3.0, 1.0]) == [-8, -2] and share(7, [0.0, 0.0]) == [4, 3]
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel ({len(build()['cases'])} durum)")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
