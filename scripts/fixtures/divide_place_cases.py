"""Böl and Ölç with blocks, a first distance and the ends: shared cases of cad.entities.divide (docs/adr/0299).

    python3 scripts/fixtures/divide_place_cases.py           # writes the file
    python3 scripts/fixtures/divide_place_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.divide.places.json. The ADR's rules, computed here without KentOS code (an
elliptical arc's lengths with mpmath at 40 digits):

- Where: Parça sayısı n puts points at s = L·k/n (k = 1 … n−1; a closed path k = 0 … n−1); Aralık d at s = d, 2d, …
  below L (not at L), or with `start` a at s = a, a + d, … below L. `fromEnd` measures from the object's end.
- Uçlar (`ends`): `start`, `end` or `both` also put a point at the open object's own start or end, unless one is
  already there (within 1e-9·max(1, L)); a closed object's ends are its start, already among the parts' points. In
  the order written: the measuring start's end, the division's points, the measuring finish's end.
- Blok (`block`): a block's insert at each place instead of a point, its base point there, scale `scale`, turned by
  `rotation` plus, with `align`, the object's direction there (the tangent of the path from its start to its end,
  atan2 of the tangent; a closed circle runs counter-clockwise).
- Refusals added, in order after the division's own: `not_finite` and `invalid_distance` for `start` (not below 0),
  `start_needs_step` (a first distance with parts), `not_finite` and `invalid_scale` for the block's numbers,
  `unknown_block` (no block of that name), `no_points` when the first distance is past the end.

Plans compare within 1e-9 (an elliptical arc's places by its true length too); writes compare exactly where the
numbers are exact.
"""
import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.dont_write_bytecode = True

mp.mp.dps = 40
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.divide.places.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731
POST = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3001"
TREE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3002"


def num(v):
    v = float(v)
    return int(v) if v.is_integer() else v


def p(x, y):
    return {"x": num(x), "y": num(y)}


def node(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def base(i, layer="cizim"):
    return {"id": i, "layerId": layer, "attrs": {}}


ENTITIES = [
    {"kind": "line", **base(1), "a": p(0, 0), "b": p(10, 0)},
    {"kind": "polyline", **base(2), "pts": [p(0, 10), p(6, 10), p(6, 14)]},
    {"kind": "arc", **base(3), "c": p(30, 0), "r": 4, "a0": 0, "a1": math.pi / 2},
    {"kind": "circle", **base(4), "c": p(30, 20), "r": 2},
    {"kind": "ellipse", **base(5), "c": p(50, 0), "major": p(4, 0), "ratio": 0.5, "t0": 0, "t1": math.pi},
    {"kind": "polygon", **base(6), "pts": [p(40, 20), p(44, 20), p(44, 24), p(40, 24)]},
    {"kind": "point", **base(7), "p": p(90, 0)},
]
IDS = [e["id"] for e in ENTITIES]
NEXT = 8
BLOCKS = [
    {"id": POST, "name": "Direk", "base": p(0, 0),
     "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": p(0, 0), "r": 0.3},
                  {"kind": "line", "id": 2, "layerId": "0", "attrs": {}, "a": p(0, 0), "b": p(1, 0)}]},
    {"id": TREE, "name": "Ağaç", "base": p(0, 0),
     "entities": [{"kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": p(0, 0), "r": 0.8}]},
]
UNCHANGED = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "Böl ve Ölç",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(40, 20),
        "layers": [node("cizim", "Çizim"), node("nokta", "Nokta")],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
        "blocks": copy.deepcopy(BLOCKS),
    }


# ── The objects as paths: length, point and direction at s from the start ─────────────────────────────────────────

BY_ID = {e["id"]: e for e in ENTITIES}


def ellipse_len(e, t0, t1):
    a = mp.mpf(e["major"]["x"])
    b = a * mp.mpf(e["ratio"])
    return mp.quad(lambda t: mp.sqrt((a * mp.sin(t)) ** 2 + (b * mp.cos(t)) ** 2), [t0, t1])


def path(e):
    """(length, closed, place(s) -> ((x, y), direction))."""
    k = e["kind"]
    if k == "line":
        a, b = e["a"], e["b"]
        dx, dy = b["x"] - a["x"], b["y"] - a["y"]
        L = math.hypot(dx, dy)
        return L, False, lambda s: ((a["x"] + dx * s / L, a["y"] + dy * s / L), math.atan2(dy, dx))
    if k in ("polyline", "polygon"):
        pts = [(q["x"], q["y"]) for q in e["pts"]]
        closed = k == "polygon"
        if closed:
            pts = pts + [pts[0]]
        edges = list(zip(pts, pts[1:]))
        lens = [math.hypot(b[0] - a[0], b[1] - a[1]) for a, b in edges]

        def place(s):
            for (a, b), l in zip(edges, lens):
                if s <= l + 1e-12:
                    t = s / l
                    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t), math.atan2(b[1] - a[1], b[0] - a[0])
                s -= l
            raise AssertionError("past the end")
        return sum(lens), closed, place
    if k == "arc":
        c, r = e["c"], e["r"]

        def place(s):
            ang = e["a0"] + s / r
            return (c["x"] + r * math.cos(ang), c["y"] + r * math.sin(ang)), math.atan2(math.cos(ang), -math.sin(ang))
        return r * (e["a1"] - e["a0"]), False, place
    if k == "circle":
        c, r = e["c"], e["r"]

        def place(s):
            ang = s / r
            return (c["x"] + r * math.cos(ang), c["y"] + r * math.sin(ang)), math.atan2(math.cos(ang), -math.sin(ang))
        return 2 * math.pi * r, True, place
    if k == "ellipse":
        a = e["major"]["x"]
        b = a * e["ratio"]
        L = float(ellipse_len(e, e["t0"], e["t1"]))

        def place(s):
            t = mp.findroot(lambda t: ellipse_len(e, e["t0"], t) - s, e["t0"] + (e["t1"] - e["t0"]) * s / L)
            t = float(t)
            return ((e["c"]["x"] + a * math.cos(t), e["c"]["y"] + b * math.sin(t)),
                    math.atan2(b * math.cos(t), -a * math.sin(t)))
        return L, False, place
    raise AssertionError(k)


def places(e, division, from_end=False, start=None, ends="none"):
    L, closed, place = path(e)
    eps = 1e-9 * max(1.0, L)
    if division["kind"] == "parts":
        n = division["count"]
        ss = [L * k / n for k in range(0 if closed else 1, n)]
    else:
        d = division["length"]
        s = d if start is None else start
        ss = []
        while s < L - eps:
            ss.append(s)
            s += d
    head = ends in (("start", "both") if not from_end else ("end", "both"))
    tail = ends in (("end", "both") if not from_end else ("start", "both"))
    if not closed and ss:
        pass
    if not closed:
        if head and not (ss and ss[0] <= eps):
            ss = [0.0] + ss
        if tail and not (ss and ss[-1] >= L - eps):
            ss = ss + [L]
    return [place(L - s if from_end else s) for s in ss]


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def point(at):
    return {"kind": "point", "id": 0, "layerId": "nokta", "attrs": {}, "p": p(*at)}


def insert(block, at, rotation, scale=1):
    return {"kind": "insert", "id": 0, "layerId": "nokta", "attrs": {}, "block": block, "p": p(*at), "scale": num(scale),
            "rotation": num(rotation)}


def made(e, block=None, **kw):
    out = []
    for at, direction in places(BY_ID[e], **kw):
        if block is None:
            out.append(point(at))
        else:
            turn = block.get("rotation", 0) + (direction if block.get("align", True) else 0)
            out.append(insert(POST if block["name"] == "Direk" else TREE, at, turn, block.get("scale", 1)))
    return out


def inp(uid, division, **more):
    return {"uid": U(uid), "division": division, "layerId": "nokta", **more}


def case(name, uid, division, within=1e-9, exact=False, undo=False, **more):
    kw = {k: more[k] for k in ("from_end", "start", "ends") if k in more}
    block = more.get("block")
    i = inp(uid, division, **{"fromEnd" if k == "from_end" else k: v for k, v in more.items()})
    entities = made(uid, block=block, division=division, **kw)
    steps = [{"op": "plan", "input": i, "within": within,
              "result": {"status": "completed", "warnings": [], "output": {"entities": entities, "revision": "$current"}},
              "expect": {"revision": "same"}}]
    ids = list(range(NEXT, NEXT + len(entities)))
    written = {"op": "execute", "input": i,
               "result": {"status": "completed", "warnings": [],
                          "output": {"created": [U(n) for n in ids], "ids": ids, "revision": "$current"}},
               "expect": {"ids": IDS + ids, "revision": "changed", "canUndo": True}}
    if exact:
        written["expect"]["entities"] = {str(n): {**e, "id": n} for n, e in zip(ids, entities)}
    steps.append(written)
    if undo:
        steps.append({"op": "undo", "returns": "Böl", "expect": {"ids": IDS}})
    return {"name": name, "steps": steps}


def refused(name, input_, code, message, path_, non_finite=None):
    step = {"op": "execute", "input": input_}
    if non_finite:
        step["nonFinite"] = non_finite
    step["result"] = {"status": "failed", "error": {"code": code, "message": message, "path": path_}}
    step["expect"] = UNCHANGED
    return {"name": name, "steps": [step]}


PARTS = lambda n: {"kind": "parts", "count": n}  # noqa: E731
STEP = lambda d: {"kind": "step", "length": d}  # noqa: E731
DIREK = {"name": "Direk"}


def cases():
    return [
        case("Blok: çizgi dört parçaya, aralarına üç Direk; çizginin doğrultusunda; adım “Böl”", 1, PARTS(4), block=DIREK,
             exact=True, undo=True),
        case("Blok hizalı: köşeli çoklu çizgide her yerleştirme kendi kenarının doğrultusunda", 2, PARTS(4), block=DIREK),
        case("Blok hizasız: yerleştirmeler bloğun kendi dönüşüyle", 2, PARTS(4), block={"name": "Direk", "align": False},
             exact=True),
        case("Blok hizalı ve 45° dönük: dönüş doğrultuya eklenir", 2, PARTS(4),
             block={"name": "Direk", "rotation": math.pi / 4}),
        case("Blok ölçekli: 2 kat", 1, PARTS(2), block={"name": "Ağaç", "scale": 2}, exact=True),
        case("Yay üç parçaya: Direkler yayın teğetinde", 3, PARTS(3), block=DIREK),
        case("Daire dört parçaya: başlangıç dahil dört Direk, saat yönü tersine teğette", 4, PARTS(4), block=DIREK),
        case("Elips yayı üç parçaya: Direkler elipsin teğetinde, yay uzunluğuyla eşit aralıkta", 5, PARTS(3), block=DIREK),
        case("Sondan ölçmek: Direkler yine nesnenin doğrultusunda", 1, STEP(4), block=DIREK, from_end=True),
        case("Aralık 3, başlangıç 1: noktalar 1, 4 ve 7'de", 1, STEP(3), start=1, exact=True),
        case("Aralık 3, başlangıç 0: başta da bir nokta", 1, STEP(3), start=0, exact=True),
        case("Uçlar son: aralığın kısa kalanından sonra sona da bir nokta", 1, STEP(3), ends="end", exact=True),
        case("Uçlar ikisi: başa ve sona da", 1, STEP(3), ends="both", exact=True),
        case("Uçlar ikisi ve aralık tam bölüyor: uçta yinelenen nokta yok", 1, STEP(2.5), ends="both", start=0),
        case("Parça sayısı ve uçlar ikisi: beş nokta, uçlar dahil", 1, PARTS(4), ends="both", exact=True),
        case("Kapalı alanda uçlar: başlangıç zaten parçaların noktası, eklenmez", 6, PARTS(4), ends="both", exact=True),
        case("Sondan ölçmek ve uçlar baş: nesnenin başı en sonda yazılır", 1, STEP(4), from_end=True, ends="start",
             exact=True),
        case("Aralık nesneden uzun ama uçlar ikisi: yalnız iki uç", 1, STEP(12), ends="both", exact=True),
        case("Blok ve uçlar ikisi: köşeli yolun uçlarında da hizalı Direk", 2, STEP(4), block=DIREK, ends="both"),
        refused("Başlangıç parça sayısıyla verilmez", inp(1, PARTS(4), start=1), "start_needs_step",
                "Başlangıç yalnız aralıkla bölmede verilir; parça sayısıyla bölmede kaldırın.", "start"),
        refused("Başlangıç eksi", inp(1, STEP(3), start=-1), "invalid_distance",
                "Başlangıç sıfır ya da sıfırdan büyük sonlu bir uzunluk (metre) olmalı.", "start"),
        refused("Başlangıç sonlu değil", inp(1, STEP(3), start=1), "not_finite",
                "Başlangıç sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir uzunluk verin.", "start",
                non_finite={"start": "NaN"}),
        refused("Başlangıç nesnenin boyundan uzun", inp(1, STEP(3), start=12), "no_points",
                "Başlangıç nesnenin boyundan uzun; nokta konmadı. Daha kısa bir başlangıç verin.", "start"),
        refused("Adı bilinmeyen blok", inp(1, PARTS(4), block={"name": "Yok"}), "unknown_block",
                "“Yok” adlı blok çizimde tanımlı değil. Blok oluştur ile tanımlayın ya da var olan bir bloğun adını verin.",
                "block.name"),
        refused("Blok ölçeği sıfır", inp(1, PARTS(4), block={"name": "Direk", "scale": 0}), "invalid_scale",
                "Bloğun ölçeği sıfırdan büyük olmalı.", "block.scale"),
        refused("Blok dönüşü sonlu değil", inp(1, PARTS(4), block={"name": "Direk", "rotation": 0}), "not_finite",
                "Bloğun dönüşü sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir açı verin.", "block.rotation",
                non_finite={"block.rotation": "Infinity"}),
        refused("Nokta bölünmez (eski kural)", inp(7, PARTS(4), block=DIREK), "not_divisible",
                "Bu türdeki nesne bölünmez: çizgi, çoklu çizgi, alan, yay, daire, elips ya da eğri olmalı.", "uid"),
    ]


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.divide", "commandVersion": 1,
            "title": "Böl ve Ölç: blok, hizalama, başlangıç ve uçlar",
            "note": "ADR 0299. Bağımsız başvuru: scripts/fixtures/divide_place_cases.py (elips yayı mpmath'le 40 basamakta).",
            "setup": setup(), "cases": cases()}


def main():
    check = "--check" in sys.argv[1:]
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} tutarlı: {len(data['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(data['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
