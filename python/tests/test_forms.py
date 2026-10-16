"""The project's forms from Python (docs/adr/0401): ``kentos.cad.forms.edit`` makes a layer's form of its fields,
saves, publishes and enforces it as the desktop's Formlar window does; ``kentos.cad.forms.validate`` asks about a
record and ``kentos.cad.forms.submit`` writes one through the form; ``Document.forms`` reads them, and they travel in
the project file."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad


def drawing() -> tuple[cad.Document, str]:
    doc = cad.Document.new("Mahalle", srid=5256)
    cad.layers.edit(doc, operation="add", name="Bina", activate=True)
    layer = next(n.id for n in doc.layers() if n.name == "Bina")
    cad.layers.edit(
        doc,
        operation="fields",
        layer=layer,
        fields=[
            cad.LayerField(
                name="Durum",
                kind="text",
                required=True,
                values=[cad.FieldChoice(code="iyi", label="İyi"), cad.FieldChoice(code="inceleme", label="Ek inceleme gerekli")],
            ),
            cad.LayerField(name="Kat", kind="integer", min="1"),
            cad.LayerField(name="Açıklama", kind="text"),
        ],
    )
    return doc, layer


class FormsTests(unittest.TestCase):
    def test_a_form_is_made_published_used_and_saved_in_the_file(self) -> None:
        doc, layer = drawing()
        made = cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Bina denetimi", layer=layer))
        self.assertTrue(made.changed)
        form = made.form
        assert form is not None
        self.assertEqual(form.id, "bina-denetimi")
        self.assertEqual([i.field for i in form.draft.items], ["Durum", "Kat", "Açıklama"])
        self.assertEqual(made.issues, [])

        # The draft gets a rule: the note is asked only for a building to be looked at again.
        items = list(form.draft.items)
        note = items[2]
        items[2] = cad.FieldFormItem(
            id=note.id, field=note.field, visible_when="Durum = 'inceleme'", required_when="Durum = 'inceleme'"
        )
        saved = cad.forms.edit(doc, edit=cad.UpdateFormsOp(id=form.id, body=cad.FormBody(revision=1, items=items)))
        self.assertEqual(saved.form.draft.revision if saved.form else None, 2)
        published = cad.forms.edit(doc, edit=cad.PublishFormsOp(id=form.id, note="İlk yayın"))
        self.assertEqual(published.forms[0].version, 1)

        poly = cad.polygon.create(doc, pts=[cad.Vec2(0.0, 0.0), cad.Vec2(10.0, 0.0), cad.Vec2(10.0, 8.0)], layer_id=layer)
        uid = poly.uid
        asked = cad.forms.validate(doc, form=form.id, uid=uid, values={"Durum": "inceleme"})
        self.assertFalse(asked.valid)
        self.assertEqual([i.code for i in asked.issues], ["form_required"])
        shown = {s.id: s.shown for s in asked.items}
        self.assertTrue(shown[note.id])

        written = cad.forms.submit(doc, form=form.id, uid=uid, values={"Durum": "İyi", "Kat": "3"})
        self.assertTrue(written.changed)
        self.assertEqual(written.attrs, {"Durum": "iyi", "Kat": "3"})
        self.assertTrue(doc.info().can_undo)

        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "mahalle.kcad"
            doc.save(path)
            again = cad.Document.open(path)
            self.assertEqual([f.id for f in again.forms], [form.id])
            self.assertEqual(again.forms[0].releases[0].version, 1)

    def test_refusals_say_their_codes(self) -> None:
        doc, layer = drawing()
        cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Bina", layer=layer, id="bina"))
        with self.assertRaises(cad.CommandFailed) as unpublished:
            cad.forms.edit(doc, edit=cad.EnforceFormsOp(id="bina", on=True))
        self.assertEqual(unpublished.exception.code, "form_unpublished")
        poly = cad.polygon.create(doc, pts=[cad.Vec2(0.0, 0.0), cad.Vec2(10.0, 0.0), cad.Vec2(10.0, 8.0)], layer_id=layer)
        with self.assertRaises(cad.CommandFailed) as invalid:
            cad.forms.submit(doc, form="bina", draft=True, uid=poly.uid, values={"Kat": "0"})
        self.assertEqual(invalid.exception.code, "form_invalid")
        cad.forms.edit(doc, edit=cad.PublishFormsOp(id="bina"))
        cad.forms.edit(doc, edit=cad.EnforceFormsOp(id="bina", on=True))
        with self.assertRaises(cad.CommandFailed) as guarded:
            cad.entities.set(doc, uids=[poly.uid], operation="attributes", attrs={"Kat": "2"})
        self.assertEqual(guarded.exception.code, "form_rules")


if __name__ == "__main__":
    unittest.main()
