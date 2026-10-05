"""Independent reference of Veri karşılaştır's rule (docs/adr/0179), and the cases both platforms play.

Two sets of objects, old and new, are paired and compared:

- Each object is taken apart into its defining points, its lengths and its other values. Points come as fixed lists (a
  point's and its parts', a circle's centre, an arc's centre and ends, a text's or an insert's place), as paths (a line,
  a polyline and its parts, a leader, an open spline: direction free) and as rings (an area's ring, holes and parts, a
  hatch's, a closed spline's: start and direction free). A path or ring has its vertices and the middle of each edge: the
  arc's middle on a bulged edge, the chord's on a straight one.
- The location difference of two objects of the same kind and structure (the same lists, paths and rings with as many
  points) is the largest distance between their matching points, taking each path's and ring's best alignment; with
  another structure it is the distance between the centres of the boxes around their points. Objects of other kinds
  have none.
- The geometry is the same when the kind and structure are, the location difference is within the tolerance, the
  lengths differ by no more than the tolerance and the other values are equal.
- Paired by location: candidates of the same kind within the search distance, taken from the smallest difference up
  (ties: the old object's order, then the new one's), each object once. Paired by key: the same trimmed value of the key
  attribute; an object without one, or with one its side repeats, is a key problem.
- The attributes compared are both objects' fields in the order of their characters' codes (a drawing keeps no order of
  its own: the desktop's are sorted, a file's may be), less the ignored ones and, paired by key, the key field; a missing
  field is empty.

The rows follow the new set's order (its pair, added or a key problem), then the old set's unpaired objects in theirs
(removed or a key problem).

    python3 scripts/fixtures/compare_cases.py           # writes fixtures/compare/v1/cases.json
    python3 scripts/fixtures/compare_cases.py --check   # writes nothing; compares

The rule is written here again, from the ADR, not from either platform's code. Numbers are mpmath's at 60 digits:
the inputs' binary values and every sum and product of them are exact there; an arc's ends come from its angles.
"""

import json
import sys
from pathlib import Path

import mpmath as mp

mp.mp.dps = 60

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/compare/v1/cases.json"


def P(v):
    return (mp.mpf(v["x"]), mp.mpf(v["y"]))


def mid(a, b, bulge):
    """An edge's middle: the chord's, moved by the sagitta to the right of a→b for a counter-clockwise (positive) bulge."""
    mx, my = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
    if not bulge:
        return (mx, my)
    dx, dy = b[0] - a[0], b[1] - a[1]
    k = mp.mpf(bulge) / 2
    return (mx + dy * k, my - dx * k)


def edges(pts, bulges, closed):
    n = len(pts)
    count = n if closed else n - 1
    bs = list(bulges or [])
    return [mid(pts[i], pts[(i + 1) % n], bs[i] if i < len(bs) else 0) for i in range(count)]


def path(pts, bulges=None):
    p = [P(q) for q in pts]
    return ("path", p, edges(p, bulges, False))


def ring(pts, bulges=None):
    p = [P(q) for q in pts]
    return ("ring", p, edges(p, bulges, True))


def ring_of(r):
    return ring(r["pts"], r.get("bulges"))


def defs(shape):
    """The shape taken apart: its components, its lengths, its other values."""
    k = shape["kind"]
    comps, lengths, others = [], [], []
    if k == "point":
        comps = [("fixed", [P(shape["p"])] + [P(q["p"]) for q in shape.get("parts") or []], None)]
    elif k == "line":
        comps = [path([shape["a"], shape["b"]])]
    elif k == "polyline":
        comps = [path(shape["pts"], shape.get("bulges"))] + [path(q["pts"], q.get("bulges")) for q in shape.get("parts") or []]
    elif k == "polygon":
        comps = [ring(shape["pts"], shape.get("bulges"))] + [ring_of(h) for h in shape.get("holes") or []]
        for part in shape.get("parts") or []:
            comps += [ring(part["pts"], part.get("bulges"))] + [ring_of(h) for h in part.get("holes") or []]
    elif k == "circle":
        comps = [("fixed", [P(shape["c"])], None)]
        lengths = [mp.mpf(shape["r"])]
    elif k == "arc":
        c, r = P(shape["c"]), mp.mpf(shape["r"])
        a0, a1 = mp.mpf(shape["a0"]), mp.mpf(shape["a1"])
        comps = [("fixed", [c, (c[0] + r * mp.cos(a0), c[1] + r * mp.sin(a0)), (c[0] + r * mp.cos(a1), c[1] + r * mp.sin(a1))], None)]
        lengths = [r]
    elif k == "ellipse":
        c = P(shape["c"])
        m = P(shape["major"])
        comps = [("fixed", [c, (c[0] + m[0], c[1] + m[1])], None)]
        others = [shape["ratio"], shape["t0"], shape["t1"]]
    elif k == "spline":
        comps = [ring(shape["pts"]) if shape["closed"] else path(shape["pts"])]
        others = [shape["closed"]]
    elif k in ("xline", "ray"):
        comps = [("fixed", [P(shape["p"])], None)]
        others = [shape["dir"]["x"], shape["dir"]["y"]]
    elif k == "text":
        comps = [("fixed", [P(shape["p"])], None)]
        lengths = [mp.mpf(shape["height"])]
        # A width factor of 1 is no factor, as a file writes it.
        others = [shape["text"], shape["rotation"], shape.get("align"), None if shape.get("widthFactor") in (None, 1) else shape["widthFactor"]]
    elif k == "dimension":
        comps = [("fixed", [P(shape["a"]), P(shape["b"])] + ([P(shape["c"])] if shape.get("c") else []), None)]
        lengths = [mp.mpf(shape["offset"]), mp.mpf(shape["height"])]
        others = [shape.get("text"), shape.get("style"), shape.get("angle")]
    elif k == "hatch":
        comps = [ring(shape["ring"])] + [ring(h) for h in shape.get("holes") or []]
        others = [shape["pattern"]]
    elif k == "insert":
        comps = [("fixed", [P(shape["p"])], None)]
        # Not mirrored is no `mirror`, as a file writes it.
        others = [shape["block"], shape["scale"], shape["rotation"], True if shape.get("mirror") else None]
    elif k == "leader":
        comps = [path(shape["pts"])]
        lengths = [mp.mpf(shape["height"])]
        others = [shape.get("text"), shape["rotation"], shape.get("arrow")]
    else:
        raise ValueError(k)
    return comps, lengths, others


def d2(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2


def best(ca, cb):
    """A component pair's largest squared distance at its best alignment, or None when their shapes differ."""
    ta, pa, ma = ca
    tb, pb, mb = cb
    if ta != tb or len(pa) != len(pb):
        return None
    n = len(pa)
    if ta == "fixed":
        return max(d2(pa[i], pb[i]) for i in range(n)) if n else mp.mpf(0)
    options = []
    if ta == "path":
        e = len(ma)
        options.append(([*range(n)], [*range(e)]))
        options.append(([n - 1 - i for i in range(n)], [e - 1 - i for i in range(e)]))
    else:
        for k in range(n):
            options.append(([(k + i) % n for i in range(n)], [(k + i) % n for i in range(n)]))
            # Backwards: vertex i to k − i, and edge i (i → i + 1) to the edge from k − i − 1 to k − i.
            options.append(([(k - i) % n for i in range(n)], [(k - i - 1) % n for i in range(n)]))
    worst = None
    for vs, es in options:
        m = max([d2(pa[i], pb[vs[i]]) for i in range(n)] + [d2(ma[i], mb[es[i]]) for i in range(len(ma))])
        if worst is None or m < worst:
            worst = m
    return worst


def centre(comps):
    pts = [p for _, ps, ms in comps for p in ps + (ms or [])]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)


def difference(a, b):
    """(squared location difference, same structure) of two members' shapes, or None for other kinds."""
    if a["shape"]["kind"] != b["shape"]["kind"]:
        return None
    ca, cb = defs(a["shape"])[0], defs(b["shape"])[0]
    if len(ca) == len(cb):
        parts = [best(x, y) for x, y in zip(ca, cb)]
        if all(p is not None for p in parts):
            return (max(parts) if parts else mp.mpf(0), True)
    return (d2(centre(ca), centre(cb)), False)


def same_geometry(a, b, tol):
    diff = difference(a, b)
    if diff is None or not diff[1] or diff[0] > tol * tol:
        return False
    _, la, oa = defs(a["shape"])
    _, lb, ob = defs(b["shape"])
    return all(abs(x - y) <= tol for x, y in zip(la, lb)) and oa == ob


def changed_fields(a, b, ignore):
    names = sorted(set(a["attrs"]) | set(b["attrs"]))
    return [k for k in names if k not in ignore and a["attrs"].get(k, "") != b["attrs"].get(k, "")]


def compare(old, new, s):
    tol, search = mp.mpf(s["tolerance"]), mp.mpf(s["search"])
    pairs = {}  # new index → old index
    problems_old, problems_new = set(), set()
    if s["match"] == "key":
        def keys(side):
            out = {}
            for i, m in enumerate(side):
                out.setdefault(m["attrs"].get(s["key"], "").strip(), []).append(i)
            return out

        ko, kn = keys(old), keys(new)
        # A key is bad when it is empty or either side repeats it: every object with it is a key problem.
        bad = {k for k in set(ko) | set(kn) if not k or len(ko.get(k, [])) > 1 or len(kn.get(k, [])) > 1}
        for key, idx in ko.items():
            if key in bad:
                problems_old.update(idx)
        for key, idx in kn.items():
            if key in bad:
                problems_new.update(idx)
            elif key in ko:
                pairs[idx[0]] = ko[key][0]
    else:
        cands = []
        for i, a in enumerate(old):
            for j, b in enumerate(new):
                diff = difference(a, b)
                if diff is not None and diff[0] <= search * search:
                    cands.append((diff[0], i, j))
        cands.sort(key=lambda c: (c[0], c[1], c[2]))
        taken = set()
        for _, i, j in cands:
            if i not in taken and j not in pairs:
                pairs[j] = i
                taken.add(i)
    rows = []
    paired_old = set(pairs.values())
    for j, b in enumerate(new):
        if j in problems_new:
            rows.append({"status": "key", "old": None, "new": j, "distance": None, "fields": []})
        elif j in pairs:
            i = pairs[j]
            a = old[i]
            diff = difference(a, b)
            geometry = not same_geometry(a, b, tol)
            # Paired by key, the key field is the same by its rule (trimmed): it is not a change.
            ignore = set(s.get("ignore", [])) | ({s["key"]} if s["match"] == "key" else set())
            fields = changed_fields(a, b, ignore)
            status = {(False, False): "same", (True, False): "geometry", (False, True): "attributes", (True, True): "both"}[
                (geometry, bool(fields))]
            rows.append({"status": status, "old": i, "new": j,
                         "distance": None if diff is None else float(mp.sqrt(diff[0])), "fields": fields})
        else:
            rows.append({"status": "added", "old": None, "new": j, "distance": None, "fields": []})
    for i, _ in enumerate(old):
        if i in problems_old:
            rows.append({"status": "key", "old": i, "new": None, "distance": None, "fields": []})
        elif i not in paired_old:
            rows.append({"status": "removed", "old": i, "new": None, "distance": None, "fields": []})
    return rows


# ── The cases ─────────────────────────────────────────────────────────────

def xy(x, y):
    return {"x": x, "y": y}


def member(shape, **attrs):
    return {"shape": shape, "attrs": attrs}


def square(x, y, s, bulges=None):
    shape = {"kind": "polygon", "pts": [xy(x, y), xy(x + s, y), xy(x + s, y + s), xy(x, y + s)]}
    if bulges:
        shape["bulges"] = bulges
    return shape


def parcels():
    """Ada 120's parcels before and after an update, a road line, a point and a text."""
    old = [
        member(square(487000, 4420000, 20), No="1", Malik="Ali"),
        member(square(487020, 4420000, 20), No="2", Malik="Ayşe"),
        member(square(487040, 4420000, 20), No="3", Malik="Veli"),
        member(square(487060, 4420000, 20), No="4", Malik="Can"),
        member({"kind": "line", "a": xy(487000, 4419990), "b": xy(487080, 4419990)}, No="Y1"),
        member({"kind": "point", "p": xy(487100, 4420000)}, No="N1"),
        member({"kind": "text", "p": xy(487005, 4420005), "text": "120/1", "height": 1.5, "rotation": 0}, No="T1"),
        member({"kind": "circle", "c": xy(487110, 4420010), "r": 2.0}, No="K1"),
    ]
    moved = square(487020, 4420000, 20)
    moved["pts"][2] = xy(487040.4, 4420020)  # a corner moved 0.4 m
    shifted = square(487040.0004, 4420000.0003, 20)  # within a millimetre
    new = [
        member(square(487000, 4420000, 20), No="1", Malik="Ali"),
        member(moved, No="2", Malik="Ayşe"),
        member(shifted, No="3", Malik="Veli Can"),
        # Parcel 4 is gone; a new one stands east of it.
        member(square(487200, 4420000, 20), No="5", Malik="Ece"),
        # The road line drawn the other way round.
        member({"kind": "line", "a": xy(487080, 4419990), "b": xy(487000, 4419990)}, No="Y1"),
        member({"kind": "point", "p": xy(487100.0005, 4420000)}, No="N1", Kod="PN"),
        member({"kind": "text", "p": xy(487005, 4420005), "text": "120/1A", "height": 1.5, "rotation": 0}, No="T1"),
        member({"kind": "circle", "c": xy(487110, 4420010), "r": 2.5}, No="K1"),
    ]
    return old, new


def rings():
    """Rings started elsewhere and turned round, a bulged edge, a hole, and a vertex more."""
    base = square(0, 0, 10)
    turned = {"kind": "polygon", "pts": [xy(10, 10), xy(10, 0), xy(0, 0), xy(0, 10)]}
    bulged = square(20, 0, 10, [0, 0.5, 0, 0])
    bulged_less = square(20, 0, 10, [0, 0.4, 0, 0])
    holed = square(40, 0, 10)
    holed["holes"] = [{"pts": [xy(42, 2), xy(44, 2), xy(44, 4), xy(42, 4)]}]
    holed_moved = square(40, 0, 10)
    holed_moved["holes"] = [{"pts": [xy(43, 2), xy(45, 2), xy(45, 4), xy(43, 4)]}]
    five = {"kind": "polygon", "pts": [xy(60, 0), xy(70, 0), xy(70, 10), xy(65, 12), xy(60, 10)]}
    four = square(60, 0, 10)
    arc_a = {"kind": "arc", "c": xy(100, 0), "r": 5, "a0": 0, "a1": 1.5707963267948966}
    arc_b = {"kind": "arc", "c": xy(100, 0), "r": 5, "a0": 0.0001, "a1": 1.5707963267948966}
    insert_a = {"kind": "insert", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1001", "p": xy(120, 0), "scale": 1, "rotation": 0}
    insert_b = {"kind": "insert", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1001", "p": xy(120, 0), "scale": 1, "rotation": 0.5}
    old = [member(base, Ad="A"), member(bulged, Ad="B"), member(holed, Ad="C"), member(four, Ad="D"), member(arc_a, Ad="E"),
           member(insert_a, Ad="F")]
    new = [member(turned, Ad="A"), member(bulged_less, Ad="B"), member(holed_moved, Ad="C"), member(five, Ad="D"),
           member(arc_b, Ad="E"), member(insert_b, Ad="F")]
    return old, new


def keyed():
    """Paired by key: a repeated key, an empty key, kinds that differ, and an ignored field."""
    old = [
        member(square(0, 0, 10), ParselNo="10", Tarih="2024"),
        member(square(20, 0, 10), ParselNo="11", Tarih="2024"),
        member(square(40, 0, 10), ParselNo="12", Tarih="2024"),
        member(square(60, 0, 10), ParselNo="12", Tarih="2024"),
        member(square(80, 0, 10), ParselNo=" ", Tarih="2024"),
    ]
    new = [
        member(square(500, 0, 10), ParselNo="10", Tarih="2025"),
        member({"kind": "point", "p": xy(25, 5)}, ParselNo=" 11 ", Tarih="2025"),
        member(square(40, 0, 10), ParselNo="12", Tarih="2025"),
        member(square(100, 0, 10), ParselNo="13", Tarih="2025"),
        member(square(120, 0, 10), Tarih="2025"),
    ]
    return old, new


def cases():
    out = []
    old, new = parcels()
    for name, s in [
        ("Konumla, 1 m arama, 1 mm tolerans: köşesi kayan, milimetre altı kayan, silinen ve eklenen parsel; ters çizilmiş yol; "
         "kayan nokta ve öznitelik; metni değişen yazı; yarıçapı değişen daire", {"match": "location", "search": 1.0, "tolerance": 0.001}),
        ("Konumla, 0,3 m arama: köşesi 0,4 m kayan parsel artık eş değil, silinen ve eklenen olur",
         {"match": "location", "search": 0.3, "tolerance": 0.001}),
        ("Anahtar alanla (No) ve Malik karşılaştırılmadan", {"match": "key", "key": "No", "search": 1.0, "tolerance": 0.001,
                                                            "ignore": ["Malik"]}),
    ]:
        out.append({"name": name, "old": old, "new": new, "settings": s, "rows": compare(old, new, s)})
    old, new = rings()
    s = {"match": "key", "key": "Ad", "search": 1.0, "tolerance": 0.001}
    out.append({"name": "Halkalar: başka köşeden ve ters yönde çizilmiş aynı alan; yaylı kenarın bükümü; delik; bir köşe fazla; "
                        "yayın ucu; bloğun dönüklüğü", "old": old, "new": new, "settings": s, "rows": compare(old, new, s)})
    old, new = keyed()
    s = {"match": "key", "key": "ParselNo", "search": 1.0, "tolerance": 0.001, "ignore": ["Tarih"]}
    out.append({"name": "Anahtar alanla: yinelenen ve boş anahtar sorunlu, türü değişen eş, uzaklaşan eş, Tarih karşılaştırılmaz",
                "old": old, "new": new, "settings": s, "rows": compare(old, new, s)})
    return {"format": "kentos.compare-cases", "version": 1,
            "note": "Written by scripts/fixtures/compare_cases.py from docs/adr/0179, not from either platform's code. A row's "
                    "`old` and `new` are indices into the two lists; `distance` is the location difference in metres (null: "
                    "none); `fields` the changed attributes. Statuses: same, geometry, attributes, both, added, removed, key.",
            "cases": out}


def main():
    text = json.dumps(cases(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 scripts/fixtures/compare_cases.py ile yeniden yazın")
            sys.exit(1)
        print(f"Veri karşılaştırma durumları tutarlı: {OUT.relative_to(ROOT)}")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
