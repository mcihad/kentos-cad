"""The project's forms from Python (docs/adr/0401): ``kentos.cad.forms.edit`` makes a layer's form of its fields,
saves, publishes and enforces it as the desktop's Formlar window does; ``kentos.cad.forms.validate`` asks about a
record and ``kentos.cad.forms.submit`` writes one through the form; ``Document.forms`` reads them, and they travel in
the project file."""

from __future__ import annotations

import tempfile
import unittest
import warnings
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

    def test_calculations_defaults_choices_and_a_drawn_object(self) -> None:
        """docs/adr/0402: a calculated field, an expression default reading the moment and the user, a choice filter, a
        soft check; a polygon drawn on the layer of an enforcing form takes its defaults and calculations in its step."""
        doc, layer = drawing()
        cad.layers.edit(
            doc,
            operation="fields",
            layer=layer,
            fields=[
                cad.LayerField(
                    name="Durum",
                    kind="text",
                    values=[cad.FieldChoice(code="iyi", label="İyi"), cad.FieldChoice(code="yikik", label="Yıkık")],
                ),
                cad.LayerField(name="Kat", kind="integer", min="1"),
                cad.LayerField(name="Yükseklik", kind="decimal", scale=2),
                cad.LayerField(name="Taban", kind="decimal", scale=2),
                cad.LayerField(name="Kayıt", kind="text"),
                cad.LayerField(name="Açıklama", kind="text"),
            ],
        )
        body = cad.FormBody(
            revision=1,
            items=[
                cad.FieldFormItem(id="durum", field="Durum", choice_filter="Kat > 3 veya @secenek <> 'yikik'"),
                cad.FieldFormItem(id="kat", field="Kat"),
                cad.FieldFormItem(id="yukseklik", field="Yükseklik", calculate="Kat * 3"),
                cad.FieldFormItem(id="taban", field="Taban", calculate="$alan"),
                cad.FieldFormItem(id="kayit", field="Kayıt", default_expr="@kullanici"),
                cad.FieldFormItem(
                    id="aciklama",
                    field="Açıklama",
                    checks=[cad.FormCheck(rule="uzunluk(Açıklama) >= 10", message="Kısa görünüyor.", warning=True)],
                ),
            ],
        )
        made = cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Bina", layer=layer, id="bina", body=body))
        self.assertEqual([i.code for i in made.issues], ["form_moment_only"])
        cad.forms.edit(doc, edit=cad.PublishFormsOp(id="bina"))
        cad.forms.edit(doc, edit=cad.EnforceFormsOp(id="bina", on=True))

        # Drawn: the document completes the new object in the polygon's own step.
        poly = cad.polygon.create(
            doc, pts=[cad.Vec2(0.0, 0.0), cad.Vec2(20.0, 0.0), cad.Vec2(20.0, 10.0), cad.Vec2(0.0, 10.0)], layer_id=layer,
            attrs={"Kat": "2"},
        )
        drawn = doc.entity(poly.uid).entity
        self.assertEqual(drawn.attrs, {"Kat": "2", "Yükseklik": "6", "Taban": "200"})

        asked = cad.forms.validate(doc, form="bina", uid=poly.uid)
        offered = {s.id: s.choices for s in asked.items}
        self.assertEqual(offered["durum"], ["iyi"])
        self.assertTrue(next(s for s in asked.items if s.id == "yukseklik").read_only)

        with warnings.catch_warnings(record=True) as seen:
            warnings.simplefilter("always")
            written = cad.forms.submit(
                doc, form="bina", uid=poly.uid, defaults=True, values={"Kat": "5", "Durum": "Yıkık", "Açıklama": "Kısa"},
                moment="2026-10-16T14:30:00", user="Ayşe Yılmaz",
            )
        self.assertEqual(
            written.attrs,
            {"Açıklama": "Kısa", "Durum": "yikik", "Kat": "5", "Kayıt": "Ayşe Yılmaz", "Taban": "200", "Yükseklik": "15"},
        )
        notes = [w.message for w in seen if isinstance(w.message, cad.CommandWarning)]
        self.assertEqual([n.code for n in notes], ["form_check_warning"])

        with self.assertRaises(cad.CommandFailed) as calculated:
            cad.entities.set(doc, uids=[poly.uid], operation="attributes", attrs={"Yükseklik": "99"})
        self.assertEqual(calculated.exception.code, "form_rules")
        with self.assertRaises(cad.CommandFailed) as moment:
            cad.forms.validate(doc, form="bina", moment="dün")
        self.assertEqual(moment.exception.code, "invalid_moment")

        # docs/adr/0406 §1: the layer's records by the form; the drawn one has no status.
        audit = cad.forms.audit(doc, form="bina", warnings=True)
        self.assertEqual((audit.checked, audit.failing, audit.warned), (1, 0, 1))
        self.assertEqual([r.uid for r in audit.records], [poly.uid])
        with self.assertRaises(cad.CommandFailed) as limit:
            cad.forms.audit(doc, form="bina", limit=0)
        self.assertEqual(limit.exception.code, "invalid_count")

    def test_many_records_are_written_at_once(self) -> None:
        """docs/adr/0403 §5: the same values to many records, each checked by itself, every record or none."""
        doc, layer = drawing()
        cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Bina", layer=layer, id="bina"))
        cad.forms.edit(doc, edit=cad.PublishFormsOp(id="bina"))
        square = lambda x: [cad.Vec2(x, 0.0), cad.Vec2(x + 10.0, 0.0), cad.Vec2(x + 10.0, 8.0)]  # noqa: E731
        uids = [cad.polygon.create(doc, pts=square(x), layer_id=layer).uid for x in (0.0, 20.0, 40.0)]
        done = cad.forms.fill(doc, form="bina", uids=uids, values={"Durum": "iyi", "Kat": "2"})
        self.assertEqual(done.changed, uids)
        self.assertEqual({doc.entity(u).entity.attrs.get("Kat") for u in uids}, {"2"})
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.forms.fill(doc, form="bina", uids=uids, values={"Kat": "0"})
        self.assertEqual(refused.exception.code, "form_invalid")
        doc.undo()
        self.assertEqual({doc.entity(u).entity.attrs.get("Kat") for u in uids}, {None})

    def test_a_list_from_another_layer_chains_and_writes_codes(self) -> None:
        """docs/adr/0404: provinces on their own layer; a label is written as its code, a value the list does not offer
        is refused."""
        doc, layer = drawing()
        cad.layers.edit(doc, operation="add", name="İller")
        provinces = next(n.id for n in doc.layers() if n.name == "İller")
        cad.layers.edit(
            doc,
            operation="fields",
            layer=provinces,
            fields=[cad.LayerField(name="kod", kind="text"), cad.LayerField(name="ad", kind="text")],
        )
        for code, name, x in (("06", "Ankara", 0.0), ("34", "İstanbul", 5.0)):
            cad.point.create(doc, p=cad.Vec2(x, 50.0), layer_id=provinces, attrs={"kod": code, "ad": name})
        cad.layers.edit(doc, operation="fields", layer=layer, fields=[cad.LayerField(name="İl", kind="text")])
        body = cad.FormBody(
            revision=1,
            items=[
                cad.FieldFormItem(
                    id="il", field="İl", lookup=cad.FormLookup(layer=provinces, key="kod", label="ad")
                )
            ],
        )
        cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Adres", layer=layer, id="adres", body=body))
        poly = cad.polygon.create(doc, pts=[cad.Vec2(0.0, 0.0), cad.Vec2(10.0, 0.0), cad.Vec2(10.0, 8.0)], layer_id=layer)
        asked = cad.forms.validate(doc, form="adres", draft=True, uid=poly.uid)
        options = next(s.options for s in asked.items if s.id == "il")
        self.assertEqual([(o.code, o.label) for o in options or []], [("06", "Ankara"), ("34", "İstanbul")])
        written = cad.forms.submit(doc, form="adres", draft=True, uid=poly.uid, values={"İl": "Ankara"})
        self.assertEqual(written.attrs, {"İl": "06"})
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.forms.submit(doc, form="adres", draft=True, uid=poly.uid, values={"İl": "35"})
        self.assertEqual(refused.exception.code, "form_invalid")

    def test_a_relation_keeps_its_children(self) -> None:
        """docs/adr/0405: a meter refers to its building; the building deleted, its meters go with it (cascade); a
        restricting relation keeps a building with meters."""
        doc, layer = drawing()
        cad.layers.edit(doc, operation="add", name="Sayaç")
        meters = next(n.id for n in doc.layers() if n.name == "Sayaç")
        cad.layers.edit(doc, operation="fields", layer=meters, fields=[cad.LayerField(name="bina", kind="text")])
        made = cad.relations.edit(
            doc,
            edit=cad.CreateRelationsOp(
                relation=cad.ProjectRelation(
                    id="bina-sayac", name="Sayaçlar", parent=layer, child=meters, field="bina", on_delete="cascade"
                )
            ),
        )
        self.assertTrue(made.changed)
        building = cad.polygon.create(
            doc, pts=[cad.Vec2(0.0, 0.0), cad.Vec2(10.0, 0.0), cad.Vec2(10.0, 8.0)], layer_id=layer
        ).uid
        meter = cad.point.create(doc, p=cad.Vec2(2.0, 2.0), layer_id=meters, attrs={"bina": building}).uid
        cad.entities.delete(doc, uids=[building])
        self.assertEqual([r.uid for r in doc.entities(layer=meters)], [])
        doc.undo()
        self.assertEqual([r.uid for r in doc.entities(layer=meters)], [meter])
        cad.relations.edit(
            doc,
            edit=cad.UpdateRelationsOp(
                relation=cad.ProjectRelation(
                    id="bina-sayac", name="Sayaçlar", parent=layer, child=meters, field="bina", on_delete="restrict"
                )
            ),
        )
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.entities.delete(doc, uids=[building])
        self.assertEqual(refused.exception.code, "relation_restricted")


if __name__ == "__main__":
    unittest.main()
