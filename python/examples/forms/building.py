"""A building survey form, end to end (docs/adr/0401–0421).

Kapsam: masaüstü ve başsız çizim (``kentos.cad``), bulutsuz. Katmanın alanları, alanlardan form, kurallar (görünme,
zorunluluk), hesap, yayın ve kuralları uygulama, formla yazma, kayıtları denetleme, senaryolar ve hesapları yenileme.

    python examples/forms/building.py
"""
from __future__ import annotations

import kentos.cad as cad

SQUARE = [cad.Vec2(0.0, 0.0), cad.Vec2(12.0, 0.0), cad.Vec2(12.0, 9.0), cad.Vec2(0.0, 9.0)]


def main() -> dict:
    doc = cad.Document.new("Hasar tespiti", srid=5256)
    cad.layers.edit(doc, operation="add", name="Bina", activate=True)
    layer = next(n.id for n in doc.layers() if n.name == "Bina")
    states = [cad.FieldChoice(code="iyi", label="İyi"), cad.FieldChoice(code="inceleme", label="Ek inceleme gerekli")]
    cad.layers.edit(
        doc,
        operation="fields",
        layer=layer,
        fields=[
            cad.LayerField(name="Durum", kind="text", values=states),
            cad.LayerField(name="Kat", kind="integer", min="1", max="40"),
            cad.LayerField(name="Yükseklik", kind="decimal", scale=1),
            cad.LayerField(name="Açıklama", kind="text"),
        ],
    )

    # The form, its rules and its calculation, saved over the draft the fields made.
    body = cad.FormBody(
        revision=1,
        items=[
            cad.FieldFormItem(id="durum", field="Durum", required=True),
            cad.FieldFormItem(id="kat", field="Kat"),
            cad.FieldFormItem(id="yukseklik", field="Yükseklik", calculate="Kat * 3"),
            cad.FieldFormItem(
                id="aciklama", field="Açıklama", visible_when="Durum = 'inceleme'", required_when="Durum = 'inceleme'"
            ),
        ],
    )
    cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Bina denetimi", layer=layer, id="bina", body=body))
    cad.forms.edit(doc, edit=cad.PublishFormsOp(id="bina", note="İlk yayın"))

    # A building drawn, then written through the form: the calculation fills its height.
    uid = cad.polygon.create(doc, pts=SQUARE, layer_id=layer).uid
    asked = cad.forms.validate(doc, form="bina", uid=uid, values={"Durum": "inceleme"})
    written = cad.forms.submit(doc, form="bina", uid=uid, values={"Durum": "İyi", "Kat": "4"})

    # A second building written around the form, then the layer judged by it.
    other = cad.polygon.create(doc, pts=[cad.Vec2(p.x + 20.0, p.y) for p in SQUARE], layer_id=layer).uid
    cad.entities.set(doc, uids=[other], operation="attributes", attrs={"Durum": "inceleme", "Kat": "2"})
    audited = cad.forms.audit(doc, form="bina")

    # The form changes: four metres a floor. Its recalculation over the layer is one undo step.
    # The draft's revision is the one the change was made against.
    four = cad.FormBody(revision=1, items=[*body.items[:2], cad.FieldFormItem(
        id="yukseklik", field="Yükseklik", calculate="Kat * 4"), body.items[3]])
    cad.forms.edit(doc, edit=cad.UpdateFormsOp(id="bina", body=four))
    cad.forms.edit(doc, edit=cad.PublishFormsOp(id="bina", note="Kat yüksekliği 4 m"))
    recomputed = cad.forms.recompute(doc, form="bina")

    return {
        "asked_valid": asked.valid,
        "asked_issues": [i.code for i in asked.issues],
        "written": written.attrs,
        "audit_failing": audited.failing,
        "recomputed": len(recomputed.changed),
        "height_now": doc.entity(uid).entity.attrs.get("Yükseklik"),
    }


if __name__ == "__main__":
    for key, value in main().items():
        print(f"{key}: {value}")
