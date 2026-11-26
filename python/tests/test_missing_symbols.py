"""ADR 9317: a sheet whose map shows a layer drawn with a symbol neither the project nor the system has (one of a
desktop's Kitaplığım the project was not saved with) prints that layer's plain look, and the PDF's findings say which
symbol, once (`symbol_missing`); the same drawing holding the symbol says nothing."""
import json
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
SHEET = json.loads((ROOT / "fixtures/sheet/v1/native-preflight.json").read_text())["book"]["sheets"][0]
FILL = {"type": "fill", "layers": [{"id": "0", "type": "simpleFill", "color": "#FF00FF"}]}


def drawing(symbol: str) -> cad.Document:
    """The fixture's sheet over a polygon of the active layer, the layer drawn with `symbol`."""
    doc = cad.Document.new("Sembol", workspace="cad", drawing_unit="mm")
    layer = doc.active_layer
    cad.polygon.create(doc, layer_id=layer, pts=[(0, -1), (5, -1), (5, 1), (0, 1)])
    cad.layers.renderer(doc, layer=layer, renderer={"type": "single", "symbols": {"fill": {"ref": symbol}}})
    cad.sheets.edit(doc, ops=[dict(op="addSheet", sheet=SHEET)])
    return doc


class MissingSymbols(unittest.TestCase):
    def test_a_symbol_the_print_has_not_is_said_once(self):
        with tempfile.TemporaryDirectory() as folder:
            printed = drawing("u-deneme").export_pdf(Path(folder) / "pafta.pdf")
        said = [f for f in printed.findings if f["code"] == "symbol_missing"]
        self.assertEqual(len(said), 1, printed.findings)
        self.assertIn("“u-deneme” sembolü", said[0]["message"])
        self.assertEqual(said[0]["severity"], "warning")

    def test_a_symbol_of_the_system_is_not_said(self):
        with tempfile.TemporaryDirectory() as folder:
            printed = drawing("temel.alan.dolu").export_pdf(Path(folder) / "pafta.pdf")
        self.assertFalse([f for f in printed.findings if f["code"] == "symbol_missing"], printed.findings)


if __name__ == "__main__":
    unittest.main()
