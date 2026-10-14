"""The shared cases of the product command cad.snap.find (docs/adr/0272).

    python3 scripts/fixtures/snap_find_cases.py           # writes the file
    python3 scripts/fixtures/snap_find_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.snap.find.json. The snaps follow the snap's rules (docs/adr/0029, 0163, 0269,
0271), written here by hand: a candidate counts within the aperture; the least distance × weight wins (end point,
node and insertion point 1, crossing 1.02, centre and centroid 1.08, quadrant 1.1, midpoint 1.15, perpendicular
1.25); the nearest point only when nothing else is; `all` gives every candidate best first, a point met again once;
a hidden layer's objects are not snapped; a crossing's object is the first of its two in the drawing's order. The
place on the object is the independent reference of docs/adr/0263 (`subref_cases.place_at`, beside this file),
not KentOS code. The drawing is chosen so that every point and every along place's `t` is a double exactly.
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
sys.path.insert(0, str(Path(__file__).resolve().parent))
from subref_cases import place_at  # noqa: E402  (the independent reference of places)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.snap.find.json"

E0, N0 = 487000, 4420000
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}


def P(x, y):
    return {"x": x, "y": y}


def layer(i, name, visible=True):
    return {"id": i, "name": name, "type": "layer", "visible": visible, "locked": False, "expanded": True,
            "style": STYLE, "children": []}


SQUARE = {"kind": "polygon", "id": 1, "layerId": "yapi", "attrs": {},
          "pts": [P(E0, N0), P(E0 + 10, N0), P(E0 + 10, N0 + 10), P(E0, N0 + 10)]}
WALL = {"kind": "line", "id": 2, "layerId": "yapi", "attrs": {}, "a": P(E0 - 2, N0 + 1), "b": P(E0 + 12, N0 + 1)}
CIRCLE = {"kind": "circle", "id": 3, "layerId": "yapi", "attrs": {}, "c": P(E0 + 30, N0), "r": 5}
TEXT = {"kind": "text", "id": 4, "layerId": "yapi", "attrs": {}, "p": P(E0 + 40, N0 + 5), "text": "Bina",
        "height": 1.0, "rotation": 0}
POINT = {"kind": "point", "id": 5, "layerId": "yapi", "attrs": {}, "p": P(E0 + 50, N0)}
HIDDEN = {"kind": "line", "id": 6, "layerId": "gizli", "attrs": {}, "a": P(E0 + 60, N0), "b": P(E0 + 70, N0)}
ENTITIES = [SQUARE, WALL, CIRCLE, TEXT, POINT, HIDDEN]
IDS = [e["id"] for e in ENTITIES]
SLOT = {e["id"]: e for e in ENTITIES}

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Kenet bul",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": P(E0, N0), "layers": [layer("yapi", "Yapı"), layer("gizli", "Gizli", visible=False)],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
}
SAME = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def snap(kind, x, y, slot=None, placed=True):
    """A snap as the answer says it; its place from the independent reference."""
    out = {"kind": kind, "point": P(float(x), float(y))}
    if slot is not None:
        out["uid"] = f"$uidOf:{slot}"
        if placed:
            pl = place_at(SLOT[slot], kind, P(x, y))
            assert pl is not None, (kind, x, y, slot)
            out["place"] = pl
    return out


def done(*snaps):
    return {"status": "completed", "output": {"snaps": list(snaps), "revision": "$current"}, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, op="execute", **more):
    s = {"op": op, "input": inp, "result": result, "expect": SAME}
    s.update(more)
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def at(x, y):
    return P(x, y)


NOT_FINITE = "değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def cases():
    out = []
    out.append(case("alanın köşesi: uç nokta, kimliği ve yeri (köşe 2)",
                    step({"at": at(E0 + 10.2, N0 + 9.9), "aperture": 0.5},
                         done(snap("endpoint", E0 + 10, N0 + 10, 1)))))
    out.append(case("alanın alt kenarının ortası: orta nokta, kenar 0",
                    step({"at": at(E0 + 5.1, N0 + 0.1), "aperture": 0.5},
                         done(snap("midpoint", E0 + 5, N0, 1)))))
    out.append(case("duvarla alanın kesişimi: yeri yok, ilk nesnenin kimliği",
                    step({"at": at(E0 + 0.1, N0 + 1.05), "aperture": 0.5},
                         done(snap("intersection", E0, N0 + 1, 1, placed=False)))))
    out.append(case("dairenin merkezi",
                    step({"at": at(E0 + 30.1, N0 + 0.2), "aperture": 0.5},
                         done(snap("center", E0 + 30, N0, 3)))))
    out.append(case("dairenin doğu çeyreği: kenar üstündeki yer (t 0)",
                    step({"at": at(E0 + 35.1, N0 + 0.1), "aperture": 0.5},
                         done(snap("quadrant", E0 + 35, N0, 3)))))
    out.append(case("yazının ekleme noktası: düğüm 0",
                    step({"at": at(E0 + 40.1, N0 + 5.1), "aperture": 0.5},
                         done(snap("insertion", E0 + 40, N0 + 5, 4)))))
    out.append(case("noktanın düğümü",
                    step({"at": at(E0 + 50.2, N0 - 0.1), "aperture": 0.5},
                         done(snap("node", E0 + 50, N0, 5)))))
    out.append(case("son noktadan alanın doğu kenarına dik: kenar 1'in dörtte biri",
                    step({"at": at(E0 + 10.1, N0 + 2.6), "aperture": 0.5, "from": at(E0 + 20, N0 + 2.5),
                          "kinds": ["perpendicular"]},
                         done(snap("perpendicular", E0 + 10, N0 + 2.5, 1)))))
    out.append(case("en yakın: duvarın ortasındaki nokta (t 0,5)",
                    step({"at": at(E0 + 5, N0 + 1.2), "aperture": 0.5, "kinds": ["nearest"]},
                         done(snap("nearest", E0 + 5, N0 + 1, 2)))))
    out.append(case("ağırlık merkezi: yeri yok",
                    step({"at": at(E0 + 5.1, N0 + 5.1), "aperture": 0.5, "kinds": ["centroid"]},
                         done(snap("centroid", E0 + 5, N0 + 5, 1, placed=False)))))
    out.append(case("bütün adaylar en iyiden: kesişim, sonra köşe",
                    step({"at": at(E0 + 0.1, N0 + 0.9), "aperture": 1.5, "all": True},
                         done(snap("intersection", E0, N0 + 1, 1, placed=False), snap("endpoint", E0, N0, 1)))))
    out.append(case("yalnız uç nokta istenince kenarın ortası kenetlenmez",
                    step({"at": at(E0 + 5.1, N0 + 0.1), "aperture": 0.5, "kinds": ["endpoint"]}, done())))
    out.append(case("açıklıkta aday yok: boş",
                    step({"at": at(E0 + 20, N0 + 20), "aperture": 0.5}, done())))
    out.append(case("gizli katmanın nesnesi kenetlenmez",
                    step({"at": at(E0 + 60.1, N0 + 0.1), "aperture": 0.5}, done())))
    out.append(case("plan aynı cevabı verir; validate yalnız denetler",
                    step({"at": at(E0 + 10.2, N0 + 9.9), "aperture": 0.5},
                         done(snap("endpoint", E0 + 10, N0 + 10, 1)), op="plan"),
                    step({"at": at(E0 + 10.2, N0 + 9.9), "aperture": 0.5},
                         {"status": "completed", "output": None, "warnings": []}, op="validate")))
    out.append(case("tıklamanın doğusu sonlu olmalı",
                    step({"at": at(E0, N0), "aperture": 0.5},
                         failed("not_finite", f"Tıklamanın doğu (Y) {NOT_FINITE}", "at.x"),
                         nonFinite={"at.x": "Infinity"})))
    out.append(case("açıklık sonlu olmalı",
                    step({"at": at(E0, N0), "aperture": 0.5},
                         failed("not_finite",
                                "Kenet açıklığı sonlu bir sayı değil (NaN ya da sonsuz). Metre olarak bir sayı verin.",
                                "aperture"), nonFinite={"aperture": "NaN"})))
    out.append(case("son noktanın kuzeyi sonlu olmalı",
                    step({"at": at(E0, N0), "aperture": 0.5, "from": at(E0, N0)},
                         failed("not_finite", f"Son noktanın kuzey (X) {NOT_FINITE}", "from.y"),
                         nonFinite={"from.y": "-Infinity"})))
    for a in (0, -1):
        out.append(case(f"açıklık {a}: invalid_aperture",
                        step({"at": at(E0, N0), "aperture": a},
                             failed("invalid_aperture",
                                    "Kenet açıklığı sıfırdan büyük olmalı: tıklamanın yakınında aranacak uzaklığı metre olarak verin.",
                                    "aperture"))))
    out.append(case("boş tür listesi: no_snap_kinds",
                    step({"at": at(E0, N0), "aperture": 0.5, "kinds": []},
                         failed("no_snap_kinds",
                                "Hiç kenet türü verilmedi: kinds'i kaldırın (varsayılan türler) ya da en az bir tür verin.",
                                "kinds"))))
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.snap.find", "commandVersion": 1,
        "title": "Kenet bul: tür, nokta, nesnenin kimliği ve yeri",
        "note": "ADR 0272. Denetim sırası: tıklamanın, açıklığın ve son noktanın sayıları; açıklık sıfırdan büyük; "
                "tür listesi boş değil. Çizim değişmez. Bağımsız başvuru: scripts/fixtures/snap_find_cases.py "
                "(yerler subref_cases.py'nin place_at'inden).",
        "setup": SETUP, "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"kenet bul durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
