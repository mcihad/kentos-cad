"""A building and its owners: records without a geometry and a one-to-many relation (docs/adr/0405, 0409, 0410, 0415).

Kapsam: masaüstü ve başsız çizim. İlişki (üst ve alt katman, silmede izleme), geometrisiz kayıtlar, formdaki ilişkili
kayıtlar öğesi ve alt kayıtlarıyla çoğaltma.

    python examples/forms/owners.py
"""
from __future__ import annotations

import kentos.cad as cad


def main() -> dict:
    doc = cad.Document.new("Mülkiyet", srid=5256)
    for name in ("Bina", "Malik"):
        cad.layers.edit(doc, operation="add", name=name)
    buildings = next(n.id for n in doc.layers() if n.name == "Bina")
    owners = next(n.id for n in doc.layers() if n.name == "Malik")
    cad.layers.edit(doc, operation="fields", layer=buildings, fields=[cad.LayerField(name="Kod", kind="text")])
    cad.layers.edit(doc, operation="fields", layer=owners, fields=[
        cad.LayerField(name="Ad", kind="text", required=True),
        cad.LayerField(name="Pay", kind="decimal", scale=2),
        cad.LayerField(name="Bina", kind="text"),
    ])
    cad.relations.edit(doc, edit=cad.CreateRelationsOp(relation=cad.ProjectRelation(
        id="bina-malik", name="Malikler", parent=buildings, child=owners, field="Bina", on_delete="cascade")))

    building = cad.polygon.create(doc, pts=[cad.Vec2(0.0, 0.0), cad.Vec2(10.0, 0.0), cad.Vec2(10.0, 8.0)],
                                  layer_id=buildings, attrs={"Kod": "B-1"}).uid
    made = cad.records.create(doc, layer_id=owners, rows=[
        cad.RecordRow(attrs={"Ad": "Ayşe Demir", "Pay": "0.5", "Bina": building}, label="Ayşe Demir"),
        cad.RecordRow(attrs={"Ad": "Mehmet Kaya", "Pay": "0.5", "Bina": building}, label="Mehmet Kaya"),
    ])

    # The building's form lists its owners.
    body = cad.FormBody(revision=1, items=[
        cad.FieldFormItem(id="kod", field="Kod"),
        cad.RelatedFormItem(id="malikler", layer=owners, field="Bina", label="Malikler"),
    ])
    cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Bina", layer=buildings, id="bina", body=body))
    asked = cad.forms.validate(doc, form="bina", draft=True, uid=building)
    related = next(s.related for s in asked.items if s.id == "malikler")

    # The building copied with its owners, then deleted: its own owners go with it, the copy's stay.
    copied = cad.records.duplicate(doc, uids=[building])
    cad.entities.delete(doc, uids=[building])
    left = [r.uid for r in doc.entities(layer=owners)]
    return {
        "owners": made.uids,
        "related": related,
        "copies": len(copied.copies),
        "left_after_delete": len(left),
    }


if __name__ == "__main__":
    for key, value in main().items():
        print(f"{key}: {value}")
