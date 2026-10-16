"""An address chosen from other layers: province, then its districts (docs/adr/0404).

Kapsam: masaüstü ve başsız çizim. Başka katmandan değer listesi (anahtar ve etiket), zincir (ilçe ile seçilen il),
etiketin kod olarak yazılması ve listede olmayan değerin reddi.

    python examples/forms/address.py
"""
from __future__ import annotations

import kentos.cad as cad


def main() -> dict:
    doc = cad.Document.new("Adres", srid=5256)
    for name in ("Bina", "İl", "İlçe"):
        cad.layers.edit(doc, operation="add", name=name)
    buildings, provinces, districts = (next(n.id for n in doc.layers() if n.name == x) for x in ("Bina", "İl", "İlçe"))
    cad.layers.edit(doc, operation="fields", layer=provinces,
                    fields=[cad.LayerField(name="kod", kind="text"), cad.LayerField(name="ad", kind="text")])
    cad.layers.edit(doc, operation="fields", layer=districts, fields=[
        cad.LayerField(name="kod", kind="text"), cad.LayerField(name="ad", kind="text"), cad.LayerField(name="il", kind="text")])
    for code, name, x in (("06", "Ankara", 0.0), ("34", "İstanbul", 5.0)):
        cad.point.create(doc, p=cad.Vec2(x, 50.0), layer_id=provinces, attrs={"kod": code, "ad": name})
    for code, name, province, x in (("0601", "Çankaya", "06", 0.0), ("0602", "Keçiören", "06", 2.0),
                                    ("3401", "Kadıköy", "34", 5.0)):
        cad.point.create(doc, p=cad.Vec2(x, 60.0), layer_id=districts, attrs={"kod": code, "ad": name, "il": province})
    cad.layers.edit(doc, operation="fields", layer=buildings,
                    fields=[cad.LayerField(name="İl", kind="text"), cad.LayerField(name="İlçe", kind="text")])

    body = cad.FormBody(revision=1, items=[
        cad.FieldFormItem(id="il", field="İl", lookup=cad.FormLookup(layer=provinces, key="kod", label="ad")),
        cad.FieldFormItem(id="ilce", field="İlçe", lookup=cad.FormLookup(
            layer=districts, key="kod", label="ad", matches=[cad.LookupMatch(field="il", record="İl")])),
    ])
    cad.forms.edit(doc, edit=cad.CreateFormsOp(name="Adres", layer=buildings, id="adres", body=body))
    uid = cad.polygon.create(doc, pts=[cad.Vec2(0.0, 0.0), cad.Vec2(10.0, 0.0), cad.Vec2(10.0, 8.0)],
                             layer_id=buildings).uid
    asked = cad.forms.validate(doc, form="adres", draft=True, uid=uid, values={"İl": "Ankara"})
    districts_offered = [o.label for o in next(s.options for s in asked.items if s.id == "ilce") or []]
    written = cad.forms.submit(doc, form="adres", draft=True, uid=uid, values={"İl": "Ankara", "İlçe": "Çankaya"})
    try:
        cad.forms.submit(doc, form="adres", draft=True, uid=uid, values={"İlçe": "Kadıköy"})
        refused = None
    except cad.CommandFailed as e:
        refused = e.code
    return {"districts_offered": districts_offered, "written": written.attrs, "refused": refused}


if __name__ == "__main__":
    for key, value in main().items():
        print(f"{key}: {value}")
