#!/usr/bin/env python3
"""The shared cases of `cad.inserts.clip` (docs/adr/0351 §1; plan/cad.md CAD-REF-12).

    python3 scripts/fixtures/inserts_clip_cases.py           # writes the file
    python3 scripts/fixtures/inserts_clip_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.inserts.clip.json with Python's standard library, no KentOS code, from ADR 0351 §1:

- the boundary is given in the drawing (a rectangle by two opposite corners, along the axes, its corners from the
  lower left counterclockwise; or a polygon, a closing corner the same as the first dropped) and kept in the insert's
  definition: each corner through the inverse of the insert's placement x' = p + R(rotation)·S·M·(x − base)
  (S = diag(scale, yScale), M mirrors in the x axis), computed here with the same floating point steps the ADR gives;
- a boundary has at least three corners, an area, and no two edges meeting but neighbours at their corner (a
  neighbour folding back along the other counts); the pair said is the lowest first number, then the lowest second;
- Remove, Frame and Invert change the clipped inserts named and pass over the others; none clipped is refused;
- inserts on a locked layer are left out and counted, all locked refused; the refusals in the contract's order;
- `clip_empty` when the boundary leaves an insert nothing: here every piece's box lies outside the boundary's box
  (kept inside) or inside it (kept outside), which the cases are drawn so that it decides.
"""
import json
import math
import sys
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.inserts.clip.json"

MASA = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7201"
UZAK = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7202"
UID = "0192f5a1-9999-7000-8000-000000000001"
HALF = math.pi / 2.0
MAX_CORNERS = 10_000


def P(x, y):
    return {"x": float(x), "y": float(y)}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


# The definition: a table top (an area), its diagonal (a path) and its name (a text); base (0, 0).
BLOCKS = [
    {"id": MASA, "name": "Masa", "base": P(0, 0), "entities": [
        {"kind": "polygon", "id": 1, "layerId": "", "attrs": {}, "pts": [P(0, 0), P(2, 0), P(2, 1), P(0, 1)]},
        {"kind": "line", "id": 2, "layerId": "", "attrs": {}, "a": P(0, 0), "b": P(2, 1)},
    ]},
    {"id": UZAK, "name": "Uzak", "base": P(1, 1), "entities": [
        {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": P(1, 1), "b": P(3, 1)},
    ]},
]
# The definition's pieces' boxes (min x, min y, max x, max y), for clip_empty.
BOXES = {MASA: [(0, 0, 2, 1), (0, 0, 2, 1)], UZAK: [(1, 1, 3, 1)]}
BASE = {MASA: (0.0, 0.0), UZAK: (1.0, 1.0)}


def insert(i, at, *, block=MASA, scale=1.0, rotation=0.0, y_scale=None, mirror=False, layer_id="cizim", clipping=None):
    e = {"kind": "insert", "id": i, "layerId": layer_id, "attrs": {}, "block": block, "p": P(*at),
         "scale": float(scale), "rotation": float(rotation)}
    if y_scale is not None:
        e["yScale"] = float(y_scale)
    if mirror:
        e["mirror"] = True
    if clipping is not None:
        e["clipping"] = clipping
    return e


CLIPPED = {"boundary": [P(0, 0), P(1, 0), P(1, 1)], "inverted": True}

ENTITIES = [
    insert(1, (10, 10)),
    insert(2, (20, 10), scale=2.0, rotation=HALF),
    insert(3, (30, 10), y_scale=0.5, mirror=True),
    {"kind": "line", "id": 4, "layerId": "cizim", "attrs": {}, "a": P(0, 0), "b": P(5, 0)},
    insert(5, (40, 10), layer_id="kilitli"),
    insert(6, (50, 10), clipping=CLIPPED),
    insert(7, (60, 10), block=UZAK, rotation=-HALF),
    insert(8, (70, 10), rotation=0.5),
]


def drawing(entities=ENTITIES):
    return {"format": "kentos.document", "version": 1, "name": "Kırp",
            "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 50, "workspace": "cad"},
            "origin": P(0, 0), "layers": [layer("cizim", "Çizim"), layer("kilitli", "Kilitli", locked=True)],
            "activeLayer": "cizim", "entities": entities, "styles": {"items": [], "categories": []},
            "blocks": BLOCKS}


def by_slot(n):
    return next(e for e in ENTITIES if e["id"] == n)


# ── The placement and its inverse (ADR 0351 §1) ───────────────────────────

def turn(a):
    """(sin, cos), exact for the quarter turns."""
    if a == 0.0:
        return 0.0, 1.0
    if a in (HALF, -3.0 * HALF):
        return 1.0, 0.0
    if a in (math.pi, -math.pi):
        return 0.0, -1.0
    if a in (3.0 * HALF, -HALF):
        return -1.0, 0.0
    return math.sin(a), math.cos(a)


def placement(e):
    s, c = turn(e["rotation"])
    k = -1.0 if e.get("mirror") else 1.0
    sx = e["scale"]
    sy = e.get("yScale", sx)
    return [sx * c, sx * s, -sy * k * s, sy * k * c, e["p"]["x"], e["p"]["y"]]


def to_definition(e, corner):
    m = placement(e)
    det = m[0] * m[3] - m[1] * m[2]
    bx, by = BASE[e["block"]]
    x, y = corner[0] - m[4], corner[1] - m[5]
    return P((m[3] * x - m[2] * y) / det + bx, (m[0] * y - m[1] * x) / det + by)


def to_drawing(e, p):
    m = placement(e)
    bx, by = BASE[e["block"]]
    x, y = p[0] - bx, p[1] - by
    return (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])


# ── The boundary's rules ──────────────────────────────────────────────────

def ring_of(boundary):
    if boundary["kind"] == "rectangle":
        a, b = boundary["a"], boundary["b"]
        x0, x1 = min(a["x"], b["x"]), max(a["x"], b["x"])
        y0, y1 = min(a["y"], b["y"]), max(a["y"], b["y"])
        return [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    ring = [(p["x"], p["y"]) for p in boundary["points"]]
    if len(ring) > 1 and ring[0] == ring[-1]:
        ring.pop()
    return ring


def orient(a, b, c):
    """The sign of the turn a → b → c, exactly (fractions)."""
    v = (Fraction(b[0]) - Fraction(a[0])) * (Fraction(c[1]) - Fraction(a[1])) - \
        (Fraction(b[1]) - Fraction(a[1])) * (Fraction(c[0]) - Fraction(a[0]))
    return (v > 0) - (v < 0)


def on_segment(a, b, q):
    return min(a[0], b[0]) <= q[0] <= max(a[0], b[0]) and min(a[1], b[1]) <= q[1] <= max(a[1], b[1])


def meet(a, b, c, d):
    o1, o2, o3, o4 = orient(a, b, c), orient(a, b, d), orient(c, d, a), orient(c, d, b)
    if o1 * o2 < 0 and o3 * o4 < 0:
        return True
    return (o1 == 0 and on_segment(a, b, c)) or (o2 == 0 and on_segment(a, b, d)) or \
        (o3 == 0 and on_segment(c, d, a)) or (o4 == 0 and on_segment(c, d, b))


def crossing(ring):
    n = len(ring)
    for i in range(n):
        for j in range(i + 1, n):
            a, b = ring[i], ring[(i + 1) % n]
            c, d = ring[j], ring[(j + 1) % n]
            if j == i + 1 or (i == 0 and j == n - 1):
                shared, x, y = (b, a, d) if j == i + 1 else (a, b, c)
                dot = (x[0] - shared[0]) * (y[0] - shared[0]) + (x[1] - shared[1]) * (y[1] - shared[1])
                if orient(x, shared, y) == 0 and dot > 0:
                    return i, j
                continue
            if meet(a, b, c, d):
                return i, j
    return None


def area2(ring):
    n = len(ring)
    return sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n))


def boundary_problem(boundary):
    ring = ring_of(boundary)
    if len(ring) < 3:
        return "Kırpma sınırı en az üç köşeli bir çokgendir."
    if len(ring) > MAX_CORNERS:
        return f"Kırpma sınırı en çok {MAX_CORNERS} köşeli olabilir."
    if area2(ring) == 0.0:
        return "Kırpma sınırının alanı yok: köşeleri bir doğru üzerinde ya da üst üste."
    c = crossing(ring)
    if c is not None:
        return f"Kırpma sınırının {c[0] + 1}. ve {c[1] + 1}. kenarı kesişiyor; kendini kesmeyen bir sınır çizin."
    return None


def empty(e, ring, inverted):
    """Whether nothing of the insert is left: every piece's drawing box outside the boundary's box (inside kept), or
    inside it (outside kept)."""
    bx0, by0 = min(p[0] for p in ring), min(p[1] for p in ring)
    bx1, by1 = max(p[0] for p in ring), max(p[1] for p in ring)
    for (x0, y0, x1, y1) in BOXES[e["block"]]:
        corners = [to_drawing(e, (x, y)) for x in (x0, x1) for y in (y0, y1)]
        px0, py0 = min(p[0] for p in corners), min(p[1] for p in corners)
        px1, py1 = max(p[0] for p in corners), max(p[1] for p in corners)
        outside = px1 < bx0 or bx1 < px0 or py1 < by0 or by1 < py0
        inside = bx0 < px0 and px1 < bx1 and by0 < py0 and py1 < by1
        if (inverted and not inside) or (not inverted and not outside):
            return False
    return True


# ── Cases ─────────────────────────────────────────────────────────────────

cases = []


def refused(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


LOCKED_ONE = "“Kilitli” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."


def locked_some(n):
    return {"code": "layer_locked", "path": "uids",
            "message": f"{n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."}


def clip_empty(n):
    return {"code": "clip_empty", "path": "operation",
            "message": f"Kırpma sınırı {n} yerleştirmede çizilecek bir şey bırakmıyor; sınırın içeriğin üstünde olduğundan emin olun."}


def add(name, input_, result, expect=None, op="execute", setup=None, within=None, more=()):
    step = {"op": op, "input": input_, "result": result}
    if within is not None:
        step["within"] = within
    if expect is not None:
        step["expect"] = expect
    elif result["status"] != "completed" or op != "execute":
        step["expect"] = {"canUndo": False, "revision": "same"}
    cases.append({"name": name, "setup": setup or drawing(), "steps": [step, *more]})


def clipping_of(boundary_def, inverted=False, frame="hide"):
    out = {"boundary": boundary_def}
    if inverted:
        out["inverted"] = True
    if frame != "hide":
        out["frame"] = frame
    return out


def set_case(name, slots, boundary, *, inverted=False, frame="hide", locked=0, within=None):
    """Set over `slots` (by the input's order): each named insert's new clipping and the warnings."""
    ring = ring_of(boundary)
    op = {"kind": "set", "boundary": boundary}
    if inverted:
        op["inverted"] = True
    if frame != "hide":
        op["frame"] = frame
    written, fields, gone = [], {}, 0
    for n in sorted(s for s in slots if by_slot(s).get("layerId") != "kilitli"):
        e = by_slot(n)
        c = clipping_of([to_definition(e, p) for p in ring], inverted, frame)
        written.append({"uid": f"$uidOf:{n}", "clipping": c})
        fields[str(n)] = {"clipping": c}
        gone += empty(e, ring, inverted)
    warnings = ([locked_some(locked)] if locked else []) + ([clip_empty(gone)] if gone else [])
    add(name, {"uids": [f"$uidOf:{s}" for s in slots], "operation": op},
        {"status": "completed", "output": {"inserts": written, "revision": "$current"}, "warnings": warnings},
        expect={"canUndo": True, "revision": "changed", "fields": fields}, within=within)


def rect(a, b):
    return {"kind": "rectangle", "a": P(*a), "b": P(*b)}


def poly(*pts):
    return {"kind": "polygon", "points": [P(*p) for p in pts]}


set_case("Dikdörtgenle kırp: sınır yerleştirmenin tanımına taşınır", [1], rect((10, 10), (11, 10.5)))
set_case("Dikdörtgenin köşeleri ters verilse de sol alttan saat yönü tersine", [1], rect((11, 10.5), (10, 10)))
set_case("Çokgenle kırp: kapanış köşesi yazılmaz", [1], poly((10, 10), (12, 10), (11, 11), (10, 10)))
set_case("Döndürülmüş ve ölçekli yerleştirme: sınır tanımda", [2], rect((18, 10), (19, 12)))
set_case("Aynalı ve Y ölçekli yerleştirme", [3], rect((30, 9.75), (31, 10)))
set_case("Tabanı kaydırılmış blok, eksi çeyrek dönüş", [7], rect((60, 9), (61, 10)))
set_case("Dönüşü çeyrek olmayan yerleştirme", [8], poly((70.5, 10.5), (71, 10.5), (70.75, 10.9)), within=1e-9)
set_case("İki yerleştirme tek adımda, çizimin sırasıyla", [2, 1], rect((10, 10), (19, 12)))
set_case("Dışını göster ve çerçeveyi yazdır", [1], rect((10.5, 10.25), (11.5, 10.75)), inverted=True, frame="plot")
set_case("Çerçeve yalnız ekranda", [1], rect((10, 10), (11, 10.5)), frame="show")
set_case("Var olan sınırın yerine yenisi", [6], rect((50, 10), (51, 10.5)))
set_case("Kilitli katmandaki atlanır ve sayılır", [1, 5], rect((10, 10), (11, 10.5)), locked=1)
set_case("Sınır içeriğe değmez: kırpılır, boş kaldığı söylenir", [1], rect((100, 100), (101, 101)))
set_case("Dışı gösterilir ama sınır içeriğin hepsini sarar", [1], rect((0, 0), (100, 100)), inverted=True)

# Remove, Frame, Invert.
c6 = by_slot(6)["clipping"]
add("Kırpmayı kaldır", {"uids": ["$uidOf:6"], "operation": {"kind": "remove"}},
    {"status": "completed", "output": {"inserts": [{"uid": "$uidOf:6"}], "revision": "$current"}, "warnings": []},
    expect={"canUndo": True, "revision": "changed", "fields": {"6": {"clipping": None}}})
add("Kırpılmamış olan geçilir", {"uids": ["$uidOf:1", "$uidOf:6"], "operation": {"kind": "remove"}},
    {"status": "completed", "output": {"inserts": [{"uid": "$uidOf:6"}], "revision": "$current"}, "warnings": []},
    expect={"canUndo": True, "revision": "changed", "fields": {"6": {"clipping": None}, "1": {"clipping": None}}})
framed = {**c6, "frame": "show"}
add("Çerçevenin görünüşü", {"uids": ["$uidOf:6"], "operation": {"kind": "frame", "frame": "show"}},
    {"status": "completed", "output": {"inserts": [{"uid": "$uidOf:6", "clipping": framed}], "revision": "$current"},
     "warnings": []},
    expect={"canUndo": True, "revision": "changed", "fields": {"6": {"clipping": framed}}})
flipped = {"boundary": c6["boundary"]}
add("İçi ve dışı yer değiştirir", {"uids": ["$uidOf:6"], "operation": {"kind": "invert"}},
    {"status": "completed", "output": {"inserts": [{"uid": "$uidOf:6", "clipping": flipped}], "revision": "$current"},
     "warnings": []},
    expect={"canUndo": True, "revision": "changed", "fields": {"6": {"clipping": flipped}}})
add("Kırpılmış olmayanı kaldırmak", {"uids": ["$uidOf:1"], "operation": {"kind": "remove"}},
    refused("not_clipped", "Verilen yerleştirmelerin hiçbiri kırpılmış değil; önce bir kırpma sınırı verin.", "uids"))
add("Kırpılmış olmayanın çerçevesi", {"uids": ["$uidOf:2"], "operation": {"kind": "frame", "frame": "plot"}},
    refused("not_clipped", "Verilen yerleştirmelerin hiçbiri kırpılmış değil; önce bir kırpma sınırı verin.", "uids"))

# Refusals in the contract's order.
add("Boş kimlik listesi", {"uids": [], "operation": {"kind": "remove"}},
    refused("no_entities", "Kırpılacak blok ya da dış başvuru verilmedi. En az bir nesnenin kalıcı kimliğini verin.", "uids"))
add("Geçersiz kimlik", {"uids": ["masa-1"], "operation": {"kind": "remove"}},
    refused("invalid_uid", "“masa-1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.", "uids[0]"))
add("Çizimde olmayan nesne", {"uids": [UID], "operation": {"kind": "remove"}},
    refused("entity_not_found", f"“{UID}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.", "uids[0]"))
add("Yerleştirme olmayan nesne", {"uids": ["$uidOf:1", "$uidOf:4"], "operation": {"kind": "remove"}},
    {"status": "failed", "error": {"code": "not_an_insert", "message": "$any", "path": "uids[1]"}})
add("Hepsi kilitli katmanda", {"uids": ["$uidOf:5"], "operation": {"kind": "set", "boundary": rect((0, 0), (1, 1))}},
    refused("layer_locked", LOCKED_ONE, "uids[0]"))
for name, boundary in [
    ("İki köşeli sınır", poly((10, 10), (11, 11))),
    ("Kapanış köşesiyle yine iki köşe", poly((10, 10), (11, 11), (10, 10))),
    ("Alanı olmayan dikdörtgen", rect((10, 10), (12, 10))),
    ("Bir doğru üzerindeki köşeler", poly((0, 0), (1, 1), (2, 2))),
    ("Kendini kesen sınır (papyon)", poly((10, 10), (14, 14), (14, 10), (10, 12))),
    ("Geri dönen sivri uç", poly((10, 10), (14, 10), (12, 10), (12, 13))),
    ("Kendine köşede değen sınır", poly((10, 10), (14, 10), (12, 12), (14, 14), (10, 14), (12, 12))),
    ("Kesişen uzak kenarlar", poly((0, 0), (6, 0), (6, 4), (2, 4), (2, -2), (1, -2), (1, 5), (0, 5))),
]:
    add(f"Geçersiz sınır: {name}", {"uids": ["$uidOf:1"], "operation": {"kind": "set", "boundary": boundary}},
        refused("invalid_clipping", boundary_problem(boundary), "operation.boundary"))
add("Kilitli katman sınırdan önce söylenir", {"uids": ["$uidOf:5"], "operation": {"kind": "set", "boundary": poly((0, 0), (1, 1))}},
    refused("layer_locked", LOCKED_ONE, "uids[0]"))

# Plan, validation, revision, undo.
ring = ring_of(rect((10, 10), (11, 10.5)))
planned = clipping_of([to_definition(by_slot(1), p) for p in ring])
add("Plan yazmadan ne olacağını söyler", {"uids": ["$uidOf:1"], "operation": {"kind": "set", "boundary": rect((10, 10), (11, 10.5))}},
    {"status": "completed", "output": {"inserts": [{"uid": "$uidOf:1", "clipping": planned}], "revision": "$current"},
     "warnings": []}, op="plan", expect={"canUndo": False, "revision": "same", "fields": {"1": {"clipping": None}}})
cases.append({"name": "Doğrulama", "setup": drawing(), "steps": [
    {"op": "validate", "input": {"uids": ["$uidOf:6"], "operation": {"kind": "invert"}},
     "result": {"status": "completed", "output": None, "warnings": []}}]})
add("Geçersiz sürüm", {"uids": ["$uidOf:6"], "operation": {"kind": "invert"}, "expectedRevision": "x"},
    {"status": "failed", "error": {"code": "invalid_revision", "message": "$any", "path": "expectedRevision"}})
add("Çizim bu arada değişti", {"uids": ["$uidOf:6"], "operation": {"kind": "invert"}, "expectedRevision": "999"},
    {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}})
add("Tek adımda geri alınır", {"uids": ["$uidOf:1"], "operation": {"kind": "set", "boundary": rect((10, 10), (11, 10.5))}},
    {"status": "completed", "output": {"inserts": [{"uid": "$uidOf:1", "clipping": planned}], "revision": "$current"},
     "warnings": []}, expect={"canUndo": True, "revision": "changed", "fields": {"1": {"clipping": planned}}},
    more=[{"op": "undo", "expect": {"canUndo": False, "fields": {"1": {"clipping": None}}}}])


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build():
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.inserts.clip",',
           '  "commandVersion": 1,', '  "title": "Kırp: blok ve dış başvuru yerleştirmelerinin kırpma sınırı",',
           f'  "note": {compact("Written by scripts/fixtures/inserts_clip_cases.py from docs/adr/0351 §1; do not edit by hand.")},',
           f'  "setup": {compact(drawing())},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},', f'      "setup": {compact(c["setup"])},',
                 '      "steps": [', ",\n".join(f"        {compact(s)}" for s in c["steps"]), "      ]", "    }"]
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.extend(["  ]", "}"])
    return "\n".join(out) + "\n"


def main():
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


main()
