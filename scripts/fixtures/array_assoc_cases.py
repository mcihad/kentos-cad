"""Associative arrays: shared cases of cad.entities.array with `associative` (docs/adr/0300).

    python3 scripts/fixtures/array_assoc_cases.py           # writes the file
    python3 scripts/fixtures/array_assoc_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.entities.array.assoc.json. The ADR's rules, computed here without KentOS code:

- The objects become a block “Dizi n” (the first free n), their copies inside it numbered from 1 in the input's order,
  its base point the middle of their box (a circle's box is its centre ± r, a line's its ends). The objects are removed.
- Every place gets an insert of the block on the first object's layer, scale 1, turn 0 (a rigid map turns it): the
  first place is the base point, a grid's place j columns and i rows away moves j·dx east and i·dy north (row after
  row), a line's place k moves k·(dx, dy) or with `fit` k·(dx, dy)/(count − 1), a path's place k moves by the path's
  point at the place less its start (spread evenly from start to end without a spacing).
- The array keeps the block, the first place, the layout and an item per place: its insert.
- At a step's end: an item's insert removed by hand leaves the item empty; changed by hand, the item is kept; a path
  array's path changed, the items neither removed nor kept go to their new places.
- One undo step named after the tool (Dizi, Kopyala, Kutupsal dizi, Yol boyunca dizi).

Grid, line and straight path places are exact sums; polar places are checked by their count and record only.
"""
import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.entities.array.assoc.json"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
U = lambda i: f"$uidOf:{i}"  # noqa: E731


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


CIRCLE = {"kind": "circle", **base(1), "c": p(10, 0), "r": 1}
LINE = {"kind": "line", **base(2), "a": p(20, 0), "b": p(22, 0)}
PATH = {"kind": "line", **base(3), "a": p(0, 30), "b": p(40, 30)}
LOCKED = {"kind": "circle", **base(4, "kilitli"), "c": p(50, 50), "r": 1}
POINT = {"kind": "point", **base(5), "p": p(80, 0)}
ENTITIES = [CIRCLE, LINE, PATH, LOCKED, POINT]
IDS = [e["id"] for e in ENTITIES]
NEXT = 6
LOCKED_WORDS = "1 nesne kilitli katmanda olduğu için atlandı. Kopyalamak için katmanın kilidini Katmanlar panelinden açın."


def setup():
    return {
        "format": "kentos.document", "version": 1, "name": "İlişkili diziler",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                     "workspace": "cad"},
        "origin": p(40, 20),
        "layers": [node("cizim", "Çizim"), node("kilitli", "Kilitli", locked=True)],
        "activeLayer": "cizim",
        "entities": copy.deepcopy(ENTITIES),
        "styles": {"items": [], "categories": []},
    }


# ── The rules ──────────────────────────────────────────────────────────────────────────────────────────────────────

def box(e):
    if e["kind"] == "circle":
        c, r = e["c"], e["r"]
        return (c["x"] - r, c["y"] - r, c["x"] + r, c["y"] + r)
    if e["kind"] == "line":
        a, b = e["a"], e["b"]
        return (min(a["x"], b["x"]), min(a["y"], b["y"]), max(a["x"], b["x"]), max(a["y"], b["y"]))
    raise AssertionError(e["kind"])


def middle(sources):
    boxes = [box(e) for e in sources]
    lo_x, lo_y = min(b[0] for b in boxes), min(b[1] for b in boxes)
    hi_x, hi_y = max(b[2] for b in boxes), max(b[3] for b in boxes)
    return ((lo_x + hi_x) / 2, (lo_y + hi_y) / 2)


def moves(layout, path=None):
    """The places' moves from the first, the first's none."""
    kind = layout["kind"]
    if kind == "grid":
        return [(j * layout["dx"], i * layout["dy"]) for i in range(layout["rows"]) for j in range(layout["cols"])]
    if kind == "line":
        n = layout["count"]
        if layout.get("fit"):
            return [(k * layout["dx"] / (n - 1), k * layout["dy"] / (n - 1)) for k in range(n)]
        return [(k * layout["dx"], k * layout["dy"]) for k in range(n)]
    if kind == "path":
        a, b = path["a"], path["b"]
        length = ((b["x"] - a["x"]) ** 2 + (b["y"] - a["y"]) ** 2) ** 0.5
        n = layout["count"]
        out = []
        for k in range(n):
            s = k * layout["spacing"] if "spacing" in layout else length * k / (n - 1)
            out.append(((b["x"] - a["x"]) * s / length, (b["y"] - a["y"]) * s / length))
        return out
    if kind == "polar":
        # Checked by their count and the record only (cos and sin are not exact sums).
        return [None] * layout["count"]
    raise AssertionError(kind)


def block_of(sources, name):
    content = []
    for k, e in enumerate(sources, 1):
        c = copy.deepcopy(e)
        c["id"] = k
        content.append(c)
    mx, my = middle(sources)
    return {"name": name, "base": p(mx, my), "entities": content}


def insert(slot, name, at, layer="cizim"):
    return {"kind": "insert", "id": slot, "layerId": layer, "attrs": {}, "block": f"$blockOf:{name}", "p": p(*at),
            "scale": 1, "rotation": 0}


def record(sources, name, layout, slots, kept=(), removed=(), layer="cizim"):
    mx, my = middle(sources)
    items = []
    for k, slot in enumerate(slots):
        if k in removed:
            items.append({})
        elif k in kept:
            items.append({"uid": U(slot), "kept": True})
        else:
            items.append({"uid": U(slot)})
    return {"block": f"$blockOf:{name}", "layerId": layer, "p": p(mx, my), "scale": 1, "rotation": 0, "layout": layout,
            "items": items}


def made(sources, layout, path=None, start=NEXT, name="Dizi 1"):
    """The items' slots and inserts of an associative array of `sources`."""
    mx, my = middle(sources)
    if layout["kind"] == "path" and path is None:
        path = PATH
    shifts = moves(layout, path)
    slots = list(range(start, start + len(shifts)))
    if layout["kind"] == "polar":
        return slots, {}
    inserts = {str(s): insert(s, name, (mx + dx, my + dy)) for s, (dx, dy) in zip(slots, shifts)}
    return slots, inserts


# ── Cases ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def assoc(uids, layout):
    return {"uids": [U(i) for i in uids], "layout": layout, "associative": True}


def written(slots, locked=()):
    return {"status": "completed", "warnings": [{"code": "layer_locked", "message": LOCKED_WORDS, "path": "uids"}] if locked else [],
            "output": {"created": [U(s) for s in slots], "locked": [U(i) for i in locked], "array": "$any", "revision": "$current"}}


def create(sources_ids, layout, path=None, locked=(), label=None, undo=False, redo=False, name="Dizi 1", start=NEXT,
           exact=True):
    sources = [e for e in ENTITIES if e["id"] in sources_ids and e["id"] not in locked]
    slots, inserts = made(sources, layout, path, start, name)
    kept_ids = [i for i in IDS if i not in [e["id"] for e in sources]]
    expect = {"ids": sorted(kept_ids + slots), "blocks": [block_of(sources, name)], "arrays": [record(sources, name, layout, slots)],
              "revision": "changed", "canUndo": True}
    if exact:
        expect["entities"] = inserts
    steps = [{"op": "execute", "input": assoc(sources_ids, layout), "result": written(slots, locked), "expect": expect}]
    if undo:
        steps.append({"op": "undo", "returns": label, "expect": {"ids": IDS, "blocks": [], "arrays": []}})
    if redo:
        steps.append({"op": "redo", "returns": label, "expect": {"ids": sorted(kept_ids + slots), "arrays": [record(sources, name, layout, slots)]}})
    return steps, slots


GRID = {"kind": "grid", "rows": 2, "cols": 3, "dx": 5, "dy": 4}
LINE_LAYOUT = {"kind": "line", "count": 3, "dx": 0, "dy": 10}
FIT = {"kind": "line", "count": 3, "dx": 0, "dy": 10, "fit": True}
ON_PATH = {"kind": "path", "path": U(3), "count": 3, "align": False}
SPACED = {"kind": "path", "path": U(3), "count": 3, "spacing": 10, "align": True}
POLAR = {"kind": "polar", "center": p(10, 10), "count": 4, "fill": 360, "rotate": True}
STILL = {"kind": "polar", "center": p(10, 10), "count": 4, "fill": 360, "rotate": False}
STRETCH = {"window": {"min": p(39, 29), "max": p(41, 31)}, "dx": 20, "dy": 0}
STRETCHED = {**PATH, "b": p(60, 30)}


def helper(command, input_):
    return {"op": "execute", "command": command, "input": input_, "result": "$any"}


def path_steps():
    steps, slots = create([1], ON_PATH)
    return steps, slots


def cases():
    out = []
    steps, _ = create([1], GRID, label="Dizi", undo=True, redo=True)
    out.append({"name": "Izgara 2×3 ilişkili: çember “Dizi 1” bloğu olur, altı yerleştirme ve dizi; adım “Dizi”, geri alınır ve yinelenir",
                "steps": steps})
    steps, _ = create([1, 2], LINE_LAYOUT, label="Kopyala", undo=True)
    out.append({"name": "İki nesne: bloğun taban noktası kutularının ortası, çizgi boyunca üç yer; adım “Kopyala”", "steps": steps})
    steps, _ = create([1], FIT)
    out.append({"name": "Sığdır: üç yer yer değiştirmeyi paylaşır", "steps": steps})
    steps, _ = create([1], POLAR, label="Kutupsal dizi", undo=True, exact=False)
    out.append({"name": "Kutupsal dizi dönerek: dört yerleştirme ve dizi; adım “Kutupsal dizi”", "steps": steps})
    steps, _ = create([1], STILL, exact=False)
    out.append({"name": "Kutupsal dizi dönmeden: taban noktası çember üzerinde", "steps": steps})
    steps, _ = create([1], ON_PATH, label="Yol boyunca dizi", undo=True)
    out.append({"name": "Yol boyunca: yolun başına göre üç yer, eşit dağıtılmış; adım “Yol boyunca dizi”", "steps": steps})
    steps, _ = create([1], SPACED)
    out.append({"name": "Yol boyunca aralıkla ve hizalı: düz yolda dönüş yok", "steps": steps})

    # The path array follows its path.
    steps, slots = path_steps()
    _, after = made([CIRCLE], ON_PATH, STRETCHED)
    steps.append({**helper("cad.entities.stretch", STRETCH), "expect": {"entities": after, "arrays": [record([CIRCLE], "Dizi 1", ON_PATH, slots)]}})
    steps.append({"op": "undo", "returns": "Esnet", "expect": {"entities": made([CIRCLE], ON_PATH, PATH)[1]}})
    out.append({"name": "Yol esnetilince dizi izler: öğeler yeni yerlerine aynı adımda; geri alınınca dönerler", "steps": steps})

    steps, slots = path_steps()
    moved = insert(slots[1], "Dizi 1", (30, 5))
    steps.append({**helper("cad.entities.transform", {"uids": [U(slots[1])], "transform": {"kind": "move", "dx": 0, "dy": 5}}),
                  "expect": {"arrays": [record([CIRCLE], "Dizi 1", ON_PATH, slots, kept=(1,))], "entities": {str(slots[1]): moved}}})
    _, after = made([CIRCLE], ON_PATH, STRETCHED)
    after[str(slots[1])] = moved
    steps.append({**helper("cad.entities.stretch", STRETCH), "expect": {"entities": after}})
    out.append({"name": "Elle taşınan öğe korunur: yol değişince yerinde kalır, öbürleri izler", "steps": steps})

    steps, slots = path_steps()
    steps.append({**helper("cad.entities.delete", {"uids": [U(slots[2])]}),
                  "expect": {"arrays": [record([CIRCLE], "Dizi 1", ON_PATH, slots, removed=(2,))], "ids": sorted([2, 3, 4, 5] + slots[:2])}})
    _, after = made([CIRCLE], ON_PATH, STRETCHED)
    del after[str(slots[2])]
    steps.append({**helper("cad.entities.stretch", STRETCH), "expect": {"entities": after, "ids": sorted([2, 3, 4, 5] + slots[:2])}})
    out.append({"name": "Elle silinen öğe boş kalır: yol değişince geri gelmez", "steps": steps})

    steps, slots = path_steps()
    steps.append(helper("cad.entities.delete", {"uids": [U(slots[2])]}))
    steps.append({"op": "undo", "returns": "Sil", "expect": {"arrays": [record([CIRCLE], "Dizi 1", ON_PATH, slots)], "ids": sorted([2, 3, 4, 5] + slots)}})
    out.append({"name": "Silmeyi geri almak öğeyi diziye geri verir", "steps": steps})

    steps, slots = path_steps()
    steps.append({**helper("cad.entities.transform", {"uids": [U(slots[0])], "transform": {"kind": "move", "dx": 1, "dy": 0}}),
                  "expect": {"arrays": [record([CIRCLE], "Dizi 1", ON_PATH, slots, kept=(0,))]}})
    out.append({"name": "İlk yerin öğesi de elle taşınınca korunur", "steps": steps})

    steps, slots = path_steps()
    # (The record names the path by `$uidOf:3`, which a removed slot cannot fill: the items are checked.)
    steps.append({**helper("cad.entities.delete", {"uids": [U(3)]}),
                  "expect": {"ids": sorted([2, 4, 5] + slots), "entities": made([CIRCLE], ON_PATH, PATH)[1]}})
    out.append({"name": "Yol silinince dizi ve öğeleri olduğu gibi kalır", "steps": steps})

    steps, slots = create([1], GRID)
    two, inserts = made([LINE], LINE_LAYOUT, start=slots[-1] + 1, name="Dizi 2")
    steps.append({"op": "execute", "input": assoc([2], LINE_LAYOUT), "result": written(two),
                  "expect": {"blocks": [block_of([CIRCLE], "Dizi 1"), block_of([LINE], "Dizi 2")], "entities": inserts,
                             "arrays": [record([CIRCLE], "Dizi 1", GRID, slots), record([LINE], "Dizi 2", LINE_LAYOUT, two)]}})
    out.append({"name": "İkinci dizi “Dizi 2” olur, ikisi de belgede", "steps": steps})

    steps, _ = create([1, 4], {"kind": "line", "count": 2, "dx": 5, "dy": 0}, locked=(4,))
    out.append({"name": "Kilitli katmandaki nesne uyarıyla atlanır, yerinde kalır", "steps": steps})
    out.append({"name": "Yalnız kilitli nesne: reddedilir, hiçbir şey yazılmaz",
                "steps": [{"op": "execute", "input": assoc([4], GRID),
                           "result": {"status": "failed", "error": {"code": "layer_locked", "message": LOCKED_WORDS, "path": "uids"}},
                           "expect": {"ids": IDS, "blocks": [], "arrays": [], "canUndo": False, "revision": "same"}}]})

    copies = [{**CIRCLE, "id": 0, "c": p(10 + j * 5, i * 4)} for i in range(2) for j in range(3)][1:]
    out.append({"name": "Plan ilişkili de olsa kopyaları gösterir; hiçbir şey yazılmaz",
                "steps": [{"op": "plan", "input": assoc([1], GRID),
                           "result": {"status": "completed", "warnings": [], "output": {"sources": [U(1)], "entities": copies, "locked": [], "revision": "$current"}},
                           "expect": {"ids": IDS, "blocks": [], "arrays": [], "revision": "same"}}]})
    out.append({"name": "İlişkisiz dizi eskisi gibi: kopyalar, blok ve dizi yok",
                "steps": [{"op": "execute", "input": {"uids": [U(1)], "layout": LINE_LAYOUT},
                           "result": {"status": "completed", "warnings": [], "output": {"created": [U(6), U(7)], "locked": [], "revision": "$current"}},
                           "expect": {"ids": IDS + [6, 7], "blocks": [], "arrays": []}}]})

    steps, slots = path_steps()
    steps.append(helper("cad.entities.stretch", STRETCH))
    steps.append({"op": "undo", "returns": "Esnet", "expect": {"entities": made([CIRCLE], ON_PATH, PATH)[1]}})
    steps.append({"op": "redo", "returns": "Esnet", "expect": {"entities": made([CIRCLE], ON_PATH, STRETCHED)[1]}})
    out.append({"name": "Esnetmeyi yinelemek öğeleri yine yolun yeni yerlerine koyar", "steps": steps})

    steps, slots = create([1], GRID)
    steps.append({**helper("cad.entities.transform", {"uids": [U(slots[4])], "transform": {"kind": "move", "dx": 0, "dy": 1}}),
                  "expect": {"arrays": [record([CIRCLE], "Dizi 1", GRID, slots, kept=(4,))]}})
    steps.append({"op": "undo", "returns": "Taşı", "expect": {"arrays": [record([CIRCLE], "Dizi 1", GRID, slots)]}})
    out.append({"name": "Izgarada elle taşıma öğeyi korur; geri alınınca koruma kalkar", "steps": steps})

    steps, slots = create([1], GRID)
    steps.append({**helper("cad.entities.delete", {"uids": [U(s) for s in slots[:3]]}),
                  "expect": {"arrays": [record([CIRCLE], "Dizi 1", GRID, slots, removed=(0, 1, 2))]}})
    out.append({"name": "Bir satırın öğeleri birlikte silinir, üç öğe boş kalır", "steps": steps})
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.entities.array", "commandVersion": 1,
            "title": "İlişkili diziler: kaynak blok, yerleşim, öğeler ve izleme",
            "note": "ADR 0300. Bağımsız başvuru: scripts/fixtures/array_assoc_cases.py.",
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
