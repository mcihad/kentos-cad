#!/usr/bin/env python3
"""The shared cases of `file.import.block` from a DXF or DWG block library (docs/adr/0726 §2).

    python3 scripts/fixtures/block_library_cases.py           # writes the files
    python3 scripts/fixtures/block_library_cases.py --check   # writes nothing; compares

Writes the library fixtures/blocks/v1/kitaplik (four DXF files, written here by hand from the DXF reference) and
fixtures/commands/v1/file.import.block.library.json, with Python's standard library, no KentOS code. The DWG twins in
fixtures/blocks/v1/kitaplik-dwg are the codec's (cargo run -p kentos-dwg --example corpus): their bytes are receipts,
their cases the DXF's with the path changed (scripts/fixtures/dwg_import_command_cases.py does the same).

The rules, from ADR 0726 §2 and the ones it builds on (0257 a DXF's unit, 0315 §4 a block's unit, 0193 the
exchange's `take` and `file_block`, 0316 §3 the command):

- a file's numbers are in the unit it names ($INSUNITS) or `sourceUnit`, whatever it names; a file naming none without
  `sourceUnit` is refused (`source_unit_unknown`), so no block comes in at a size nobody chose;
- a block is in its record's unit (BLOCK_RECORD 70), else the file's; numbers turn into metres with the unit's exact
  ratio, one multiplication then one division (dxf_units_cases.metres); an insert's factor, in the model or in a
  definition, is multiplied by its owner's unit over its block's (ADR 0589);
- `blocks`: the blocks named and those they place, in the file's order; a name the drawing has is kept (`skipped`),
  or with `replace` redefined (its id stays, `changed`); `drawing`: the file's objects one block named after the file,
  its unit the file's, its base the lower left of the objects' own boxes (an insert's: its point), the blocks they
  place before it, the layers they are on with them by path;
- `category` goes on the cards of those coming in that have none (version 1);
- a block's size is the box of what it draws placed at the origin at scale 1, its pieces only (not its base point);
- the reader's left-out objects (a 3DSOLID) are said (`objects_left_out`).
"""
import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import dxf_units_cases as units  # noqa: E402  (the DXF writer's pairs and the units' exact ratios)

ROOT = Path(__file__).resolve().parents[2]
LIB = ROOT / "fixtures/blocks/v1/kitaplik"
OUT = ROOT / "fixtures/commands/v1/file.import.block.library.json"
FIX = "$fixture:blocks/v1/kitaplik"
FIX_DWG = "$fixture:blocks/v1/kitaplik-dwg"
WITHIN = 1e-9

# ── The library ──────────────────────────────────────────────────────────
# A block: its name, its record's unit code (None: the file's) and its objects in its own unit:
#   ("rect", x0, y0, x1, y1)                  a closed LWPOLYLINE
#   ("line", x0, y0, x1, y1)
#   ("circle", x, y, r)
#   ("arc", x, y, r, a0, a1)                  degrees, counter-clockwise
#   ("insert", name, x, y, scale, turn)       turn a multiple of 90 degrees
#   ("solid",)                                a 3DSOLID, which the reader leaves out
LIBRARY = {
    # A bedroom in millimetres: a 90 × 200 bed (its base the middle of its back edge), a bedside table, and a rug
    # whose record says centimetres, placed at its physical size (insert factor 10).
    "yatak-odasi/yatak-odasi.dxf": {
        "insunits": 4,
        "blocks": [
            ("Yatak 90", None, [("rect", -450, 0, 450, 2000), ("rect", -350, 100, 350, 450),
                                ("line", -450, 600, 450, 600)]),
            ("Komodin", None, [("rect", 0, 0, 500, 400), ("circle", 250, 200, 100)]),
            ("Kilim", 5, [("rect", 0, 0, 200, 140), ("line", 0, 70, 200, 70)]),
        ],
        "model": [("insert", "Yatak 90", 0, 0, 1, 0), ("insert", "Komodin", 500, 0, 1, 0),
                  ("insert", "Kilim", -1000, 2200, 10, 0)],
    },
    # A dining set in centimetres: a 160 × 90 table that places four chairs, two of them turned round.
    "yemek/yemek-takimi.dxf": {
        "insunits": 5,
        "blocks": [
            ("Sandalye", None, [("rect", -22.5, 0, 22.5, 45), ("rect", -22.5, 45, 22.5, 50)]),
            ("Yemek masası", None, [("rect", -80, -45, 80, 45),
                                    ("insert", "Sandalye", -40, -95, 1, 0), ("insert", "Sandalye", 40, -95, 1, 0),
                                    ("insert", "Sandalye", -40, 95, 1, 180), ("insert", "Sandalye", 40, 95, 1, 180)]),
        ],
        "model": [("insert", "Yemek masası", 0, 0, 1, 0)],
    },
    # A toilet whose file names no unit (its numbers are millimetres, which only its user knows), with a 3DSOLID.
    "islak-hacim/klozet.dxf": {
        "insunits": None,
        "blocks": [
            ("Klozet", None, [("rect", -190, 0, 190, 180), ("circle", 0, 370, 180), ("arc", 0, 370, 150, 0, 180)]),
        ],
        "model": [("insert", "Klozet", 0, 0, 1, 0), ("solid",)],
    },
    # A washbasin in inches: its front an arc, whose top is its depth.
    "islak-hacim/lavabo.dxf": {
        "insunits": 1,
        "blocks": [("Lavabo", None, [("rect", -12, 0, 12, 10), ("arc", 0, 10, 12, 0, 180)])],
        "model": [("insert", "Lavabo", 0, 0, 1, 0)],
    },
}


def lwpolyline(x0, y0, x1, y1, layer):
    groups = [(0, "LWPOLYLINE"), (8, layer), (100, "AcDbEntity"), (100, "AcDbPolyline"), (90, 4), (70, 1)]
    for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1)):
        groups += [(10, x), (20, y)]
    return units.pairs(*groups)


def written(obj, layer):
    kind = obj[0]
    if kind == "rect":
        return lwpolyline(*obj[1:], layer)
    if kind == "line":
        return units.line(*obj[1:], layer)
    if kind == "circle":
        return units.circle(*obj[1:], layer)
    if kind == "arc":
        x, y, r, a0, a1 = obj[1:]
        return units.pairs((0, "ARC"), (8, layer), (10, x), (20, y), (30, 0.0), (40, r), (50, a0), (51, a1))
    if kind == "insert":
        name, x, y, s, turn = obj[1:]
        return units.pairs((0, "INSERT"), (8, layer), (2, name), (10, x), (20, y), (30, 0.0), (41, s), (42, s),
                           (43, s), (50, turn))
    if kind == "solid":
        return units.pairs((0, "3DSOLID"), (8, layer), (100, "AcDbEntity"), (100, "AcDbModelerGeometry"), (70, 1),
                           (1, "ACIS"))
    raise ValueError(kind)


def dxf(f):
    """The file: its header ($INSUNITS when it names one), the layers 0 and MOBILYA, its block records (their unit
    when they name one) and blocks (their objects on 0), its model (on MOBILYA)."""
    head = [(0, "SECTION"), (2, "HEADER"), (9, "$ACADVER"), (1, "AC1015")]
    if f["insunits"] is not None:
        head += [(9, "$INSUNITS"), (70, f["insunits"])]
    head += [(0, "ENDSEC")]
    tables = [(0, "SECTION"), (2, "TABLES"), (0, "TABLE"), (2, "LAYER"),
              (0, "LAYER"), (2, "0"), (70, 0), (62, 7), (6, "CONTINUOUS"),
              (0, "LAYER"), (2, "MOBILYA"), (70, 0), (62, 3), (6, "CONTINUOUS"),
              (0, "ENDTAB"), (0, "TABLE"), (2, "BLOCK_RECORD")]
    for i, (name, unit, _) in enumerate(f["blocks"]):
        tables += [(0, "BLOCK_RECORD"), (5, f"{0x100 + i:X}"), (100, "AcDbSymbolTableRecord"),
                   (100, "AcDbBlockTableRecord"), (2, name)]
        if unit is not None:
            tables += [(70, unit)]
    tables += [(0, "ENDTAB"), (0, "ENDSEC")]
    text = units.pairs(*head) + units.pairs(*tables)
    text += units.pairs((0, "SECTION"), (2, "BLOCKS"))
    for name, _, objects in f["blocks"]:
        text += units.pairs((0, "BLOCK"), (8, "0"), (2, name), (70, 0), (10, 0.0), (20, 0.0), (30, 0.0), (3, name))
        text += "".join(written(o, "0") for o in objects)
        text += units.pairs((0, "ENDBLK"), (8, "0"))
    text += units.pairs((0, "ENDSEC"), (0, "SECTION"), (2, "ENTITIES"))
    text += "".join(written(o, "MOBILYA") for o in f["model"])
    text += units.pairs((0, "ENDSEC"), (0, "EOF"))
    return text


# ── What reading a file gives ────────────────────────────────────────────

UNIT_NAMES = {1: "in", 2: "ft", 4: "mm", 5: "cm", 6: "m"}
CODE_OF = {v: k for k, v in UNIT_NAMES.items()}


def decision(f, source_unit):
    """The file's unit as decided (ADR 0257 §1 with a millimetre fallback that this command never takes, §2 of 0726):
    None when it names none and none is given."""
    declared = f["insunits"] if f["insunits"] not in (None, 0) else None
    if source_unit is not None:
        used, source = CODE_OF[source_unit], "chosen"
    elif declared in units.AUTOCAD:
        used, source = declared, "declared"
    else:
        return None
    out = {"used": used, "name": units.AUTOCAD[used][1], "source": source,
           "factor": units.factor(used), "assumed": False}
    if declared is not None:
        out["declared"] = declared
    file_code = declared or 0
    listed = [{"name": n, "code": u} for n, u, _ in f["blocks"] if u not in (None, 0) and u != file_code]
    if listed:
        out["blocks"] = listed
    return out


def scale_of(code):
    (mul, div), _ = units.AUTOCAD[code]
    return mul, div


def ratio(owner, target):
    """An insert's factor's multiplier: its owner's unit over its block's, as the reader computes it."""
    om, od = scale_of(owner)
    tm, td = scale_of(target)
    return om * td / (od * tm)


def turned(x, y, turn):
    """(x, y) turned by a multiple of 90 degrees, exactly."""
    quarter = (round(turn) // 90) % 4
    return [(x, y), (-y, x), (-x, -y), (y, -x)][quarter]


def arc_box(cx, cy, r, a0, a1):
    """The box of an arc from a0 to a1 degrees counter-clockwise: its ends and the quadrant points it passes."""
    a0r, a1r = math.radians(a0), math.radians(a1)
    span = (a1 - a0) % 360 or 360
    pts = [(cx + r * math.cos(a0r), cy + r * math.sin(a0r)), (cx + r * math.cos(a1r), cy + r * math.sin(a1r))]
    for q in (0, 90, 180, 270):
        if (q - a0) % 360 <= span:
            pts.append((cx + r * math.cos(math.radians(q)), cy + r * math.sin(math.radians(q))))
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return [min(xs), min(ys), max(xs), max(ys)]


def union(a, b):
    if a is None:
        return b
    if b is None:
        return a
    return [min(a[0], b[0]), min(a[1], b[1]), max(a[2], b[2]), max(a[3], b[3])]


class Read:
    """A library file as KentOS reads it, in the unit decided: each block's unit code and its box in metres."""

    def __init__(self, f, used):
        self.f = f
        self.file_code = used
        self.blocks = {name: (unit or used, objects) for name, unit, objects in f["blocks"]}

    def code(self, name):
        return self.blocks[name][0]

    def box_of_objects(self, objects, code):
        box = None
        for o in objects:
            box = union(box, self.object_box(o, code))
        return box

    def object_box(self, o, code):
        m = lambda v: units.metres(v, code)  # noqa: E731
        kind = o[0]
        if kind in ("rect", "line"):
            x0, y0, x1, y1 = (m(v) for v in o[1:])
            return [min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1)]
        if kind == "circle":
            x, y, r = (m(v) for v in o[1:])
            return [x - r, y - r, x + r, y + r]
        if kind == "arc":
            return arc_box(m(o[1]), m(o[2]), m(o[3]), o[4], o[5])
        if kind == "insert":
            name, x, y, s, turn = o[1:]
            target = self.code(name)
            factor = s * ratio(code, target)
            inner = self.box(name)
            px, py = m(x), m(y)
            corners = [turned(cx * factor, cy * factor, turn) for cx in (inner[0], inner[2]) for cy in (inner[1], inner[3])]
            xs = [px + c[0] for c in corners]
            ys = [py + c[1] for c in corners]
            return [min(xs), min(ys), max(xs), max(ys)]
        if kind == "solid":
            return None
        raise ValueError(kind)

    def box(self, name):
        code, objects = self.blocks[name]
        return self.box_of_objects(objects, code)

    def model_box(self):
        return self.box_of_objects(self.f["model"], self.file_code)

    def model_base(self):
        """The lower left of the model objects' own boxes: an insert's is its point."""
        xs, ys = [], []
        for o in self.f["model"]:
            if o[0] == "insert":
                xs.append(units.metres(o[2], self.file_code))
                ys.append(units.metres(o[3], self.file_code))
            elif o[0] != "solid":
                b = self.object_box(o, self.file_code)
                xs.append(b[0])
                ys.append(b[1])
        return {"x": min(xs), "y": min(ys)}

    def placed(self, names):
        """The blocks `names` and those they place, in the file's order."""
        wanted = set()
        todo = list(names)
        while todo:
            n = todo.pop()
            if n in wanted:
                continue
            wanted.add(n)
            todo += [o[1] for o in self.blocks[n][1] if o[0] == "insert"]
        return [n for n, _, _ in self.f["blocks"] if n in wanted]

    def model_placed(self):
        return self.placed([o[1] for o in self.f["model"] if o[0] == "insert"])

    def inserts(self, name):
        return [o[1] for o in self.blocks[name][1] if o[0] == "insert"]

    def count(self, name):
        return len([o for o in self.blocks[name][1] if o[0] != "solid"])

    def unread(self):
        solids = sum(1 for o in self.f["model"] if o[0] == "solid") + sum(
            1 for _, _, objs in self.f["blocks"] for o in objs if o[0] == "solid")
        return [f"3DSOLID ({solids})"] if solids else []


# ── The drawings imported into ────────────────────────────────────────────

SETTINGS = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
            "workspace": "cad", "drawingFont": "barlow"}
OUR_BED = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7101"


def layer(i, name):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def setup(unit="m", bed=False, mobilya=False):
    """A local CAD drawing in `unit` with the layer Çizim and one point; maybe a layer MOBILYA; maybe a block
    “Yatak 90” of its own (a 1 m line), placed once."""
    settings = dict(SETTINGS)
    if unit != "m":
        settings["drawingUnit"] = unit
    layers = [layer("cizim", "Çizim")] + ([layer("mobilya", "MOBILYA")] if mobilya else [])
    entities = [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": {"x": 0.0, "y": 0.0}}]
    doc = {"format": "kentos.document", "version": 1, "name": "Kitaplık", "settings": settings,
           "origin": {"x": 0, "y": 0}, "layers": layers, "activeLayer": "cizim", "entities": entities,
           "styles": {"items": [], "categories": []}}
    if bed:
        doc["blocks"] = [{"id": OUR_BED, "name": "Yatak 90", "base": {"x": 0, "y": 0}, "entities": [
            {"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 1, "y": 0}}]}]
        doc["entities"].append({"kind": "insert", "id": 2, "layerId": "cizim", "attrs": {}, "block": OUR_BED,
                                "p": {"x": 5, "y": 5}, "scale": 1, "rotation": 0})
    return doc


# ── The cases ─────────────────────────────────────────────────────────────

def file_name(rel):
    return rel.rsplit("/", 1)[-1]


def stem(rel):
    return file_name(rel).rsplit(".", 1)[0]


def card(category):
    return {"category": category, "version": 1} if category else None


def imported(rel, mode="blocks", names=None, source_unit=None, category=None, ours=None, replace=False, name=None):
    """What `file.import.block` gives for `rel` into `ours` (a setup): the output and what the drawing then has."""
    ours = ours or setup()
    f = LIBRARY[rel]
    dec = decision(f, source_unit)
    assert dec is not None, "a refused read has no answer"
    r = Read(f, dec["used"])
    have = {b["name"]: b for b in ours.get("blocks", [])}
    added, changed, skipped, sizes, fields = [], [], [], [], {}
    layers = 0
    if mode == "blocks":
        taken = r.placed(names if names is not None else [n for n, _, _ in f["blocks"]])
        for n in taken:
            if n in have and not replace:
                skipped.append(n)
                continue
            ref = f"$blockOf:{n}"
            (changed if n in have else added).append(ref)
        for n in taken:
            if n in have and not replace:
                continue
            box = r.box(n)
            sizes.append({"block": f"$blockOf:{n}", "name": n, "units": UNIT_NAMES[r.code(n)],
                          "width": box[2] - box[0], "depth": box[3] - box[1]})
            fields[n] = {"units": UNIT_NAMES[r.code(n)], "card": card(category), "base": {"x": 0, "y": 0},
                         "$count": r.count(n), "$inserts": r.inserts(n)}
        # A name kept is ours as it was.
        for n in skipped:
            fields[n] = {"card": None, "$count": 1, "$inserts": []}
        # Ours redefined, in the drawing's order: ours first, then the new ones.
        order = [b["name"] for b in ours.get("blocks", [])] + [n for n in taken if n not in have]
        sizes.sort(key=lambda s: order.index(s["name"]))
        added.sort(key=lambda ref: order.index(ref[len("$blockOf:"):]))
        changed.sort(key=lambda ref: order.index(ref[len("$blockOf:"):]))
    else:
        nested = r.model_placed()
        block = name or stem(rel)
        for n in nested:
            added.append(f"$blockOf:{n}")
            box = r.box(n)
            sizes.append({"block": f"$blockOf:{n}", "name": n, "units": UNIT_NAMES[r.code(n)],
                          "width": box[2] - box[0], "depth": box[3] - box[1]})
            fields[n] = {"units": UNIT_NAMES[r.code(n)], "card": card(category), "$inserts": r.inserts(n)}
        added.append(f"$blockOf:{block}")
        box = r.model_box()
        sizes.append({"block": f"$blockOf:{block}", "name": block, "units": UNIT_NAMES[dec["used"]],
                      "width": box[2] - box[0], "depth": box[3] - box[1]})
        model_inserts = [o[1] for o in f["model"] if o[0] == "insert"]
        fields[block] = {"units": UNIT_NAMES[dec["used"]], "card": card(category), "base": r.model_base(),
                         "$count": len(model_inserts), "$inserts": model_inserts}
        # The model's objects are on MOBILYA: it comes with them unless the drawing has it.
        layers = 0 if any(l["name"] == "MOBILYA" for l in ours["layers"]) else 1
    output = {"added": added, "changed": changed, "layers": layers, "revision": "$current"}
    if mode == "drawing":
        output["block"] = f"$blockOf:{name or stem(rel)}"
    if skipped:
        output["skipped"] = skipped
    if sizes:
        output["sizes"] = sizes
    output["unit"] = dec
    warnings = []
    unread = r.unread()
    if unread:
        warnings.append({"code": "objects_left_out", "message": f"Dosyanın okunamayan nesneleri alınmadı: {', '.join(unread)}.",
                         "path": "path"})
    return output, warnings, fields


def ids_of(doc):
    return [e["id"] for e in doc["entities"]]


def execute(path, input_extra=None):
    step = {"op": "execute", "input": {"path": path, **(input_extra or {})}}
    return step


def ok_case(title, rel, *, mode="blocks", names=None, source_unit=None, category=None, ours=None, replace=False,
            name=None, dwg=False, plan=False):
    ours_doc = ours or setup()
    output, warnings, fields = imported(rel, mode, names, source_unit, category, ours_doc, replace, name)
    path = (f"{FIX_DWG}/{stem(rel)}.dwg" if dwg else f"{FIX}/{rel}")
    inp = {"path": path}
    if mode != "blocks":
        inp["mode"] = mode
    else:
        inp["mode"] = "blocks"
    if names is not None:
        inp["names"] = names
    if source_unit is not None:
        inp["sourceUnit"] = source_unit
    if category is not None:
        inp["category"] = category
    if replace:
        inp["replace"] = True
    if name is not None:
        inp["name"] = name
    steps = []
    if plan:
        planned = copy.deepcopy(output)
        for key in ("added", "changed"):
            planned[key] = ["$any"] * len(planned[key])
        if "block" in planned:
            planned["block"] = "$any"
        for s in planned.get("sizes", []):
            s["block"] = "$any"
        steps.append({"op": "plan", "input": copy.deepcopy(inp), "within": WITHIN,
                      "result": {"status": "completed", "output": planned, "warnings": warnings},
                      "expect": {"ids": ids_of(ours_doc), "canUndo": False, "revision": "same"}})
    steps.append({"op": "execute", "input": inp, "within": WITHIN,
                  "result": {"status": "completed", "output": output, "warnings": warnings},
                  "expect": {"ids": ids_of(ours_doc), "canUndo": True, "revision": "changed", "blockFields": fields}})
    file = f"{stem(rel)}.dwg" if dwg else file_name(rel)
    undo = {"op": "undo", "returns": f"Dosyadan blok: {file}",
            "expect": {"ids": ids_of(ours_doc), "blocks": copy.deepcopy(ours_doc.get("blocks", []))}}
    if ours_doc.get("blocks"):
        undo["expect"]["blocks"] = [{k: v for k, v in b.items() if k != "id"} for b in ours_doc["blocks"]]
    steps.append(undo)
    case = {"name": title, "steps": steps}
    if ours is not None:
        case["setup"] = ours_doc
    return case


def failed_case(title, path, code, message, at, *, extra=None, ours=None, op="execute"):
    ours_doc = ours or setup()
    step = {"op": op, "input": {"path": path, **(extra or {})},
            "result": {"status": "failed", "error": {"code": code, "message": message, "path": at}},
            "expect": {"ids": ids_of(ours_doc), "canUndo": False, "revision": "same"}}
    case = {"name": title, "steps": [step]}
    if ours is not None:
        case["setup"] = ours_doc
    return case


def cases():
    bed = "yatak-odasi/yatak-odasi.dxf"
    dining = "yemek/yemek-takimi.dxf"
    toilet = "islak-hacim/klozet.dxf"
    basin = "islak-hacim/lavabo.dxf"
    out = [
        ok_case("milimetre DXF'in blokları metre çizime: gerçek boyları, kilim kendi santimetresinde; tek adımda geri alınır",
                bed),
        ok_case("aynı DXF milimetre çizime: boylar metrede aynı, çizimin birimi değiştirmez", bed, ours=setup("mm")),
        ok_case("yalnız adı verilen blok gelir", bed, names=["Yatak 90"]),
        ok_case("kitaplığın kategorisi alınan blokların kartına yazılır", bed,
                category="Kitaplık/yatak-odasi/yatak-odasi"),
        ok_case("çizimin tamamı tek blok: dosyanın birimiyle, taban noktası nesnelerin sol altı, iç bloklar önce",
                bed, mode="drawing"),
        ok_case("çizimin tamamı adıyla ve kategorisiyle; MOBILYA katmanı çizimde varsa yeni katman açılmaz", bed,
                mode="drawing", name="Yatak odası takımı", category="Kitaplık/yatak-odasi", ours=setup(mobilya=True)),
        ok_case("santimetre DXF'te masa dört sandalyesiyle: yerleştirdiği blok da gelir, ikisi ters dönük", dining,
                names=["Yemek masası"]),
        ok_case("yemek takımı milimetre çizime tek blok", dining, mode="drawing", ours=setup("mm")),
        ok_case("birim bildirmeyen dosya seçilen milimetreyle: 38 × 55 cm; okunamayan 3DSOLID söylenir", toilet,
                source_unit="mm"),
        ok_case("birim bildirmeyen dosya santimetre seçilince on kat: seçilen birim neyse o", toilet, source_unit="cm"),
        ok_case("inç DXF'in lavabosu: önündeki yay derinliği verir", basin),
        ok_case("dosyanın bildirdiği milimetre yerine seçilen santimetre: blokların boyu ona göre, kilim kendi biriminde",
                bed, source_unit="cm"),
        ok_case("çizimde aynı adlı blok: korunur ve söylenir, öbürleri gelir", bed, ours=setup(bed=True)),
        ok_case("replace ile çizimin yatağı dosyanınkiyle yeniden tanımlanır, kimliği kalır, kategori yazılır", bed,
                names=["Yatak 90"], replace=True, category="Kitaplık/yatak-odasi", ours=setup(bed=True)),
        ok_case("plan okur, hiçbir şey yazmaz; execute aynı cevabı verir", bed, names=["Komodin"], plan=True),
        ok_case("DWG ikizi: aynı bloklar, aynı boylar ve birim kararı", bed, dwg=True),
        ok_case("DWG ikizi tek blok olarak", bed, mode="drawing", dwg=True),
        ok_case("birim bildirmeyen DWG ikizi seçilen milimetreyle", toilet, source_unit="mm", dwg=True),
        ok_case("yemek takımının DWG ikizi: masa ve sandalyeleri", dining, names=["Yemek masası"], dwg=True),
        failed_case("birim bildirmeyen dosya sourceUnit'siz: hiçbir şey alınmaz", f"{FIX}/{toilet}",
                    "source_unit_unknown",
                    "klozet.dxf birimini bildirmiyor; blokların boyu bilinmeden alınmaz. Dosyanın birimini verin "
                    "(sourceUnit: mm, cm, m, in, ft).", "sourceUnit", extra={"mode": "blocks"}),
        failed_case("birim bildirmeyen dosya tek blok olarak da reddedilir; doğrulama da aynı", f"{FIX}/{toilet}",
                    "source_unit_unknown",
                    "klozet.dxf birimini bildirmiyor; blokların boyu bilinmeden alınmaz. Dosyanın birimini verin "
                    "(sourceUnit: mm, cm, m, in, ft).", "sourceUnit", op="validate"),
        failed_case("birim bildirmeyen DWG ikizi sourceUnit'siz", f"{FIX_DWG}/klozet.dwg", "source_unit_unknown",
                    "klozet.dwg birimini bildirmiyor; blokların boyu bilinmeden alınmaz. Dosyanın birimini verin "
                    "(sourceUnit: mm, cm, m, in, ft).", "sourceUnit"),
        failed_case("KentOS çizimi birimini taşır: sourceUnit verilmez", "$fixture:blocks/v1/ofis-bloklari.kcad",
                    "invalid_input",
                    "ofis-bloklari.kcad bir KentOS çizimi; birimini kendisi taşır. sourceUnit yalnız DXF ve DWG "
                    "dosyasında verilir.", "sourceUnit", extra={"sourceUnit": "mm"}),
        failed_case("kategorinin boş parçası", f"{FIX}/{bed}", "invalid_block_card",
                    "Kategori kurallarına uymuyor: “Kitaplık//Mobilya” kategorisinin bir parçası boş; “/” iki ad "
                    "arasında olur.", "category", extra={"category": "Kitaplık//Mobilya"}),
        failed_case("kategorinin başında boşluk", f"{FIX}/{bed}", "invalid_block_card",
                    "Kategori kurallarına uymuyor: “ Mobilya” kategorisi boş ya da başında, sonunda boşluk var.",
                    "category", extra={"category": " Mobilya"}),
        failed_case("göreli DXF yolu", "kitaplik/yatak-odasi.dxf", "invalid_file_path",
                    "“kitaplik/yatak-odasi.dxf” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad).",
                    "path"),
        failed_case("olmayan DWG", f"{FIX_DWG}/yok.dwg", "file_not_found", "$any", "path"),
        failed_case("desteklenmeyen DWG imzası", "$fixture:formats/v1/dwg/unsupported.dwg", "unsupported_cad_version",
                    "$any", "path", extra={"sourceUnit": "mm"}),
        failed_case("kesik DWG", "$fixture:formats/v1/dwg/truncated.dwg", "import_unreadable", "$any", "path",
                    extra={"sourceUnit": "mm"}),
        failed_case("DXF'te olmayan blok adı", f"{FIX}/{bed}", "unknown_block",
                    "“Gardırop” bloğu yatak-odasi.dxf içinde yok.", "names[1]",
                    extra={"mode": "blocks", "names": ["Yatak 90", "Gardırop"]}),
    ]
    return out


def build():
    doc = {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "file.import.block",
        "commandVersion": 1,
        "title": "Dosyadan blok al: DXF ve DWG kitaplığı",
        "note": __doc__.strip(),
        "setup": setup(),
        "cases": cases(),
    }
    return json.dumps(doc, ensure_ascii=False, indent=2) + "\n"


def files():
    return {LIB / rel: dxf(f) for rel, f in LIBRARY.items()}


# ── What KentOS wrote (KENTOS_WRITE_LIBRARY=1 cargo test -p kentos-headless --test headless block_library) ──

def dxf_pairs(text):
    lines = text.split("\n")
    return [(lines[i].strip(), lines[i + 1].strip()) for i in range(0, len(lines) - 1, 2)]


def records(pairs, kind):
    """Every record of `kind` (its groups, first value of each code kept) in file order."""
    out = []
    i = 0
    while i < len(pairs):
        if pairs[i] == ("0", kind):
            fields, points, j = {}, [], i + 1
            while j < len(pairs) and pairs[j][0] != "0":
                code, value = pairs[j]
                fields.setdefault(code, value)
                if code == "10":
                    points.append([float(value), None])
                elif code == "20" and points and points[-1][1] is None:
                    points[-1][1] = float(value)
                j += 1
            fields["$points"] = points
            out.append(fields)
            i = j
        else:
            i += 1
    return out


def block_box(pairs, name):
    """The box of the BLOCK `name`'s lines and polylines, in the file's numbers."""
    box, inside = None, False
    i = 0
    while i < len(pairs):
        code, value = pairs[i]
        if (code, value) == ("0", "BLOCK"):
            j, block = i + 1, None
            while pairs[j][0] != "0":
                if pairs[j][0] == "2":
                    block = pairs[j][1]
                j += 1
            inside = block == name
            i = j
            continue
        if (code, value) == ("0", "ENDBLK"):
            inside = False
        if inside and code == "0" and value in ("LINE", "LWPOLYLINE"):
            j, xs, ys = i + 1, [], []
            while pairs[j][0] != "0":
                c, v = pairs[j]
                if c in ("10", "11"):
                    xs.append(float(v))
                if c in ("20", "21"):
                    ys.append(float(v))
                j += 1
            box = union(box, [min(xs), min(ys), max(xs), max(ys)])
            i = j
            continue
        i += 1
    return box


def check_written():
    """The bed placed turned a quarter and mirrored, written to DXF by a metre and a millimetre plan (ADR 0726 §5):
    $INSUNITS and the block's record the file's unit (ADR 0315 §4: a block's numbers in the file's unit), its
    definition 0.90 × 2.00 m in those numbers, its INSERT mirrored in its x axis (42 −1) and turned 90°, standing
    2.00 × 0.90 m on the ground at (10, 20) m."""
    out = ROOT / "fixtures/blocks/v1/out"
    for name, code, f in (("daire-m.dxf", 6, 1.0), ("daire-mm.dxf", 4, 1000.0)):
        pairs = dxf_pairs((out / name).read_text())
        insunits = next(int(pairs[k + 1][1]) for k, (c, v) in enumerate(pairs) if (c, v) == ("9", "$INSUNITS"))
        assert insunits == code, (name, insunits)
        rec = next(r for r in records(pairs, "BLOCK_RECORD") if r.get("2") == "Yatak 90")
        assert int(rec["70"]) == code, (name, rec)
        box = block_box(pairs, "Yatak 90")
        want = [-0.45 * f, 0.0, 0.45 * f, 2.0 * f]
        assert all(abs(a - b) < 1e-9 * f for a, b in zip(box, want)), (name, box, want)
        ins = next(r for r in records(pairs, "INSERT") if r.get("2") == "Yatak 90")
        sx, sy, turn = float(ins.get("41", 1)), float(ins.get("42", 1)), float(ins.get("50", 0))
        px, py = ins["$points"][0]
        assert (sx, sy, turn) == (1.0, -1.0, 90.0), (name, sx, sy, turn)
        corners = [turned(x * sx, y * sy, turn) for x in (box[0], box[2]) for y in (box[1], box[3])]
        xs, ys = [px + c[0] for c in corners], [py + c[1] for c in corners]
        width, depth = max(xs) - min(xs), max(ys) - min(ys)
        assert abs(width - 2.0 * f) < 1e-9 * f and abs(depth - 0.9 * f) < 1e-9 * f, (name, width, depth)
        assert abs(min(xs) - 10.0 * f) < 1e-9 * f and abs(max(xs) - 12.0 * f) < 1e-9 * f, (name, xs)
    print("DXF: iki dosya, yatak 2.00 × 0.90 m, aynalı ve dönük, birimi dosyanınki")


def main():
    if "--written" in sys.argv:
        check_written()
        return
    check = "--check" in sys.argv
    text = build()
    stale = []
    for path, content in files().items():
        if check:
            if not path.exists() or path.read_text() != content:
                stale.append(str(path.relative_to(ROOT)))
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
    if check:
        if not OUT.exists() or OUT.read_text() != text:
            stale.append(str(OUT.relative_to(ROOT)))
        if stale:
            raise SystemExit("eski: " + ", ".join(stale))
    else:
        OUT.write_text(text)
    print(f"file.import.block.library: {len(json.loads(text)['cases'])} durum, {len(LIBRARY)} DXF")


if __name__ == "__main__":
    main()
