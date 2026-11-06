#!/usr/bin/env python3
"""A DIMSTYLE whose text style (DIMTXSTY) is an SHX style (docs/adr/1023), written by an independent DXF writer
(ezdxf, not KentOS), and what the reader must keep, worked out from how the file is built.

    .run/py/bin/python scripts/fixtures/dimension_text_style_dxf.py           # writes the two files
    .run/py/bin/python scripts/fixtures/dimension_text_style_dxf.py --check   # compares, writes nothing

Writes fixtures/exchange/v1/dimension-text-style.dxf and .json. The file has two text styles, A-OLCU (font
romans.shx) and A-NOT (no text follows either), and two dimension styles: OFIS (DIMTXSTY → A-OLCU) and DUZ (DIMTXSTY
→ Standard). One ordinate dimension follows each. The reader must keep A-OLCU as a project text style with its SHX
file although no text follows it, write OFIS's value in it (`textStyle` = “A-OLCU”), give DUZ none (Standard is
KentOS's own typeface), and leave A-NOT out (nothing follows it).
"""
import json
import sys
from pathlib import Path

import ezdxf

ROOT = Path(__file__).resolve().parents[2]
DXF = ROOT / "fixtures" / "exchange" / "v1" / "dimension-text-style.dxf"
EXPECTED = ROOT / "fixtures" / "exchange" / "v1" / "dimension-text-style.json"


def build():
    ezdxf.options.write_fixed_meta_data_for_testing = True
    doc = ezdxf.new("R2018", setup=False)
    doc.styles.add("A-OLCU", font="romans.shx")
    doc.styles.add("A-NOT", font="simplex.shx")
    ofis = doc.dimstyles.new("OFIS")
    ofis.dxf.dimtxsty = "A-OLCU"
    ofis.dxf.dimtxt = 2.5
    duz = doc.dimstyles.new("DUZ")
    duz.dxf.dimtxsty = "Standard"
    duz.dxf.dimtxt = 2.0
    msp = doc.modelspace()
    # Ordinate dimensions: another program's come in as KentOS's own (docs/adr/0147 §8); a linear one is drawn
    # from its block.
    msp.add_ordinate_x_dim(feature_location=(10, 0), offset=(0, 3), dimstyle="OFIS").render()
    msp.add_ordinate_y_dim(feature_location=(0, 8), offset=(3, 0), dimstyle="DUZ").render()
    out = DXF.with_suffix(".tmp")
    doc.saveas(out)
    data = out.read_bytes()
    out.unlink()
    expected = {
        "textStyles": [{"name": "A-OLCU", "fontFile": "romans.shx"}],
        "dimensionStyles": [{"name": "OFIS", "textStyle": "A-OLCU"}, {"name": "DUZ", "textStyle": None}],
    }
    return data, json.dumps(expected, ensure_ascii=False, indent=1) + "\n"


def main():
    data, text = build()
    if "--check" in sys.argv[1:]:
        # ezdxf's object order is not stable between runs (Python's hash order): the file on disk is read back and
        # checked for what it was built with, the expectations compared as text.
        same = EXPECTED.exists() and EXPECTED.read_text("utf-8") == text and DXF.exists()
        if same:
            disk = ezdxf.readfile(DXF)
            same = (disk.dimstyles.get("OFIS").dxf.dimtxsty == "A-OLCU"
                    and disk.dimstyles.get("DUZ").dxf.dimtxsty == "Standard"
                    and disk.styles.get("A-OLCU").dxf.font == "romans.shx"
                    and len(disk.modelspace().query("DIMENSION")) == 2)
        print("güncel" if same else "güncel değil")
        return 0 if same else 1
    DXF.write_bytes(data)
    EXPECTED.write_text(text, "utf-8")
    print(f"yazıldı: {DXF.relative_to(ROOT)}, {EXPECTED.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
