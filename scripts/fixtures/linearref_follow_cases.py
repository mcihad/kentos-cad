#!/usr/bin/env python3
"""Writes fixtures/commands/v1/cad.linearref.follow.json: the shared cases of the events a project's link keeps on their
routes (docs/adr/0569 §3), their expectations built here from the ADR's rules, without KentOS code.

A write by any command that touches objects of a link's event layer or of one of its route layers (added, removed or
changed one) is followed, in its own step, by the placing again of the event objects of the link's kind it reached
(docs/adr/0641 §2): the event objects it wrote, and the events of every route whose id an object of the route layers
held before or after the write; an event it did not reach keeps its place. The event's own attributes say its route, its measures and its offset; the route's place for them is the one the routes' rules give
(scripts/fixtures/lrs_cases.py: 50 digits, no KentOS code: `point_at`, `stretch`); an event lying further from it than
the link's tolerance moves there (a point to the point, a polyline to the route's parts, a line to the route's one
straight part). An event that cannot be placed (its route is gone, a measure is not a number or lies outside the route,
a line would need a bent path, the layer is locked) stays and the answer carries `events_dirty`. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/linearref_follow_cases.py           # writes the file
    python3 scripts/fixtures/linearref_follow_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

import mpmath as mp

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.dont_write_bytecode = True
import lrs_cases as L  # noqa: E402
import lrs_processing_cases as D  # noqa: E402

ROOT = HERE.parents[1]
OUT = ROOT / "fixtures/commands/v1/cad.linearref.follow.json"
OX, OY = 500000.0, 4400000.0
WITHIN = 1e-6
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}

FAULT_WORDS = {
    "outside": "ölçü rotanın dışında",
    "noMeasures": "rotanın ölçüsü yok",
    "same": "başlangıç ve bitiş ölçüsü aynı",
    "offset": "öteleme yapılamadı",
    "noRoute": "rota yok",
    "noMeasure": "ölçü yok",
    "fewPoints": "ikiden az nokta",
}

POINTS = {"id": "kazalar", "name": "Kazalar", "events": "kaza", "kind": "point", "routes": ["yol", "yol-eski"],
          "routeField": "Yol", "eventRoute": "Yol", "measure": "Km", "offset": "Sapma"}
LINES = {"id": "kesimler", "name": "Kaplama kesimleri", "events": "kesim", "kind": "line", "routes": ["yol", "yol-eski"],
         "routeField": "Yol", "eventRoute": "Yol", "from": "Bas", "to": "Son", "offset": "Sapma", "side": "left", "tolerance": 0.01}


def node(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True, "style": dict(STYLE), "children": []}


def pt(x, y):
    return {"x": OX + x, "y": OY + y}


def line(id_, layer, a, b, attrs, ma=None, mb=None):
    e = {"kind": "line", "id": id_, "layerId": layer, "attrs": attrs, "a": pt(*a), "b": pt(*b)}
    if ma is not None:
        e["ma"], e["mb"] = ma, mb
    return e


def point(id_, layer, p, attrs):
    return {"kind": "point", "id": id_, "layerId": layer, "attrs": attrs, "p": pt(*p)}


def polyline(id_, layer, pts, attrs, ms=None):
    e = {"kind": "polyline", "id": id_, "layerId": layer, "attrs": attrs, "pts": [pt(*p) for p in pts]}
    if ms is not None:
        e["ms"] = ms
    return e


def entities():
    """The drawing: two routes and the events, every event in the place its route gives it."""
    return [
        line(1, "yol", (0, 0), (100, 0), {"Yol": "D-100"}, 0.0, 100.0),
        polyline(2, "yol-eski", [(0, 20), (60, 20), (60, 80)], {"Yol": "E-1"}, [0.0, 60.0, 120.0]),
        point(3, "kaza", (40, -2), {"Yol": "D-100", "Km": "40", "Sapma": "2"}),
        point(4, "kaza", (60, 50), {"Yol": "E-1", "Km": "90"}),
        point(5, "kaza", (10, 0), {"Yol": "D-100", "Km": "10", "Sapma": ""}),
        line(6, "kesim", (10, 1), (30, 1), {"Yol": "D-100", "Bas": "10", "Son": "30", "Sapma": "1"}, 10.0, 30.0),
        polyline(7, "kesim", [(30, 20), (60, 20), (60, 50)], {"Yol": "E-1", "Bas": "30", "Son": "90"}, [30.0, 60.0, 90.0]),
        point(8, "diger", (500, 500), {"Not": "başka"}),
        line(9, "kaza", (0, 90), (5, 90), {"Not": "bir çizgi nesnesi"}),
    ]


def setup(name, links=(POINTS, LINES), locked=(), mutate=None):
    layers = [node("yol", "Yol"), node("yol-eski", "Eski yol"), node("kaza", "Kaza", "kaza" in locked), node("kesim", "Kesim"), node("diger", "Diğer"), node("0", "0")]
    doc = {
        "format": "kentos.document", "version": 1, "name": "Olay bağları",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow",
                     **({"linearEvents": copy.deepcopy(list(links))} if links else {})},
        "origin": {"x": OX, "y": OY},
        "layers": layers, "activeLayer": "0", "entities": entities(), "styles": {"items": [], "categories": []},
    }
    if mutate:
        mutate(doc)
    return doc


# ── The rules, from the ADR ──────────────────────────────────────────────


def trim(t):
    return D.js_trim(t)


def route_parts(e):
    return D.route_parts(e)


def read(e, link):
    """An event's own attributes: the route's id and the numbers, or why they give none (the cell, in the table's words)."""
    attrs = e["attrs"]
    cell = lambda f: attrs.get(f, "")  # noqa: E731
    rid = trim(cell(link["eventRoute"]))
    fields = [link["from"], link["to"]] if link["kind"] == "line" else [link["measure"]]
    nums = []
    for f in fields:
        v = D.read_number(cell(f))
        if v is None:
            return rid, None, None, D.cell_reason(f, cell(f))
        nums.append(v)
    off = 0.0
    if link.get("offset") and trim(cell(link["offset"])):
        off = D.read_number(cell(link["offset"]))
        if off is None:
            return rid, None, None, D.cell_reason(link["offset"], cell(link["offset"]))
    return rid, nums, off * (-1 if link.get("side") == "left" else 1), None


def routes_of(world, link):
    """The first object of an id, over the link's route layers in their order."""
    by_id = {}
    for layer in link["routes"]:
        for e in world["entities"]:
            if e["layerId"] != layer or e["kind"] not in ("line", "polyline"):
                continue
            rid = trim(e["attrs"].get(link["routeField"], ""))
            if rid and rid not in by_id:
                by_id[rid] = e
    return by_id


def shape_of(e, got, line_kind):
    """The object moved to its place, of its own kind; none when a line cannot take a bent path."""
    if not line_kind:
        return {"p": {"x": float(got["p"][0]), "y": float(got["p"][1])}}
    placed = got["parts"]
    if e["kind"] == "line":
        if len(placed) != 1 or len(placed[0]["pts"]) != 2 or any(b != 0 for b in placed[0]["bulges"]):
            return None
        (a, b), ms = placed[0]["pts"], placed[0]["ms"]
        return {"a": {"x": float(a[0]), "y": float(a[1])}, "b": {"x": float(b[0]), "y": float(b[1])}, "ma": D.num(ms[0]), "mb": D.num(ms[1])}
    first, *rest = [D.part_out({"pts": q["pts"], "bulges": q["bulges"], "ms": q["ms"]}) for q in placed]
    out = dict(first)
    if rest:
        out["parts"] = rest
    return out


def reached_of(link, written):
    """The route ids a write's objects held on the link's route layers (before or after it) and the event objects it
    left on the event layer."""
    routes, events = set(), set()
    for before, after in written:
        for e in (before, after):
            if e is not None and e["layerId"] in link["routes"]:
                rid = trim(e["attrs"].get(link["routeField"], ""))
                if rid:
                    routes.add(rid)
        if after is not None and after["layerId"] == link["events"]:
            events.add(after["id"])
    return routes, events


def follow_link(world, link, reached):
    """The events of one link the write reached placed again: the moves (id → shape fields) and the events left, with
    the first reason."""
    line_kind = link["kind"] == "line"
    routes = routes_of(world, link)
    moves, notes = {}, []
    locked = next(n for n in world["layers"] if n["id"] == link["events"])["locked"]
    route_ids, event_ids = reached
    for e in world["entities"]:
        if e["layerId"] != link["events"]:
            continue
        if e["kind"] == "point" and line_kind or e["kind"] not in (("line", "polyline") if line_kind else ("point",)):
            continue
        rid, nums, off, why = read(e, link)
        if e["id"] not in event_ids and rid not in route_ids:
            continue
        if why is None and rid not in routes:
            why = f"“{rid}” rotası yok"
        got = None
        if why is None:
            route = L.route_of(route_parts(routes[rid]))
            got = L.stretch(route, nums[0], nums[1], off) if line_kind else L.point_at(route, nums[0], off)
            if "fault" in got:
                why = FAULT_WORDS[got["fault"]]
        if why is not None:
            notes.append(f"#{e['id']}: {why}")
            continue
        if line_kind:
            placed = [(q["pts"], q["bulges"], q["ms"]) for q in got["parts"]]
            drift = D.line_drift(L.route_of(route_parts(e)), placed)
        else:
            drift = L.dist(L.P(e["p"]["x"], e["p"]["y"]), got["p"])
        if drift <= mp.mpf(link.get("tolerance", 0.001)):
            continue
        shape = shape_of(e, got, line_kind)
        if shape is None:
            notes.append(f"#{e['id']}: çizgi nesnesi kırıklı yola taşınamaz")
        else:
            moves[e["id"]] = shape
    left, first = len(notes), (notes[0] if notes else None)
    if moves and locked:
        # The layer is locked: nothing moves; the first reason is the first note, else this.
        left += len(moves)
        first = first or "olay katmanı kilitli"
        moves = {}
    return moves, left, first


def dirty(name, left, first):
    return {"code": "events_dirty",
            "message": f"“{name}” bağı olayları rotalarında tutuyor ama bu yazma {left} olayı yerine koyamadı: {first}. Olay kaymasını denetle nedenleri tek tek söyler."}


def follow(world, written):
    """The world after the follow of a write's objects (each before and after it), and the warnings."""
    world = copy.deepcopy(world)
    warnings = []
    by_id = {e["id"]: e for e in world["entities"]}
    touched = {e["layerId"] for pair in written for e in pair if e is not None}
    for link in world["settings"].get("linearEvents", []):
        if not any(layer in touched for layer in [link["events"]] + link["routes"]):
            continue
        moves, left, first = follow_link(world, link, reached_of(link, written))
        for i, shape in moves.items():
            by_id[i].update(shape)
        if left:
            warnings.append(dirty(link["name"], left, first))
    return world, warnings


# ── The user's writes ────────────────────────────────────────────────────


def apply_change(world, change):
    """The drawing after the user's write (before the follow), the command and its input, and the objects it wrote,
    each before and after it (None: not there)."""
    s = copy.deepcopy(world)
    by_id = {e["id"]: e for e in s["entities"]}
    kind = change[0]
    if kind == "attrs":
        _, i, attrs = change
        before = copy.deepcopy(by_id[i])
        for k, v in attrs.items():
            by_id[i]["attrs"][k] = v
        return s, "cad.entities.set", {"uids": [f"$uidOf:{i}"], "attrs": attrs, "operation": "attributes"}, [(before, by_id[i])]
    if kind == "delete":
        _, i = change
        before = by_id[i]
        s["entities"] = [e for e in s["entities"] if e["id"] != i]
        return s, "cad.entities.delete", {"uids": [f"$uidOf:{i}"]}, [(before, None)]
    if kind == "move":
        _, i, dx, dy = change
        e = by_id[i]
        before = copy.deepcopy(e)
        for key in ("a", "b", "p"):
            if key in e:
                e[key] = {"x": e[key]["x"] + dx, "y": e[key]["y"] + dy}
        if "pts" in e:
            e["pts"] = [{"x": p["x"] + dx, "y": p["y"] + dy} for p in e["pts"]]
        return s, "cad.entities.transform", {"uids": [f"$uidOf:{i}"], "transform": {"kind": "move", "dx": dx, "dy": dy}}, [(before, e)]
    if kind == "point":
        _, layer, p, attrs = change
        new = max(by_id) + 1
        s["entities"].append(point(new, layer, p, attrs))
        return s, "cad.point.create", {"layerId": layer, "p": pt(*p), "attrs": attrs}, [(None, s["entities"][-1])]
    raise ValueError(kind)


FIELDS = {"point": ("p",), "line": ("a", "b", "ma", "mb"), "polyline": ("pts", "ms", "parts", "bulges")}


def event_fields(world):
    """Every event object's shape, by id (the layers of the links); what the case expects after a step."""
    out = {}
    for e in world["entities"]:
        if e["layerId"] in ("kaza", "kesim"):
            out[str(e["id"])] = {k: e[k] for k in FIELDS[e["kind"]] if k in e}
    # The route objects too: a write must leave the routes as the user wrote them.
    for e in world["entities"]:
        if e["layerId"] in ("yol", "yol-eski"):
            out[str(e["id"])] = {k: e[k] for k in (("a", "b", "ma", "mb") if e["kind"] == "line" else ("pts", "ms")) if k in e}
    return out


def case(name, world, changes, undo=False, redo=False):
    """`changes`: the user's writes in order; each is followed."""
    steps = []
    states = [world]
    cur = world
    for change in changes:
        if change[0] == "define":
            link = change[1]
            nxt = copy.deepcopy(cur)
            own = nxt["settings"].get("linearEvents", [])
            if change[2] == "remove":
                own = [l for l in own if l["id"] != link["id"]]
            elif any(l["id"] == link["id"] for l in own):
                own = [link if l["id"] == link["id"] else l for l in own]
            else:
                own = own + [link]
            nxt["settings"]["linearEvents"] = own
            if not nxt["settings"]["linearEvents"]:
                del nxt["settings"]["linearEvents"]
            input_ = {"operation": "remove", "id": link["id"]} if change[2] == "remove" else {"operation": "set", "link": link}
            expect = {"dirty": True, "revision": "changed", "linearEvents": nxt["settings"].get("linearEvents", []), "fields": event_fields(nxt)}
            steps.append({"op": "execute", "command": "cad.linearref.define", "input": input_, "within": WITHIN,
                          "result": {"status": "completed", "output": "$any", "warnings": []}, "expect": expect})
            cur = nxt
            states.append(cur)
            continue
        after, command, input_, written = apply_change(cur, change)
        done, warnings = follow(after, written)
        expect = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "fields": event_fields(done)}
        steps.append({"op": "execute", "command": command, "input": input_, "within": WITHIN,
                      "result": {"status": "completed", "output": "$any", "warnings": warnings}, "expect": expect})
        cur = done
        states.append(cur)
    if undo or redo:
        before = states[-2]
        ids = lambda w: sorted(e["id"] for e in w["entities"])  # noqa: E731
        steps.append({"op": "undo", "within": WITHIN, "expect": {"canRedo": True, "ids": ids(before), "fields": event_fields(before)}})
        if redo:
            steps.append({"op": "redo", "within": WITHIN, "expect": {"canUndo": True, "canRedo": False, "ids": ids(cur), "fields": event_fields(cur)}})
    return {"name": name, "setup": world, "steps": steps}


def drifted(doc):
    """Event 3 is not where its route puts it."""
    for e in doc["entities"]:
        if e["id"] == 3:
            e["p"] = pt(90, 30)


def drifted_both(doc):
    """Events 3 (on D-100) and 4 (on E-1) are not where their routes put them."""
    for e in doc["entities"]:
        if e["id"] == 3:
            e["p"] = pt(90, 30)
        if e["id"] == 4:
            e["p"] = pt(5, 70)


def waiting(doc):
    """Event 10 names a route no object is yet (D-9): it lies where it was drawn."""
    doc["entities"].append(point(10, "kaza", (0, 0), {"Yol": "D-9", "Km": "20"}))


def bent(doc):
    """D-100 is a polyline with a bend; event 6 is a straight line over the bend."""
    doc["entities"][0] = polyline(1, "yol", [(0, 0), (50, 0), (50, 50)], {"Yol": "D-100"}, [0.0, 50.0, 100.0])
    for e in doc["entities"]:
        if e["id"] == 6:
            e["a"], e["b"] = pt(10, 1), pt(80, 1)
            e["attrs"] = {"Yol": "D-100", "Bas": "10", "Son": "80", "Sapma": ""}
            e["ma"], e["mb"] = 10.0, 80.0


def cases():
    base = setup("base")
    out = []
    out.append(case("Rota taşınınca iki bağın olayları aynı adımda izler (noktalar ve çizgi); öbür rotanın olayları durur", base, [("move", 1, 0.0, 10.0)], undo=True, redo=True))
    out.append(case("Çoklu çizgi rota taşınınca nokta ve çoklu çizgi olay izler (rotanın parçalarına oturur)", base, [("move", 2, 5.0, 0.0)]))
    out.append(case("Olayın ölçüsü değişince olay rotada yeni ölçüye gider", base, [("attrs", 3, {"Km": "70"})]))
    out.append(case("Olayın rotası değişince olay öbür rotanın ölçüsüne gider (ötelemesi sağda)", base, [("attrs", 3, {"Yol": "E-1", "Km": "30"})]))
    out.append(case("Öteleme değişince olay rotanın karşı yanına geçer", base, [("attrs", 5, {"Sapma": "-3"})]))
    out.append(case("Elle taşınan nokta olay rotanın verdiği yere geri döner", base, [("move", 5, 7.0, 7.0)]))
    out.append(case("Elle taşınan çizgi olay rotanın verdiği yere geri döner", base, [("move", 6, 0.0, 5.0)]))
    out.append(case("Olayların ve rotaların katmanına dokunmayan yazma hiçbir olayı yerine koymaz", setup("drifted", mutate=drifted), [("move", 8, 1.0, 1.0)]))
    out.append(case("Olaya dokunan yazma yalnız o olayı yerine koyar; aynı rotanın kaymış öbür olayı yerinde kalır", setup("drifted", mutate=drifted), [("attrs", 5, {"Not": "x"})]))
    out.append(case("Rotaya dokunan yazma (ilgisiz öznitelik de) o rotanın bütün olaylarını yerine koyar", setup("drifted", mutate=drifted), [("attrs", 1, {"Not": "x"})]))
    out.append(case("Rotaya dokunan yazma öbür rotanın kaymış olayına bakmaz", setup("drifted-both", mutate=drifted_both), [("move", 1, 0.0, 10.0)]))
    out.append(case("Rotanın kimliği bekleyen bir kimliğe dönünce o kimliğin olayı rotaya oturur, eski kimliğinkiler söylenir", setup("waiting", mutate=waiting), [("attrs", 1, {"Yol": "D-9"})]))
    out.append(case("Rota silinince onun olayları olduğu yerde kalır, her bağ kendi uyarısını verir", base, [("delete", 1)]))
    out.append(case("Rotanın kimliği değişince eski kimliğe bağlı olaylar yerlerinde kalır ve söylenir", base, [("attrs", 1, {"Yol": "D-101"})]))
    out.append(case("Ölçü rotanın dışındaysa olay yerinde kalır ve nedeni söylenir", base, [("attrs", 3, {"Km": "500"})]))
    out.append(case("Ölçü sayı değilse olay yerinde kalır ve hücre söylenir", base, [("attrs", 3, {"Km": "abc"})]))
    out.append(case("Ölçü boşsa olay yerinde kalır ve hücre söylenir", base, [("attrs", 3, {"Km": ""})]))
    out.append(case("Öteleme sayı değilse olay yerinde kalır ve hücre söylenir", base, [("attrs", 3, {"Sapma": "x"})]))
    out.append(case("Kilitli olay katmanı: olaylar taşınmaz ve söylenir, öteki bağın olayı izler", setup("locked", locked=("kaza",)), [("move", 1, 0.0, 10.0)]))
    out.append(case("Yeni olay nesnesi aynı adımda rotasına oturur", base, [("point", "kaza", (0.0, 0.0), {"Yol": "D-100", "Km": "60"})], undo=True))
    out.append(case("Rotası olmayan yeni olay nesnesi çizildiği yerde kalır ve söylenir", base, [("point", "kaza", (0.0, 0.0), {"Yol": "D-9", "Km": "60"})]))
    out.append(case("Tolerans: 5 mm kayan nokta (varsayılan 1 mm) taşınır, çizgi (10 mm) kalır", base, [("move", 1, 0.005, 0.0)]))
    out.append(case("Çizgi nesnesi kırıklı yola taşınamaz: olduğu yerde kalır ve söylenir", setup("bent", links=(LINES,), mutate=bent), [("attrs", 6, {"Not": "x"})]))
    out.append(case("Olay nesnesi silinince başka bir şey olmaz", base, [("delete", 3)]))
    out.append(case("Bağı tanımlamak olayları taşımaz; rotasına dokunan sonraki yazma taşır", setup("drifted", mutate=drifted), [("define", {**POINTS, "tolerance": 0.5}, "set"), ("attrs", 1, {"Not": "y"})]))
    out.append(case("Bağı silince o bağın olayları izlemez, öteki bağın olayları izler", base, [("define", POINTS, "remove"), ("move", 1, 0.0, 10.0)]))
    out.append(case("Bağsız çizimde yazma hiçbir olayı izlemez", setup("none", links=()), [("move", 1, 0.0, 10.0)]))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.linearref.define",
        "commandVersion": 1,
        "title": "Olayları rotalarında güncel tutmak (ADR 0569 §3, ADR 0641 §2)",
        "note": "scripts/fixtures/linearref_follow_cases.py yazar; beklentiler ADR 0569'un ve ADR 0641 §2'nin kurallarından, KentOS kodu olmadan (rotaların kuralları lrs_cases.py'nin 50 basamaklı başvurusu, okuma ve kayma lrs_processing_cases.py'nin). Her durum kendi çizimini kurar; koordinatlar ±1e-6 içinde.",
        "setup": setup("base"),
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"cad.linearref.follow durumları güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
