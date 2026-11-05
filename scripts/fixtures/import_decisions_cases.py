#!/usr/bin/env python3
"""The DXF reader's decisions on a drawing as a whole (docs/adr/1010; CAD-ADV-17), written by an independent DXF
writer (ezdxf, not KentOS), and the decisions the file calls for, computed here from how the file is built.

    .run/py/bin/python scripts/fixtures/import_decisions_cases.py           # writes the two files
    .run/py/bin/python scripts/fixtures/import_decisions_cases.py --check   # compares, writes nothing

Writes fixtures/exchange/v1/decisions.dxf and fixtures/exchange/v1/decisions.json. The file, read into a metre
drawing with blocks exploded, calls for:

- unit: the file says millimetres (INSUNITS 4) and the drawing is in metres: converted (`unit_converted`);
- layers: the table's layers no object is on are left out, in the table's order (`layer_unused_left_out`); an object
  on a layer the table lacks makes that layer (`layer_missing_created`); a layer turned off and a frozen one keep
  their state (`layer_state_kept`);
- planes: a circle whose extrusion is (0, 0, −1) is mirrored exactly (`ocs_flipped_mirrored`); an arc in a tilted
  plane (extrusion (0, 1, 1)) is projected (`ocs_tilted_projected`);
- blocks: under an insert of x 3, y 0.5 an open bulged polyline becomes an exact NURBS
  (`block_nonuniform_arcs_nurbs`) and a straight wide one keeps its strips (`block_nonuniform_widths_exact`);
- paper: the empty default Layout1 is left out (`paper_empty_layout_left_out`) and one sheet with one viewport
  comes (`paper_layouts_read`).

The test (`crates/shared/formats/tests/all/import_decisions.rs`) compares topic, code, subject and count, in the order
the report gives (by topic, then as taken). The words are KentOS's own and are not compared here.
"""
import json
import sys
from pathlib import Path

import ezdxf
from ezdxf import units

ROOT = Path(__file__).resolve().parents[2]
DXF = ROOT / "fixtures" / "exchange" / "v1" / "decisions.dxf"
EXPECTED = ROOT / "fixtures" / "exchange" / "v1" / "decisions.json"
TOPICS = ["unit", "ocs", "block", "layer", "paper"]


def build():
    ezdxf.options.write_fixed_meta_data_for_testing = True
    doc = ezdxf.new("R2018")
    doc.units = units.MM
    doc.layers.add("CIZIM")
    doc.layers.add("BOS")
    doc.layers.add("KAPALI").off()
    doc.layers.add("DONUK").freeze()
    doc.layers.add("BOS2")
    msp = doc.modelspace()
    msp.add_line((0, 0), (1000, 0), dxfattribs={"layer": "CIZIM"})
    msp.add_line((0, 500), (1000, 500), dxfattribs={"layer": "KAPALI"})
    msp.add_line((0, 900), (1000, 900), dxfattribs={"layer": "DONUK"})
    # On a layer the table lacks.
    msp.add_line((0, 1200), (1000, 1200), dxfattribs={"layer": "YOK"})
    # Its plane the plan's turned over.
    msp.add_circle((2000, 0), 100, dxfattribs={"layer": "CIZIM", "extrusion": (0, 0, -1)})
    # In a tilted plane.
    msp.add_arc((3000, 0), 100, 0, 90, dxfattribs={"layer": "CIZIM", "extrusion": (0, 1, 1)})
    # A block with an open bulged polyline and a straight wide one, inserted 3 wide and half high.
    egri = doc.blocks.new("EGRI")
    egri.add_lwpolyline([(0, 0, 0, 0, 1), (200, 0, 0, 0, 0), (200, -100, 0, 0, 0)], format="xyseb",
                        dxfattribs={"layer": "0"})
    egri.add_lwpolyline([(0, 300, 20, 20, 0), (400, 300, 40, 20, 0), (400, 500, 0, 0, 0)], format="xyseb",
                        dxfattribs={"layer": "0"})
    msp.add_blockref("EGRI", (5000, 0), dxfattribs={"layer": "CIZIM", "xscale": 3, "yscale": 0.5})
    # One sheet with one viewport; Layout1 stays empty.
    sheet = doc.layouts.new("Pafta-1")
    sheet.page_setup(size=(420, 297), margins=(0, 0, 0, 0), units="mm")
    sheet.add_viewport(center=(210, 148), size=(300, 200), view_center_point=(2500, 0), view_height=3000)
    return doc


def expected(doc):
    used = set()
    for e in doc.modelspace():
        used.add(e.dxf.layer)
    for name in doc.layout_names_in_taborder():
        if name == "Model":
            continue
        for e in doc.layouts.get(name):
            # A viewport's own layer counts as KentOS reads paper objects; the viewport is no object of the model.
            if e.dxftype() != "VIEWPORT":
                used.add(e.dxf.layer)
    table = [l.dxf.name for l in doc.layers]
    unused = [n for n in table if n not in used]
    missing = sorted(n for n in used if n not in table)
    states = []
    for l in doc.layers:
        if l.dxf.name in used and (l.is_off() or l.is_frozen() or l.is_locked() or not l.dxf.plot):
            states.append(l.dxf.name)
    out = [
        {"topic": "unit", "code": "unit_converted", "subject": "Birim", "count": 1},
        {"topic": "ocs", "code": "ocs_flipped_mirrored", "subject": "CIRCLE", "count": 1},
        {"topic": "ocs", "code": "ocs_tilted_projected", "subject": "ARC", "count": 1},
        {"topic": "block", "code": "block_nonuniform_arcs_nurbs", "subject": "LWPOLYLINE", "count": 1},
        {"topic": "block", "code": "block_nonuniform_widths_exact", "subject": "Genişlikli çoklu çizgi", "count": 1},
    ]
    if unused:
        out.append({"topic": "layer", "code": "layer_unused_left_out", "subject": "Katman", "count": 1,
                    "layers": unused})
    for name in missing:
        out.append({"topic": "layer", "code": "layer_missing_created", "subject": name, "count": 1})
    for name in states:
        out.append({"topic": "layer", "code": "layer_state_kept", "subject": name, "count": 1})
    layouts = [n for n in doc.layout_names_in_taborder() if n != "Model"]
    empty = [n for n in layouts if n == "Layout1" and len(doc.layouts.get(n)) == 0]
    for name in empty:
        out.append({"topic": "paper", "code": "paper_empty_layout_left_out", "subject": name, "count": 1})
    kept = [n for n in layouts if n not in empty]
    viewports = sum(1 for n in kept for e in doc.layouts.get(n) if e.dxftype() == "VIEWPORT" and e.dxf.id != 1)
    out.append({"topic": "paper", "code": "paper_layouts_read", "subject": "Pafta", "count": 1,
                "sheets": len(kept), "viewports": viewports})
    out.sort(key=lambda d: TOPICS.index(d["topic"]))
    return out


def main():
    doc = build()
    tmp = DXF.with_suffix(".tmp")
    doc.saveas(tmp)
    text = tmp.read_text(encoding="utf-8")
    tmp.unlink()
    want = json.dumps({"format": "kentos.import-decisions", "version": 1, "decisions": expected(doc)},
                      ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        ok = DXF.read_text(encoding="utf-8") == text and EXPECTED.read_text(encoding="utf-8") == want
        print("tutarlı" if ok else "FARKLI: betiği --check olmadan çalıştırın")
        sys.exit(0 if ok else 1)
    DXF.write_text(text, encoding="utf-8")
    EXPECTED.write_text(want, encoding="utf-8")
    print(f"yazıldı: {DXF.relative_to(ROOT)}, {EXPECTED.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
