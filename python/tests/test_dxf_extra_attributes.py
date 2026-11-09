"""Tanımı olmayan öznitelikler DXF'te (docs/adr/3980; plan/map-cad.md MAP-17), read by an independent reader (ezdxf).

A block without attribute definitions inserted with attributes of its own:

- a name that is a DXF tag as it is with a value of one line goes out as an invisible ATTRIB of the INSERT (AutoCAD's
  DXF reference: group 70 bit 1, “attribute is invisible”), so another program sees it: ezdxf reads its tag, value
  and flag;
- a name with a space (a DXF tag “cannot contain spaces”) and a value on two lines stay in KentOS's data alone;
- KentOS reads the file back with every attribute as it was.
"""

from __future__ import annotations

import tempfile
import unittest
import warnings
from pathlib import Path

from kentos import cad

ATTRS = {"Malzeme": "Çelik", "No": "12", "Bakım notu": "2026", "Açıklama": "iki\nsatır"}


def drawing() -> tuple[cad.Document, str]:
    doc = cad.Document.new("Öznitelik", workspace="cad")
    line = cad.line.create(doc, layer_id=doc.active_layer, a=(0, 0), b=(1, 0))
    block = cad.blocks.define(doc, name="Rögar", base=(0, 0), uids=[line.uid], units="m")
    insert = cad.blocks.insert(doc, block=block.block, at=(10, 20), layer_id=doc.active_layer, attrs=ATTRS)
    return doc, insert.uid


class DxfExtraAttributes(unittest.TestCase):
    def test_another_program_sees_them_and_kentos_reads_them_back(self) -> None:
        import ezdxf

        doc, uid = drawing()
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder) / "ogeler.dxf"
            with warnings.catch_warnings():
                warnings.simplefilter("ignore", cad.CommandWarning)
                cad.file.export.dxf(doc, path=str(out))
            external = ezdxf.readfile(out)
            (insert,) = [e for e in external.modelspace().query("INSERT") if e.dxf.name == "Rögar"]
            seen = sorted((a.dxf.tag, a.dxf.text, a.is_invisible) for a in insert.attribs)
            self.assertEqual(seen, [("Malzeme", "Çelik", True), ("No", "12", True)])
            back = cad.Document.open(out)
            inserts = [r for r in back.entities() if r.entity.to_json()["kind"] == "insert"]
            self.assertEqual(len(inserts), 1)
            self.assertEqual(inserts[0].entity.to_json()["attrs"], ATTRS)


if __name__ == "__main__":
    unittest.main()
