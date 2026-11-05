#!/usr/bin/env python3
"""CAD-ADV-17's 2D file set (docs/adr/1061 §6), written by an independent DXF writer (ezdxf, not KentOS), and the
independent reading of what KentOS writes back.

    .run/py/bin/python scripts/fixtures/exchange_set_cases.py            # writes fixtures/exchange/v1/set.dxf
    .run/py/bin/python scripts/fixtures/exchange_set_cases.py --check    # compares, writes nothing
    .run/py/bin/python scripts/fixtures/exchange_set_cases.py --roundtrip  # open → edit → export with KentOS's Python
                                                                             module, then read the export with ezdxf

The set: nested blocks with their own units (a door in millimetres inside a storey in metres), a bulged and wide
polyline, an ellipse and a rational NURBS, a bulged polyline and a straight wide one under a nonuniform block scale, a
line and a point with elevations, a complex line type (a text in its pattern), a pattern hatch, an aligned dimension
and two sheets (paper space layouts with a viewport each). ezdxf's fixed metadata makes the file the same every run.

The round trip checks with ezdxf, never with KentOS's reader: the export holds an ellipse, a NURBS of degree 2 with
weights where the nonuniform arc was (its points on the mapped ellipse), the wide straight edge's widths times
|det M|/|M t|, the line's and point's elevations, the hatch, the dimension, two layouts; and the source file is
unchanged. Not an AutoCAD test (CAD-IO-20 was left out by the owner).
"""
import hashlib
import os
import math
import sys
import tempfile
from pathlib import Path

import ezdxf
from ezdxf import units

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "exchange" / "v1" / "set.dxf"


def build():
    ezdxf.options.write_fixed_meta_data_for_testing = True
    doc = ezdxf.new("R2018", setup=True)
    doc.units = units.M
    msp = doc.modelspace()
    kapi = doc.blocks.new("KAPI")
    kapi.block_record.dxf.units = units.MM
    kapi.add_line((0, 0), (900, 0))
    kapi.add_arc((0, 0), 900, 0, 90)
    kat = doc.blocks.new("KAT")
    kat.block_record.dxf.units = units.M
    kat.add_lwpolyline([(0, 0), (5, 0), (5, 4), (0, 4)], close=True)
    kat.add_blockref("KAPI", (1, 0))
    msp.add_blockref("KAT", (100, 0))
    egik = doc.blocks.new("EGIK")
    egik.add_lwpolyline([(0, 0, 0, 0, 1.0), (2, 0, 0, 0, 0), (2, 2, 0, 0, 0)], format="xyseb")
    msp.add_blockref("EGIK", (0, 50), dxfattribs={"xscale": 2, "yscale": 0.5})
    genis = doc.blocks.new("GENIS")
    genis.add_lwpolyline([(0, 0), (4, 0), (4, 3)], dxfattribs={"const_width": 0.2})
    msp.add_blockref("GENIS", (20, 50), dxfattribs={"xscale": 3, "yscale": 0.5, "rotation": 30})
    msp.add_lwpolyline([(0, 0, 0.1, 0.1, 0.5), (10, 0, 0.2, 0.2, 0), (10, 5, 0, 0, 0)], format="xyseb")
    msp.add_ellipse((30, 0), major_axis=(4, 1), ratio=0.5)
    msp.add_rational_spline([(40, 0), (42, 3), (45, 0)], [1.0, 0.5, 1.0], degree=2)
    msp.add_line((0, 10, 3.5), (10, 10, 4.25))
    msp.add_point((5, 5, 12.0))
    doc.linetypes.add("GAZ", pattern='A,.5,-.2,["GAS",STANDARD,S=.1,U=0.0,X=-0.1,Y=-.05],-.25',
                      description="Gaz ----GAS----", length=1.0)
    msp.add_line((0, -10), (40, -10), dxfattribs={"linetype": "GAZ"})
    h = msp.add_hatch()
    h.set_pattern_fill("ANSI31", scale=0.5)
    h.paths.add_polyline_path([(0, 20), (10, 20), (10, 30), (0, 30)], is_closed=True)
    msp.add_aligned_dim(p1=(0, 0), p2=(10, 0), distance=2).render()
    for name in ("Pafta A", "Pafta B"):
        lay = doc.layouts.new(name)
        lay.add_viewport(center=(150, 100), size=(200, 150), view_center_point=(20, 10), view_height=60)
    with tempfile.TemporaryDirectory() as d:
        p = Path(d) / "set.dxf"
        doc.saveas(p)
        return p.read_bytes()


def roundtrip():
    import json
    import kentos.cad as cad
    src = OUT.read_bytes()
    with tempfile.TemporaryDirectory() as d:
        d = Path(d)
        source = d / "set.dxf"
        source.write_bytes(src)
        doc = cad.Document.open(str(source))
        out = d / "geri.dxf"
        # The source itself is not written over by an export unasked.
        refused = doc.run("file.export.dxf", {"path": str(source), "overwrite": True, "allowPolylineLoss": True})
        assert refused["status"] == "failed" and refused["error"]["code"] == "source_overwrite", refused
        assert source.read_bytes() == src
        done = doc.run("file.export.dxf", {"path": str(out), "allowPolylineLoss": True})
        assert done["status"] == "completed", done
        back = ezdxf.readfile(out)
        msp = back.modelspace()
        kinds = {e.dxftype() for e in msp}
        assert "ELLIPSE" in kinds and "HATCH" in kinds, kinds
        # The foreign DIMENSION comes as its picture (its lines, ticks and text): the view is kept, the measure is not a
        # KentOS dimension any more (the open says so); its value's text is in the export.
        texts = [t.dxf.text for t in msp.query("TEXT MTEXT") if t.dxftype() == "TEXT"] + [
            t.text for t in msp.query("MTEXT")]
        picture = [e.text for b in ezdxf.readfile(source).blocks if b.name.startswith("*D")
                   for e in b if e.dxftype() == "MTEXT"]
        assert picture and all(p in texts for p in picture), (picture, texts)
        assert len([n for n in back.layouts.names() if n != "Model"]) == 2, back.layouts.names()
        # Opened, the nonuniform blocks stay blocks: the INSERT keeps x×2, y×0.5 and its block's bulge 1 (the
        # analytic source, editable as it was).
        egik = [i for i in msp.query("INSERT") if i.dxf.name.upper().endswith("EGIK")]
        assert egik and abs(egik[0].dxf.xscale - 2) < 1e-12 and abs(egik[0].dxf.yscale - 0.5) < 1e-12, egik
        bulges = [b for pl in back.blocks.get(egik[0].dxf.name).query("LWPOLYLINE") for b in pl.get_points("b")]
        assert any(abs(b[0] - 1.0) < 1e-12 for b in bulges), bulges
        # Imported with the blocks exploded, then exported: the arc is a degree 2 rational SPLINE on the image of the
        # half circle from (0, 0) to (2, 0) about (1, 0) under x×2, y×0.5 at (0, 50): centre (2, 50), semi-axes 2 and
        # 0.5; its first two thirds (the two quarter pieces) on it, the last third the straight edge.
        setup = json.loads((ROOT / "fixtures/commands/v1/file.import.dxf.json").read_text())["setup"]
        flat = cad.Document.from_bytes(json.dumps(setup).encode())
        got = flat.run("file.import.dxf", {"path": str(source), "explodeBlocks": True, "allowPolylineLoss": True})
        assert got["status"] == "completed", got
        out2 = d / "patlak.dxf"
        done2 = flat.run("file.export.dxf", {"path": str(out2), "allowPolylineLoss": True})
        assert done2["status"] == "completed", done2
        msp2 = ezdxf.readfile(out2).modelspace()
        best = 0
        for sp in msp2.query("SPLINE"):
            if sp.dxf.degree != 2 or not sp.weights or min(sp.weights) > 1 - 1e-9:
                continue
            tool = sp.construction_tool()
            hits = 0
            for k in range(17):
                q = tool.point(k / 16 * tool.max_t)
                if abs(((q.x - 2) / 2) ** 2 + ((q.y - 50) / 0.5) ** 2 - 1) < 1e-9:
                    hits += 1
            best = max(best, hits)
        assert best >= 11, f"no exact elliptic arc in the export ({best} of 17 points)"
        # The wide straight polyline under x×3, y×0.5, 30°: |det| = 1.5; the edge along x: |M t| = 3 → 0.1; along y:
        # |M t| = 0.5 → 0.6.
        widths = [w for pl in msp2.query("LWPOLYLINE") for v in pl.get_points("se") for w in v if w]
        for want in (0.2 * 1.5 / 3, 0.2 * 1.5 / 0.5):
            assert any(abs(w - want) < 1e-9 for w in widths), (want, widths)
        # Elevations kept.
        assert any(abs(l.dxf.start.z - 3.5) < 1e-9 and abs(l.dxf.end.z - 4.25) < 1e-9 for l in msp.query("LINE")), "line z"
        assert any(abs(p.dxf.location.z - 12.0) < 1e-9 for p in msp.query("POINT")), "point z"
        print(json.dumps({"exported": done["output"]["objects"], "kinds": sorted(kinds),
                          "report": len(done["output"]["report"].get("notes", []))}, ensure_ascii=False))


def main():
    # ezdxf walks some sets while writing: with Python's random string hashing the order (and so a few bytes) can
    # differ between runs. One fixed seed makes the file the same every time.
    if os.environ.get("PYTHONHASHSEED") != "0":
        os.execve(sys.executable, [sys.executable, *sys.argv], {**os.environ, "PYTHONHASHSEED": "0"})
    if "--roundtrip" in sys.argv[1:]:
        roundtrip()
        return
    data = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_bytes() != data:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"set.dxf tutarlı ({hashlib.sha256(data).hexdigest()[:12]})")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(data)
    print(f"set.dxf yazıldı ({len(data)} bayt)")


main()
