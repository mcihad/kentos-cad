"""Kinds of buildings and the values that go together (docs/adr/0422, 0423).

Kapsam: masaüstü ve başsız çizim. Alt türler (her türün sunduğu değerler, varsayılanı, zorunluluğu) ve değer bileşimleri
(kısıtlayıcı grup); yazmalar ve form aynı kuralı uygular.

    python examples/forms/kinds.py
"""
from __future__ import annotations

import kentos.cad as cad

SQUARE = [cad.Vec2(0.0, 0.0), cad.Vec2(10.0, 0.0), cad.Vec2(10.0, 10.0), cad.Vec2(0.0, 10.0)]


def main() -> dict:
    doc = cad.Document.new("Yapı envanteri", srid=5256)
    cad.layers.edit(doc, operation="add", name="Bina", activate=True)
    layer = next(n.id for n in doc.layers() if n.name == "Bina")
    choice = cad.FieldChoice
    cad.layers.edit(
        doc,
        operation="fields",
        layer=layer,
        fields=[
            cad.LayerField(name="Tür", kind="text", values=[choice(code="konut", label="Konut"),
                                                            choice(code="sanayi", label="Sanayi")]),
            cad.LayerField(name="Malzeme", kind="text", default="ahsap", values=[
                choice(code="ahsap", label="Ahşap"), choice(code="beton", label="Betonarme"),
                choice(code="celik", label="Çelik")]),
            cad.LayerField(name="Çatı", kind="text", values=[choice(code="duz", label="Düz"),
                                                             choice(code="kirma", label="Kırma")]),
        ],
    )
    # An industrial building is concrete or steel, steel by default.
    cad.layers.subtypes(doc, layer=layer, subtypes=cad.LayerSubtypes(field="Tür", types=[
        cad.Subtype(code="konut"),
        cad.Subtype(code="sanayi", fields=[cad.SubtypeField(field="Malzeme", values=["beton", "celik"], default="celik")]),
    ]))
    # Steel goes with a flat roof only.
    code = lambda c: cad.CodeCell(code=c)  # noqa: E731
    cad.layers.contingents(doc, layer=layer, groups=[cad.FieldGroup(
        name="Malzeme ve çatı", fields=["Malzeme", "Çatı"], restrictive=True, rows=[
            cad.Combination(cells=[code("ahsap"), cad.AnyCell()]),
            cad.Combination(cells=[code("beton"), cad.AnyCell()]),
            cad.Combination(cells=[code("celik"), code("duz")]),
        ])])

    factory = cad.polygon.create(doc, pts=SQUARE, layer_id=layer, attrs={"Tür": "sanayi", "Çatı": "duz"}).uid
    refusals = []
    for attrs in ({"Malzeme": "ahsap"}, {"Çatı": "kirma"}):
        try:
            cad.entities.set(doc, uids=[factory], operation="attributes", attrs=attrs)
        except cad.CommandFailed as e:
            refusals.append(e.code)

    cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Yapı", layer=layer, id="yapi"))
    asked = cad.forms.validate(doc, form="yapi", draft=True, uid=factory)
    item_of = {i.field: i.id for i in doc.forms[0].draft.items}
    choices = {s.id: s.choices for s in asked.items}
    return {
        "factory": doc.entity(factory).entity.attrs,
        "refusals": refusals,
        "material_choices": choices[item_of["Malzeme"]],
        "roof_choices": choices[item_of["Çatı"]],
    }


if __name__ == "__main__":
    for key, value in main().items():
        print(f"{key}: {value}")
