"""Tanımlı özniteliğin boşluklu etiketinin geri adı (docs/adr/4800; plan/map-cad.md MAP-17), read and edited by an
independent program (ezdxf).

A block whose attribute definition is named “Kol boyu” (a DXF tag “cannot contain spaces”, the DXF reference),
inserted with its value:

- ezdxf reads the ATTDEF's and the ATTRIB's tag “Kol_boyu”, and KentOS's name in the ATTDEF's own extended data
  (application KENTOS: “tagname”, the tag, the name), no other ATTDEF holding one (another may keep its own height
  there, docs/adr/8046 §3);
- KentOS reads the file back with the definition's tag and the insert's value under “Kol boyu”;
- ezdxf writes the tags in capitals: KentOS still reads “Kol boyu”;
- ezdxf renames the tag in the definition and the ATTRIB: KentOS reads the new tag, the file wins;
- ezdxf drops KentOS's extended data: KentOS reads “Kol_boyu”.
"""

from __future__ import annotations

import tempfile
import unittest
import warnings
from pathlib import Path

from kentos import cad


def drawing() -> cad.Document:
    doc = cad.Document.new("Öznitelik", workspace="cad")
    line = cad.line.create(doc, layer_id=doc.active_layer, a=(0, 0), b=(0, 6))
    block = cad.blocks.define(doc, name="Direk", base=(0, 0), uids=[line.uid], units="m")
    at = lambda y: cad.Vec2(x=0.5, y=y)
    cad.blocks.edit(doc, operation="attributes", block=block.block, attributes=[
        cad.AttributeDefinition(tag="No", p=at(5.0), height=0.4, rotation=0.0, prompt="Direk numarası"),
        cad.AttributeDefinition(tag="Kol boyu", p=at(3.5), height=0.25, rotation=0.0, value="2.5 m"),
    ])
    cad.blocks.insert(doc, block=block.block, at=(10, 20), layer_id=doc.active_layer,
                      attributes={"No": "7", "Kol boyu": "3 m"})
    return doc


def read_back(path: Path) -> tuple[list[str], dict[str, str]]:
    """KentOS's reading: the definition's tags and the insert's attributes."""
    back = cad.Document.open(path)
    (insert,) = [r.entity.to_json() for r in back.entities() if r.entity.to_json()["kind"] == "insert"]
    (direk,) = [b for b in back.blocks() if b.name == "Direk"]
    return [a.tag for a in direk.attributes], insert["attrs"]


class DxfDefinedAttributeNames(unittest.TestCase):
    def test_the_name_comes_back_and_the_file_wins(self) -> None:
        import ezdxf

        doc = drawing()
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder) / "direk.dxf"
            with warnings.catch_warnings():
                warnings.simplefilter("ignore", cad.CommandWarning)
                cad.file.export.dxf(doc, path=str(out))
            external = ezdxf.readfile(out)
            defs = {d.dxf.tag: d for d in external.blocks.get("Direk").query("ATTDEF")}
            self.assertEqual(sorted(defs), ["Kol_boyu", "No"])
            self.assertEqual(defs["Kol_boyu"].get_xdata("KENTOS"),
                             [(1002, "{"), (1000, "tagname"), (1000, "Kol_boyu"), (1000, "Kol boyu"), (1002, "}")])
            # No other ATTDEF holds a name; its own height may be kept there (docs/adr/8046 §3).
            own = defs["No"].get_xdata("KENTOS") if defs["No"].has_xdata("KENTOS") else []
            self.assertNotIn((1000, "tagname"), own)
            (insert,) = external.modelspace().query("INSERT")
            self.assertEqual(sorted((a.dxf.tag, a.dxf.text) for a in insert.attribs), [("Kol_boyu", "3 m"), ("No", "7")])
            self.assertEqual(read_back(out), (["No", "Kol boyu"], {"No": "7", "Kol boyu": "3 m"}))

            def edited(change, name: str) -> Path:
                again = ezdxf.readfile(out)
                block = again.blocks.get("Direk")
                (ins,) = again.modelspace().query("INSERT")
                for e in [*block.query("ATTDEF"), *ins.attribs]:
                    change(e)
                path = Path(folder) / name
                again.saveas(path)
                return path

            def upper(e) -> None:
                e.dxf.tag = e.dxf.tag.upper()

            def rename(e) -> None:
                if e.dxf.tag == "Kol_boyu":
                    e.dxf.tag = "KOL_UZUNLUK"

            def drop(e) -> None:
                if e.dxftype() == "ATTDEF":
                    e.discard_xdata("KENTOS")

            self.assertEqual(read_back(edited(upper, "buyuk.dxf")), (["NO", "Kol boyu"], {"NO": "7", "Kol boyu": "3 m"}))
            self.assertEqual(read_back(edited(rename, "yeni.dxf")),
                             (["No", "KOL_UZUNLUK"], {"No": "7", "KOL_UZUNLUK": "3 m"}))
            self.assertEqual(read_back(edited(drop, "verisiz.dxf")), (["No", "Kol_boyu"], {"No": "7", "Kol_boyu": "3 m"}))


if __name__ == "__main__":
    unittest.main()
