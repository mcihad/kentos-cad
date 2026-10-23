"""The real typed Python host: common template creation, physical fit, read, save and group undo."""
import unittest
import kentos.cad as cad
from kentos.cad.errors import CommandFailed

class SheetTools(unittest.TestCase):
    def test_create_fit_read_and_group_undo_preserve_model_and_book(self):
        doc=cad.Document.new("Pafta",workspace="cad",drawing_unit="mm")
        line=cad.line.create(doc,layer_id=doc.active_layer,a=(0,0),b=(5,3))
        model=doc.entity(line.uid).entity.to_json()
        with doc.group("Pafta"):
            cad.sheets.create(doc,id="plan",source=cad.SystemSheetTemplateSource(id="sys:cad-mimari"),options={"name":"A-101 Zemin kat","scale":50,"center":{"x":2.5,"y":1.5}})
            read=cad.sheets.set(doc,details=True)
            book=read.book.to_json()
            item=next(i for i in book["sheets"][0]["items"] if i["kind"]["type"]=="map")
            cad.sheets.fit(doc,item=item["id"],uids=[line.uid])
        self.assertEqual(doc.entity(line.uid).entity.to_json(),model)
        after=cad.sheets.set(doc,details=True).book.to_json()
        self.assertEqual(after["sheets"][0]["name"],"A-101 Zemin kat")
        self.assertGreaterEqual(len(cad.sheets.set(doc,details=True).templates),10)
        self.assertIsNotNone(doc.undo())
        self.assertEqual(doc.sheets(),[])
        self.assertEqual(doc.entity(line.uid).entity.to_json(),model)
        self.assertIsNotNone(doc.redo())
        self.assertEqual(cad.sheets.set(doc,details=True).book.to_json(),after)
        reopened=cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(cad.sheets.set(reopened,details=True).book.to_json(),after)

    def test_json_options_are_preserved_for_domain_validation(self):
        doc=cad.Document.new("Pafta",workspace="cad")
        source=cad.SystemSheetTemplateSource(id="sys:cad-mimari")
        with self.assertRaises(CommandFailed) as failed:
            cad.sheets.create(doc,id="bad",source=source,options={"mystery":1})
        self.assertEqual(failed.exception.code,"invalid_options")
        self.assertEqual(doc.sheets(),[])
        raw={"id":"bad","source":{"type":"custom","template":{}},"options":{"mystery":1}}
        self.assertEqual(cad.SheetsCreate.from_json(raw).to_json(),raw)
