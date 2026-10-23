"""Typed paper schema defaults are fresh objects and preserve the core wire format."""
import unittest
import kentos.cad as cad

class SheetOutputTypes(unittest.TestCase):
    def test_paper_defaults_are_typed_and_independent(self):
        a = cad.TextPaperItemKind(content="Kat planı")
        b = cad.TextPaperItemKind(content="Detay")
        self.assertEqual(a.style.to_json(), {"color":"#000000","font":"barlow","italic":False,"size":2000,"weight":400})
        a.style.color = "#ff0000"
        self.assertEqual(b.style.color, "#000000")
        self.assertEqual(cad.TextPaperItemKind.from_json(b.to_json()).to_json(),b.to_json())

    def test_raw_ops_preserve_unknown_fields_for_the_core_to_reject(self):
        ops = [{"op":"removeSheet","id":"a","unknown":17}]
        self.assertEqual(cad.SheetsEdit(ops=ops).to_json()["ops"], ops)
        self.assertEqual(cad.SheetsEdit.from_json({"ops": [{"op":"removeSheet"}, {}]}).to_json()["ops"], [{"op":"removeSheet"}, {}])
        typed = cad.RemoveSheetPaperOp(id="a")
        self.assertEqual(cad.SheetsEdit(ops=[typed]).to_json()["ops"], [{"op":"removeSheet","id":"a"}])
