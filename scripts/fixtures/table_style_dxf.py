#!/usr/bin/env python3
"""DXF table styles (docs/adr/1128), with an independent DXF side: ezdxf, not KentOS.

    /path/to/.run/py/bin/python -I scripts/fixtures/table_style_dxf.py           # writes the files
    /path/to/.run/py/bin/python -I scripts/fixtures/table_style_dxf.py --check   # writes nothing; checks

1. Import: fixtures/exchange/v1/table-styles.dxf and .json. ezdxf writes the drawing (a STYLE A-TABLO on romans.shx,
   a layer, the table's anonymous block *T1 with its lines and words). ezdxf 1.4 cannot make a TABLESTYLE or an
   ACAD_TABLE (its TableStyle is a stub), so this script adds them to ezdxf's text from Autodesk's DXF reference for
   TABLESTYLE and ACAD_TABLE: two TABLESTYLE objects in the ACAD_TABLESTYLE dictionary (CETVEL, which the
   ACAD_TABLE's 342 names, and BOS, which nothing follows), their cell styles in the order data, title, heading
   (Open Design's specification as LibreDWG's `dwg2.spec` reads it), and $HANDSEED moved past the new handles.
   The .json says what the reader must keep, worked out here from the groups: sizes from metres to paper mm at
   1:100, ACI 1–6 as their RGB, a border look its bottom border's, the table's margins every cell style's, and what
   it cannot keep (a top aligned title, the title's unequal borders, the heading's hidden border, a suppressed
   heading).
2. Export: fixtures/exchange/v1/table-styles-export.json is the writer's input (a project with two table styles and
   a table that follows one). The Rust test writes table-styles-export.dxf from it; `--check` reads that file with
   ezdxf and checks the TABLESTYLE objects' groups against what the ADR's rules give for the input, their KENTOS
   data against the input's styles, and the table INSERT's reference against the CETVEL object's handle.

ezdxf's object order is not stable between runs (Python's hash order): `--check` reads the files back instead of
comparing bytes.
"""
import io
import json
import sys
from pathlib import Path

import ezdxf

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "exchange" / "v1"
DXF = DIR / "table-styles.dxf"
EXPECTED = DIR / "table-styles.json"
EXPORT_INPUT = DIR / "table-styles-export.json"
EXPORT = DIR / "table-styles-export.dxf"
SCALE = 100.0
ACI = {1: "#ff0000", 2: "#ffff00", 3: "#00ff00", 4: "#00ffff", 5: "#0000ff", 6: "#ff00ff"}


def tags(pairs):
    return "".join(f"{code:>3}\n{value}\n" for code, value in pairs)


def cell(style, height, align, text, fill, fill_on, weights, shown, colours):
    out = [(7, style), (140, height), (170, align), (62, text), (63, fill), (283, fill_on), (90, 512), (91, 0)]
    out += [(274 + i, w) for i, w in enumerate(weights)]
    out += [(284 + i, v) for i, v in enumerate(shown)]
    out += [(64 + i, c) for i, c in enumerate(colours)]
    return out


def table_style(handle, owner, name, header_suppressed, cells):
    head = [(0, "TABLESTYLE"), (5, handle), (102, "{ACAD_REACTORS"), (330, owner), (102, "}"), (330, owner),
            (100, "AcDbTableStyle"), (280, 0), (3, name), (70, 0), (71, 0), (40, 0.15), (41, 0.1), (280, 0),
            (281, header_suppressed)]
    return head + [g for c in cells for g in c]


def build():
    ezdxf.options.write_fixed_meta_data_for_testing = True
    doc = ezdxf.new("R2018", setup=False)
    doc.header["$INSUNITS"] = 6
    doc.styles.add("A-TABLO", font="romans.shx")
    doc.layers.add("TABLO")
    block = doc.blocks.new_anonymous_block(type_char="T")
    for a, b in [((0, 0), (5, 0)), ((0, -1), (5, -1)), ((0, -2), (5, -2)), ((0, 0), (0, -2)), ((5, 0), (5, -2))]:
        block.add_line(a, b, dxfattribs={"layer": "0"})
    block.add_text("Liste", dxfattribs={"insert": (0.2, -0.7), "height": 0.35, "layer": "0"})
    block.add_text("101", dxfattribs={"insert": (0.2, -1.7), "height": 0.25, "layer": "0"})
    stream = io.StringIO()
    doc.write(stream)
    text = stream.getvalue()
    lines = text.split("\n")
    seed_at = lines.index("$HANDSEED") + 2
    seed = int(lines[seed_at], 16)
    cetvel, bos, table = (f"{seed:X}", f"{seed + 1:X}", f"{seed + 2:X}")
    lines[seed_at] = f"{seed + 3:X}"
    text = "\n".join(lines)
    # The ACAD_TABLESTYLE dictionary's entries.
    owner = doc.rootdict["ACAD_TABLESTYLE"].dxf.handle
    dictionary = f"  0\nDICTIONARY\n  5\n{owner}\n"
    at = text.index(dictionary)
    end = text.index("  0\n", at + len(dictionary))
    text = text[:end] + tags([(3, "CETVEL"), (350, cetvel), (3, "BOS"), (350, bos)]) + text[end:]
    data = cell("A-TABLO", 0.25, 6, 1, 7, 0, [25] * 6, [1] * 6, [5] * 6)
    title = cell("Standard", 0.35, 2, 0, 4, 1, [25, 25, 50, 25, 25, 25], [1] * 6, [0] * 6)
    heading = cell("Standard", 0.25, 5, 3, 7, 0, [-2] * 6, [0, 1, 1, 1, 1, 1], [0] * 6)
    objects = tags(table_style(cetvel, owner, "CETVEL", 1, [data, title, heading]))
    objects += tags(table_style(bos, owner, "BOS", 0, [cell("Standard", 0.18, 5, 0, 7, 0, [-2] * 6, [1] * 6, [0] * 6)] * 3))
    end = text.rindex("  0\nENDSEC")
    text = text[:end] + objects + text[end:]
    # The ACAD_TABLE in model space, drawn by *T1, following CETVEL.
    model = ezdxf_record(doc, "*Model_Space")
    t_record = ezdxf_record(doc, block.name)
    entity = tags([(0, "ACAD_TABLE"), (5, table), (330, model), (100, "AcDbEntity"), (8, "TABLO"),
                   (100, "AcDbBlockReference"), (2, block.name), (10, 100.0), (20, 200.0), (30, 0.0),
                   (100, "AcDbTable"), (280, 0), (342, cetvel), (343, t_record), (11, 1.0), (21, 0.0), (31, 0.0),
                   (90, 22), (91, 2), (92, 1), (141, 1.0), (141, 1.0), (142, 5.0)])
    entities = text.index("ENTITIES\n")
    end = text.index("  0\nENDSEC", entities)
    text = text[:end] + entity + text[end:]
    mm = lambda metres: metres * 1000.0 / SCALE
    margins = {"horizontalMargin": mm(0.15), "verticalMargin": mm(0.1)}
    expected = {
        "scale": SCALE,
        "tableStyles": [{
            "name": "CETVEL",
            "data": {"textStyle": "A-TABLO", "height": mm(0.25), "align": "right", "color": ACI[1], **margins,
                     "borderWeight": 0.25, "borderColor": ACI[5]},
            "title": {"height": mm(0.35), "align": "center", "fill": ACI[4], **margins, "borderWeight": 0.5},
            "header": {"height": mm(0.25), "align": "center", "color": ACI[3], **margins},
        }],
        "textStyles": [{"name": "A-TABLO", "fontFile": "romans.shx"}],
        "decisions": ["table_style_from_acad_table", "table_style_lossy", "table_style_unused_left_out"],
        "lost": ["başlık hücresinin üst ya da alt hizası", "başlık hücresinin kenarlarının farklı",
                 "sütun başı hücresinin gizli kenarları", "sütun başının gizlenmesi"],
        "unused": ["BOS"],
    }
    return text.encode("utf-8"), json.dumps(expected, ensure_ascii=False, indent=1) + "\n"


def ezdxf_record(doc, name):
    return doc.block_records.get(name).dxf.handle


# ── export ────────────────────────────────────────────────────────────────────

EXPORT_STYLES = [
    {"id": "ofis-cetvel", "name": "CETVEL",
     "title": {"align": "center", "height": 3.5, "fill": "#ff0000", "borderWeight": 0.5, "borderColor": "#0000ff"},
     "header": {"height": 2.5, "color": "#00ff00"},
     "data": {"textStyle": "A-TABLO", "align": "right", "height": 2.0, "horizontalMargin": 1.5,
              "verticalMargin": 1.0, "borderWeight": 0.25}},
    {"id": "yerel-sade", "name": "SADE", "data": {"align": "left", "color": "#ff00ff"}},
]


def export_input():
    table = {"kind": "table", "id": 1, "layerId": "tablo", "attrs": {}, "p": {"x": 10.0, "y": 20.0}, "rotation": 0.0,
             "height": 0.25, "rows": [0.6, 0.5], "columns": [2.0, 2.0], "cells": [["Ada", "Alan"], ["101", "512"]],
             "header": True, "tableStyle": "ofis-cetvel"}
    return {"entities": [table], "layers": [{"id": "tablo", "name": "TABLO", "path": ["TABLO"], "color": "ink", "visible": True,
                                            "locked": False, "lineType": "continuous", "lineWeight": 0.25}],
            "scale": SCALE, "lengthDecimals": 3, "grads": False,
            "textStyles": [{"id": "t1", "name": "A-TABLO", "font": "barlow", "fontFile": "romans.shx"}],
            "tableStyles": EXPORT_STYLES}


def expected_groups(style):
    """What the ADR's rules write for `style`: drawing units (metres) at 1:100, ACI, the cell styles data, title,
    heading; one margin pair, the data cell's first. The input's colours are ACI 1–6 exactly."""
    k = SCALE / 1000.0
    cells = [style.get("data") or {}, style.get("title") or {}, style.get("header") or {}]
    h = next((c["horizontalMargin"] for c in cells if "horizontalMargin" in c), 1.5)
    v = next((c["verticalMargin"] for c in cells if "verticalMargin" in c), 1.5)
    nearest = {v: k for k, v in ACI.items()}
    out = {"40": h * k, "41": v * k, "cells": []}
    for i, c in enumerate(cells):
        align = {"left": 4, "center": 5, "right": 6}.get(c.get("align"), 4 if i == 0 else 5)
        out["cells"].append({
            "7": "A-TABLO" if c.get("textStyle") == "A-TABLO" else "Standard",
            # Its capitals' height in its style's face (docs/adr/8046 §3): A-TABLO is romans.shx (none), Standard Arial's
            # capital H, 1466 of 2048.
            "140": c.get("height", 6.35 if i == 1 else 4.572) * k * (1.0 if c.get("textStyle") == "A-TABLO" else 1466 / 2048),
            "170": align,
            "62": nearest.get(c.get("color"), 0) if "color" in c else 0,
            "283": 1 if "fill" in c else 0,
            "274": round(c["borderWeight"] * 100) if "borderWeight" in c else -2,
            "64": nearest.get(c.get("borderColor"), 0) if "borderColor" in c else 0,
        })
    return out


def check_export():
    if not EXPORT.exists():
        print(f"{EXPORT.relative_to(ROOT)} yok: Rust sınaması yazar")
        return False
    doc = ezdxf.readfile(EXPORT)
    root = doc.rootdict
    names = dict(root["ACAD_TABLESTYLE"].items())
    ok = sorted(names) == sorted(s["name"] for s in EXPORT_STYLES)
    for s in EXPORT_STYLES:
        obj = names.get(s["name"])
        if obj is None:
            return False
        raw = [(t.code, t.value) for t in obj.xtags.subclasses[1]]
        want = expected_groups(s)
        get = lambda code, items: next(v for c, v in items if c == code)
        ok &= abs(float(get(40, raw)) - want["40"]) < 1e-9 and abs(float(get(41, raw)) - want["41"]) < 1e-9
        starts = [i for i, (c, _) in enumerate(raw) if c == 7]
        if len(starts) != 3:
            return False
        for n, start in enumerate(starts):
            block = raw[start:starts[n + 1] if n + 1 < 3 else len(raw)]
            w = want["cells"][n]
            ok &= get(7, block) == w["7"] and abs(float(get(140, block)) - w["140"]) < 1e-9
            ok &= int(get(170, block)) == w["170"] and int(get(62, block)) == w["62"]
            ok &= int(get(283, block)) == w["283"] and int(get(274, block)) == w["274"]
            ok &= int(get(279, block)) == w["274"] and int(get(64, block)) == w["64"]
        # KentOS's data on the object carries the style exactly.
        data = obj.get_xdata("KENTOS")
        carried = "".join(v for c, v in data if c == 1000 and v not in ("tablestyle",))
        ok &= json.loads(carried) == s
    # The table's INSERT names CETVEL's object.
    insert = doc.modelspace().query("INSERT")[0]
    refs = [v for c, v in insert.get_xdata("KENTOS") if c == 1005]
    ok &= refs == [names["CETVEL"].dxf.handle]
    return ok


def check_import(text):
    if not (EXPECTED.exists() and EXPECTED.read_text("utf-8") == text and DXF.exists()):
        return False
    disk = ezdxf.readfile(DXF)
    found = [e.dxftype() for e in disk.objects if e.dxftype() == "TABLESTYLE"]
    return (len(found) == 2 and len(disk.modelspace().query("ACAD_TABLE")) == 1
            and sorted(dict(disk.rootdict["ACAD_TABLESTYLE"].items())) == ["BOS", "CETVEL"]
            and disk.styles.get("A-TABLO").dxf.font == "romans.shx")


def main():
    data, text = build()
    export = json.dumps(export_input(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        same = check_import(text) and EXPORT_INPUT.read_text("utf-8") == export and check_export()
        print("güncel" if same else "güncel değil")
        return 0 if same else 1
    DXF.write_bytes(data)
    EXPECTED.write_text(text, "utf-8")
    EXPORT_INPUT.write_text(export, "utf-8")
    print(f"yazıldı: {DXF.relative_to(ROOT)}, {EXPECTED.relative_to(ROOT)}, {EXPORT_INPUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
