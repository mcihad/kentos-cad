"""The forms' acceptance from Python (docs/adr/0443 §7, FORM-QA-16): the office designs and publishes, the field
fills, the reviewer sends back with notes, an agent finds the schema a page at a time. Every expectation here is
written by hand from the ADRs, not from what the code returned: the values a step leaves on the record, the notes'
canonical JSON, the counts discovery gives.
"""

import json
import unittest

import kentos.cad as cad

SQUARE = [cad.Vec2(0.0, 0.0), cad.Vec2(12.0, 0.0), cad.Vec2(12.0, 9.0), cad.Vec2(0.0, 9.0)]


def project() -> tuple[cad.Document, str]:
    doc = cad.Document.new("Kabul", srid=5256)
    cad.layers.edit(doc, operation="add", name="Bina", activate=True)
    layer = next(n.id for n in doc.layers() if n.name == "Bina")
    stages = [cad.FieldChoice(code=c, label=label) for c, label in
              (("taslak", "Taslak"), ("inceleme", "İncelemede"), ("onay", "Onaylı"))]
    cad.layers.edit(doc, operation="fields", layer=layer, fields=[
        cad.LayerField(name="Aşama", kind="text", values=stages),
        cad.LayerField(name="Kat", kind="integer", min="1", max="40"),
        cad.LayerField(name="Kimlik No", kind="text", length=11),
        cad.LayerField(name="Notlar", kind="text", length=500),
    ])
    body = cad.FormBody(
        revision=1,
        items=[
            cad.FieldFormItem(id="asama", field="Aşama"),
            cad.FieldFormItem(id="kat", field="Kat", required=True),
            cad.FieldFormItem(id="kimlik", field="Kimlik No", redact=True),
        ],
        workflow=cad.FormWorkflow(
            field="Aşama", initial="taslak", notes="Notlar",
            steps=[cad.WorkflowStep(label="Gönder", from_=["taslak"], to="inceleme", valid=True),
                   cad.WorkflowStep(label="Geri gönder", from_=["inceleme"], to="taslak"),
                   cad.WorkflowStep(label="Onayla", from_=["inceleme"], to="onay", valid=True)],
            locked=["onay"],
        ),
        translations={"en": cad.FormTranslation(name="Building", items={
            "kat": cad.ItemWords(label="Storeys"), "asama": cad.ItemWords(label="Stage", choices={"onay": "Approved"})})},
    )
    cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Bina kaydı", layer=layer, id="bina", body=body))
    cad.forms.edit(doc, edit=cad.PublishFormsOp(id="bina", note="İlk yayın"))
    return doc, layer


def leaves(nodes) -> list:
    """The layers (not groups) of the tree, in its order."""
    out = []
    for n in nodes:
        if n.type == "layer":
            out.append(n)
        out.extend(leaves(n.children or []))
    return out


class FormsAcceptance(unittest.TestCase):
    def test_the_office_the_field_and_the_reviewer(self) -> None:
        doc, layer = project()
        # The field: a building drawn and written through the form; its stage starts as the workflow says.
        uid = cad.polygon.create(doc, pts=SQUARE, layer_id=layer, attrs={"Aşama": "taslak"}).uid
        cad.forms.submit(doc, form="bina", uid=uid, values={"Kat": "3", "Kimlik No": "12345678901"})
        # The state field is the workflow's: read-only in the form, kept as drawn.
        self.assertEqual(doc.entity(uid).entity.attrs.get("Aşama"), "taslak")
        cad.forms.transition(doc, form="bina", uid=uid, step="Gönder")
        self.assertEqual(doc.entity(uid).entity.attrs.get("Aşama"), "inceleme")
        # The reviewer sends it back with a note on the storeys: the notes field holds them as canonical JSON.
        cad.forms.transition(doc, form="bina", uid=uid, step="Geri gönder", notes={"Kat": "Kat sayısı yanlış; çatı katı yok."})
        attrs = doc.entity(uid).entity.attrs
        self.assertEqual(attrs.get("Aşama"), "taslak")
        self.assertEqual(json.loads(attrs["Notlar"]), {"Kat": "Kat sayısı yanlış; çatı katı yok."})
        self.assertEqual(attrs["Notlar"], '{"Kat":"Kat sayısı yanlış; çatı katı yok."}')
        # Corrected and sent again, then approved: the next step without notes empties them.
        cad.forms.submit(doc, form="bina", uid=uid, values={"Kat": "2"})
        cad.forms.transition(doc, form="bina", uid=uid, step="Gönder")
        cad.forms.transition(doc, form="bina", uid=uid, step="Onayla")
        attrs = doc.entity(uid).entity.attrs
        self.assertEqual((attrs.get("Aşama"), attrs.get("Kat"), attrs.get("Notlar")), ("onay", "2", None))
        # An approved record is locked: the form refuses to write it.
        with self.assertRaises(cad.CommandFailed):
            cad.forms.submit(doc, form="bina", uid=uid, values={"Kat": "5"})

    def test_an_agent_finds_the_schema_a_page_at_a_time(self) -> None:
        doc, layer = project()
        everything = cad.schema.describe(doc)
        # The Bina layer, its form; no value lists, no relations (written by hand from the project).
        kinds = [(i.kind, i.id) for i in everything.items]
        self.assertIn(("forms", "bina"), kinds)
        self.assertIn(("layers", layer), kinds)
        forms = cad.schema.describe(doc, kinds=["forms"])
        self.assertEqual(forms.total, 1)
        form = forms.items[0]
        self.assertEqual((form.name, form.layer, form.count, form.version), ("Bina kaydı", layer, 3, 1))
        self.assertIs(form.detail, cad.UNSET, "a summary unless expanded")
        page = cad.schema.describe(doc, kinds=["layers"], limit=1)
        self.assertEqual(len(page.items), 1)
        self.assertEqual(page.total, len(leaves(doc.layers())))
        expanded = cad.schema.describe(doc, kinds=["layers"], layer=layer, expand=True)
        self.assertEqual([f["name"] for f in expanded.items[0].detail], ["Aşama", "Kat", "Kimlik No", "Notlar"])

    def test_a_link_table_carries_its_own_rows(self) -> None:
        """N:M with a link table (docs/adr/0445 §2): parcels and owners joined by share records without
        geometry, each link row its own id and attributes; a parcel deleted takes its shares, its owners stay."""
        doc = cad.Document.new("Hisseler", srid=5256)
        for name in ("Parsel", "Malik", "Hisse"):
            cad.layers.edit(doc, operation="add", name=name)
        lid = {n.name: n.id for n in doc.layers()}
        cad.layers.edit(doc, operation="fields", layer=lid["Parsel"], fields=[cad.LayerField(name="No", kind="text")])
        cad.layers.edit(doc, operation="fields", layer=lid["Malik"], fields=[cad.LayerField(name="Ad", kind="text")])
        cad.layers.edit(doc, operation="fields", layer=lid["Hisse"], fields=[
            cad.LayerField(name="Parsel", kind="text"), cad.LayerField(name="Malik", kind="text"),
            cad.LayerField(name="Pay", kind="text")])
        cad.relations.edit(doc, edit=cad.CreateRelationsOp(relation=cad.ProjectRelation(
            id="parsel-hisse", name="Parselin hisseleri", parent=lid["Parsel"], child=lid["Hisse"], field="Parsel",
            on_delete="cascade")))
        cad.relations.edit(doc, edit=cad.CreateRelationsOp(relation=cad.ProjectRelation(
            id="malik-hisse", name="Malikin hisseleri", parent=lid["Malik"], child=lid["Hisse"], field="Malik",
            on_delete="restrict")))
        parcels = [cad.polygon.create(doc, pts=[cad.Vec2(x, 0.0), cad.Vec2(x + 10.0, 0.0), cad.Vec2(x + 10.0, 8.0)],
                                      layer_id=lid["Parsel"], attrs={"No": f"101/{k}"}).uid
                   for k, x in ((1, 0.0), (2, 20.0))]
        owners = cad.records.create(doc, layer_id=lid["Malik"], rows=[
            cad.RecordRow(attrs={"Ad": "Ayşe"}), cad.RecordRow(attrs={"Ad": "Mehmet"})]).uids
        # Ayşe holds 1/2 of the first parcel and all of the second; Mehmet 1/2 of the first.
        shares = cad.records.create(doc, layer_id=lid["Hisse"], rows=[
            cad.RecordRow(attrs={"Parsel": parcels[0], "Malik": owners[0], "Pay": "1/2"}),
            cad.RecordRow(attrs={"Parsel": parcels[0], "Malik": owners[1], "Pay": "1/2"}),
            cad.RecordRow(attrs={"Parsel": parcels[1], "Malik": owners[0], "Pay": "1/1"})]).uids
        self.assertEqual(len(set(shares)), 3, "each link row its own id")
        rows = {r.uid: doc.entity(r.uid).entity.attrs for r in doc.entities(layer=lid["Hisse"])}
        self.assertEqual(rows[shares[1]]["Pay"], "1/2")
        # An owner holding shares is not deleted (restrict); the first parcel takes its two shares with it.
        with self.assertRaises(cad.CommandFailed):
            cad.entities.delete(doc, uids=[owners[1]])
        cad.entities.delete(doc, uids=[parcels[0]])
        left = {r.uid for r in doc.entities(layer=lid["Hisse"])}
        self.assertEqual(left, {shares[2]})
        self.assertEqual(len(list(doc.entities(layer=lid["Malik"]))), 2, "the owners stay")

    def test_the_sdk_and_the_profile_agree_with_the_host(self) -> None:
        self.assertTrue(cad.compatibility().ok)
        self.assertEqual(cad.FORM_PROFILE, "kentos.forms/1")


if __name__ == "__main__":
    unittest.main()
