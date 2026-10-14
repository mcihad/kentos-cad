"""The shared cases of the product command cad.entities.pick (docs/adr/0276).

    python3 scripts/fixtures/entities_pick_cases.py           # writes the file
    python3 scripts/fixtures/entities_pick_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.pick.json. What a click can mean follows the selection's rule (docs/adr/0187
§1), written here by hand: an object whose point or edge is within the tolerance (a point object's within half as far
again) by that distance, then the areas the click is inside by their size, the drawing's order among equals; only
objects shown (a hidden layer's never, a locked layer's do). The sub-object each falls on is the independent
reference of docs/adr/0276 (`subref_cases.place_near`, `place_near_in`, beside this file), not KentOS code. The
sub-object of a point object is searched half as far again, as the click reaches it. The
drawing is chosen so that every point and every `along` fraction is a double exactly.
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no __pycache__ in the tree
sys.path.insert(0, str(Path(__file__).resolve().parent))
from subref_cases import (  # noqa: E402  (the independent reference of places)
    closest_seg, paths, piece, place_near, place_near_in, vertices,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.pick.json"

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
TOL = 0.5


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(i, name, visible=True, locked=False, kind="layer", children=()):
    return {"id": i, "name": name, "type": kind, "visible": visible, "locked": locked, "expanded": True,
            "style": STYLE, "children": list(children)}


def base(kind, i, lay):
    return {"kind": kind, "id": i, "layerId": lay, "attrs": {}}


DOOR_ID, ROOM_ID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4301", "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4302"
DOOR = {"id": DOOR_ID, "name": "Kapı", "base": P(0, 0), "entities": [
    {**base("line", 1, "yapi"), "a": P(0, 0), "b": P(0, 1)}]}
ROOM = {"id": ROOM_ID, "name": "Oda", "base": P(0, 0), "entities": [
    {**base("polygon", 1, "yapi"), "pts": [P(0, 0), P(6, 0), P(6, 4), P(0, 4)]},
    {**base("insert", 2, "yapi"), "block": DOOR_ID, "p": P(2, 0), "scale": 1.0, "rotation": 0.0}]}
DEFS = {DOOR_ID: DOOR, ROOM_ID: ROOM}

ENTITIES = [
    {**base("polygon", 1, "yapi"), "pts": [P(0, 0), P(10, 0), P(10, 10), P(0, 10)]},
    {**base("polygon", 2, "yapi"), "pts": [P(2, 2), P(4, 2), P(4, 4), P(2, 4)]},
    {**base("line", 3, "yol"), "a": P(-5, 5), "b": P(15, 5)},
    {**base("point", 4, "yapi"), "p": P(20, 0)},
    {**base("point", 5, "yapi"), "p": P(20, 5), "parts": [{"p": P(22, 5)}]},
    {**base("polyline", 6, "yol"), "pts": [P(30, 0), P(34, 0), P(34, 4)]},
    {**base("insert", 7, "yapi"), "block": ROOM_ID, "p": P(50, 0), "scale": 1.0, "rotation": 0.0},
    {**base("polygon", 8, "gizli"), "pts": [P(0, 30), P(10, 30), P(10, 40), P(0, 40)]},
    {**base("line", 9, "kilitli"), "a": P(20, 20), "b": P(30, 20)},
]
IDS = [e["id"] for e in ENTITIES]
SLOT = {e["id"]: e for e in ENTITIES}
SHOWN = {"yapi", "yol", "kilitli"}

SETUP = {
    "format": "kentos.document", "version": 1, "name": "Tıklanan nesneler",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": P(0, 0),
    "layers": [layer("yapi", "Yapı"), layer("yol", "Yol"), layer("gizli", "Gizli", visible=False),
               layer("koruma", "Koruma", locked=True, kind="group", children=[layer("kilitli", "Sit alanı")])],
    "activeLayer": "yapi", "entities": ENTITIES, "styles": {"items": [], "categories": []},
    "blocks": [DOOR, ROOM],
}
SAME = {"ids": IDS, "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}

# ── What a click can mean (docs/adr/0187 §1) ───────────────────────────────


def ring_area(pts):
    n = len(pts)
    return abs(sum(pts[i]["x"] * pts[(i + 1) % n]["y"] - pts[(i + 1) % n]["x"] * pts[i]["y"] for i in range(n))) / 2


def inside(at, pts):
    c, n = False, len(pts)
    for i in range(n):
        a, b = pts[i], pts[(i + 1) % n]
        if (a["y"] > at["y"]) != (b["y"] > at["y"]):
            x = a["x"] + (at["y"] - a["y"]) * (b["x"] - a["x"]) / (b["y"] - a["y"])
            if at["x"] < x:
                c = not c
    return c


def shapes(e):
    """The shapes a click tests: an insert's pieces, any other object itself."""
    if e["kind"] != "insert":
        return [e]
    return [piece(DEFS, e, p) for p in paths(DEFS, e["block"])]


def score(e, at):
    """(edge distance within reach or None, area around the click or None)."""
    edge = area = None
    for s in shapes(e):
        if s["kind"] == "point":
            pts = [s["p"]] + [q["p"] for q in s.get("parts") or []]
            d, reach = min(math.hypot(q["x"] - at["x"], q["y"] - at["y"]) for q in pts), TOL_NOW * 1.5
        else:
            if s["kind"] == "line":
                segs = [(s["a"], s["b"])]
            else:
                pts = [p for _, p in vertices(s)]
                n = len(pts)
                segs = [(pts[i], pts[(i + 1) % n]) for i in range(n if s["kind"] == "polygon" else n - 1)]
            d, reach = min(closest_seg(a, b, at)[1] for a, b in segs), TOL_NOW
        if d <= reach and (edge is None or d < edge):
            edge = d
        if s["kind"] == "polygon" and inside(at, s["pts"]):
            a = ring_area(s["pts"])
            if area is None or a < area:
                area = a
    return edge, area


def hits(at):
    edges, areas = [], []
    for e in ENTITIES:
        if e["layerId"] not in SHOWN:
            continue
        edge, area = score(e, at)
        if edge is not None:
            edges.append((edge, e["id"]))
        elif area is not None:
            areas.append((area, e["id"]))
    return [i for _, i in sorted(edges, key=lambda x: x[0])] + [i for _, i in sorted(areas, key=lambda x: x[0])]


def pick(slot, at):
    e = SLOT[slot]
    out = {"uid": f"$uidOf:{slot}"}
    if e["kind"] == "insert":
        got = place_near_in(DEFS, e, at, TOL_NOW)
        if got:
            out["place"] = got["place"]
            if got["path"]:
                out["path"] = got["path"]
    else:
        # A point object's points half as far again, as the click reaches it.
        pl = place_near(e, at, TOL_NOW * (1.5 if e["kind"] == "point" else 1))
        if pl:
            out["place"] = pl
    return out


TOL_NOW = TOL


def done(picks, kinds_left=0):
    return {"status": "completed", "output": {"picks": picks, "leftOut": {"kinds": kinds_left, "layers": 0, "hidden": 0},
                                              "revision": "$current"}, "warnings": []}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, op="execute", **more):
    s = {"op": op, "input": inp, "result": result, "expect": SAME}
    s.update(more)
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def click(name, x, y, kinds=None, tolerance=TOL, op="execute"):
    global TOL_NOW
    TOL_NOW = tolerance
    at = P(x, y)
    found = hits(at)
    left = 0
    if kinds is not None:
        kept = [i for i in found if SLOT[i]["kind"] in kinds]
        left = len(found) - len(kept)
        found = kept
    inp = {"at": at, "tolerance": tolerance}
    if kinds is not None:
        inp["kinds"] = kinds
    out = case(name, step(inp, done([pick(i, at) for i in found], left), op=op))
    TOL_NOW = TOL
    return out


NOT_FINITE = "değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin."


def cases():
    return [
        click("alanın köşesinin yanı: köşesi", 10.25, 9.75),
        click("çizginin üstü, alanın içi: önce çizgi kenar boyunca, sonra alan yersiz", 5, 5.25),
        click("iki alanın içi: küçükten büyüğe, yersiz", 3, 3),
        click("küçük alanın kenarı, büyüğün içi", 3, 4.25),
        click("küçük alanın köşesi kenarından önce", 2.125, 4.125),
        click("nokta: düğüm 0, toleransın yarısı kadar uzaktan da", 20.25, 0.5),
        click("çok noktalının ikinci noktası", 22.125, 5.125),
        click("yolun ikinci kenarı", 34.25, 2),
        click("yolun dirseği", 34.125, 4.125),
        click("odadaki kapının ucu: bloğun yolu ve yeri", 52.125, 0.875),
        click("odanın üst duvarı: blokta kenar boyunca", 53, 4.25),
        click("bloğun ekleme noktası odanın köşesiyle aynı yerde: bloğunki", 50.125, 0.125),
        click("odanın içi: blok yersiz", 55, 2),
        click("yalnız çizgiler: alan dışarıda sayılır", 5, 5.25, kinds=["line"]),
        click("gizli katmandaki alan: yok", 0.25, 30.25),
        click("kilitli katmandaki çizgi tıklanır", 25, 20.25),
        click("boş yer: yok", 100, 100),
        click("sıfır tolerans, çizginin tam üstü", 5, 5, tolerance=0.0),
        click("plan aynı cevabı verir", 10.25, 9.75, op="plan"),
        case("validate yalnız denetler",
             step({"at": P(0, 0), "tolerance": 0.5}, {"status": "completed", "output": None, "warnings": []},
                  op="validate")),
        case("tıklamanın doğusu sonlu olmalı",
             step({"at": P(0, 0), "tolerance": 0.5},
                  failed("not_finite", f"Tıklamanın doğu (Y) {NOT_FINITE}", "at.x"), nonFinite={"at.x": "Infinity"})),
        case("tolerans sonlu olmalı",
             step({"at": P(0, 0), "tolerance": 0.5},
                  failed("not_finite", "Tolerans sonlu bir sayı değil (NaN ya da sonsuz). Metre olarak bir sayı verin.",
                         "tolerance"), nonFinite={"tolerance": "NaN"})),
        case("eksi tolerans",
             step({"at": P(0, 0), "tolerance": -1},
                  failed("invalid_distance",
                         "Tolerans sıfırdan küçük olamaz: tıklamanın yakınında aranacak uzaklığı metre olarak verin.",
                         "tolerance"))),
    ]


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.entities.pick", "commandVersion": 1,
        "title": "Tıklanan nesneler: en belirliden, alt nesneleriyle",
        "note": "ADR 0276. Noktalar ve kenarlar uzaklıklarıyla (nokta nesnesi 1,5 toleransla), sonra içinde kalınan "
                "alanlar boylarıyla; yalnız gösterilenler, kilitli katmanınkiler de. Alt nesne: toleranstaki en yakın "
                "nokta, yoksa kenarın en yakın noktası (along); blokta parçanın yolu. Çizim değişmez. Bağımsız "
                "başvuru: scripts/fixtures/entities_pick_cases.py (yerler subref_cases.py'nin place_near'ından).",
        "setup": SETUP, "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"tıklanan nesneler durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
