"""An extension's layer schema (docs/adr/8057 §23): ``ctx.apply_declarative_schema("schema.json")`` sets up the
package's group, value lists, layers with their fields and relations on the drawing, each layer by its schema name as
its id; what is there is kept and only what is missing is added, so a second run changes nothing. A path outside the
package and a run without a drawing are refused."""

from __future__ import annotations

import copy
import json
import os
import tempfile
import threading
import unittest

import kentos.cad as cad
from kentos import ext

SCHEMA = {
    "format": "kentos.layerSchema",
    "version": 1,
    "group": {"id": "mezarlik", "name": "Mezarlık"},
    "domains": [
        {"id": "mezar_durumu", "name": "Mezar durumu", "kind": "text",
         "values": [{"code": "BOS", "label": "Boş"}, {"code": "DOLU", "label": "Dolu"}]},
    ],
    "layers": [
        {"name": "mezar_yeri", "title": "Mezar yeri", "geometry": "area", "form": "forms/mezar_yeri.kform",
         "fields": [{"name": "mezar_no", "kind": "text", "required": True, "alias": "Mezar no"},
                    {"name": "durum", "kind": "text", "domain": "mezar_durumu"}]},
        {"name": "defin_kaydi", "title": "Defin kaydı",
         "fields": [{"name": "mezar_id", "kind": "text"}, {"name": "defin_tarihi", "kind": "date"}]},
    ],
    "relations": [
        {"id": "defin-mezar", "name": "Defin kaydının mezarı", "parent": "mezar_yeri", "child": "defin_kaydi",
         "field": "mezar_id"},
    ],
}


def context(doc: object, source: str | None) -> ext.CommandContext:
    return ext.CommandContext(ext._Link(lambda _m: None, 1, threading.Event()), doc, source=source)


FORM = {
    "format": "kentos.form-package",
    "version": 1,
    "name": "Mezar yeri",
    "body": {"revision": 1, "items": [{"type": "field", "id": "mezar-no", "field": "mezar_no", "required": True}]},
    "fields": [{"name": "mezar_no", "kind": "text"}],
}


def package(schema: dict) -> tempfile.TemporaryDirectory:
    folder = tempfile.TemporaryDirectory()
    with open(os.path.join(folder.name, "schema.json"), "w", encoding="utf-8") as f:
        json.dump(schema, f, ensure_ascii=False)
    os.makedirs(os.path.join(folder.name, "forms"))
    with open(os.path.join(folder.name, "forms", "mezar_yeri.kform"), "w", encoding="utf-8") as f:
        json.dump(FORM, f, ensure_ascii=False)
    return folder


class LayerSchemaTests(unittest.TestCase):
    def test_installs_once_and_adds_only_what_is_missing(self) -> None:
        doc = cad.Document.new("Mezarlık", srid=5256)
        with package(SCHEMA) as folder:
            said = context(doc, folder).apply_declarative_schema("schema.json")
        self.assertEqual(said.message, "Şema kuruldu: 1 grup, 1 değer listesi, 2 katman, 1 form, 1 ilişki.")
        forms = cad.schema.describe(doc, kinds=["forms"]).items
        self.assertEqual([(f.id, f.name, f.layer) for f in forms], [("mezar_yeri", "Mezar yeri", "mezar_yeri")])
        yeri = doc.layer("mezar_yeri")
        self.assertEqual(yeri.name, "Mezar yeri")
        self.assertEqual([f.name for f in yeri.fields], ["mezar_no", "durum"])
        self.assertEqual(yeri.fields[1].domain, "mezar_durumu")
        self.assertTrue(yeri.fields[0].required)
        group = doc.layer("mezarlik")
        self.assertEqual([c.id for c in group.children], ["mezar_yeri", "defin_kaydi"])
        relations = cad.schema.describe(doc, kinds=["relations"]).items
        self.assertEqual([(r.id, r.layer, r.child) for r in relations], [("defin-mezar", "mezar_yeri", "defin_kaydi")])
        # Again: nothing changes.
        revision = doc.info().revision
        with package(SCHEMA) as folder:
            again = context(doc, folder).apply_declarative_schema()
        self.assertEqual(again.message, "Şema zaten kurulu: değişiklik yok.")
        self.assertEqual(doc.info().revision, revision)
        # A later version's field: only it is added, the user's own field kept.
        cad.layers.edit(doc, operation="fields", layer="defin_kaydi",
                        fields=[*doc.layer("defin_kaydi").fields, cad.LayerField(name="not", kind="text")])
        later = copy.deepcopy(SCHEMA)
        later["layers"][1]["fields"].append({"name": "merhum_adi", "kind": "text"})
        with package(later) as folder:
            grown = context(doc, folder).apply_declarative_schema()
        self.assertEqual(grown.message, "Şema kuruldu: 1 katman.")
        defin = next(r for r in grown.rows if r["Tür"] == "Katman" and r["Kimlik"] == "defin_kaydi")
        self.assertEqual(defin["Sonuç"], "vardı; 1 alan eklendi")
        self.assertEqual([r["Sonuç"] for r in grown.rows if r["Tür"] == "Form"], ["vardı"], "the form is kept")
        self.assertEqual([f.name for f in doc.layer("defin_kaydi").fields],
                         ["mezar_id", "defin_tarihi", "not", "merhum_adi"])

    def test_a_layers_subtypes_and_value_groups_are_set_up_after_its_fields(self) -> None:
        # docs/adr/8057 §23: KentOS's own subtypes (docs/adr/0422) and value groups (0423), in the schema's words.
        schema = copy.deepcopy(SCHEMA)
        schema["domains"] += [
            {"id": "mezar_turu", "name": "Mezar türü", "kind": "text",
             "values": [{"code": "TEKLI", "label": "Tekli"}, {"code": "AILE", "label": "Aile"}]},
            {"id": "mezar_boyutu", "name": "Boyut", "kind": "text",
             "values": [{"code": "1.1x2.2", "label": "1,10 × 2,20"}, {"code": "2.2x2.2", "label": "2,20 × 2,20"}]},
        ]
        yeri = schema["layers"][0]
        yeri["fields"] += [{"name": "tur", "kind": "text", "domain": "mezar_turu"},
                           {"name": "boyut", "kind": "text", "domain": "mezar_boyutu"}]
        yeri["subtypes"] = {"field": "tur", "types": [
            {"code": "TEKLI", "label": "Tekli mezar",
             "fields": [{"field": "boyut", "values": ["1.1x2.2"], "default": "1.1x2.2"}]},
            {"code": "AILE", "label": "Aile mezarı",
             "fields": [{"field": "boyut", "values": ["2.2x2.2"], "default": "2.2x2.2", "required": True}]},
        ]}
        yeri["contingents"] = [{"name": "Durum ve boyut", "fields": ["durum", "boyut"], "restrictive": True, "rows": [
            {"cells": [{"kind": "code", "code": "BOS"}, {"kind": "any"}]},
            {"cells": [{"kind": "code", "code": "DOLU"}, {"kind": "code", "code": "1.1x2.2"}], "subtype": "TEKLI"},
        ]}]
        doc = cad.Document.new("Mezarlık", srid=5256)
        with package(schema) as folder:
            said = context(doc, folder).apply_declarative_schema()
        self.assertEqual(said.message, "Şema kuruldu: 1 grup, 3 değer listesi, 2 katman, 1 form, 1 katmana alt tür, "
                                       "1 katmana değer bileşimi, 1 ilişki.")
        layer = doc.layer("mezar_yeri")
        self.assertEqual((layer.subtypes.field, [t.code for t in layer.subtypes.types]), ("tur", ["TEKLI", "AILE"]))
        self.assertEqual(layer.subtypes.types[1].fields[0].required, True)
        self.assertEqual([(g.name, g.restrictive, len(g.rows)) for g in layer.contingents],
                         [("Durum ve boyut", True, 2)])
        self.assertEqual([r["Ad"] for r in said.rows if r["Tür"] in ("Alt türler", "Değer bileşimleri")],
                         ["Mezar yeri: 2 alt tür", "Mezar yeri: 1 grup"])
        # Again: nothing changes; the layer keeps them.
        revision = doc.info().revision
        with package(schema) as folder:
            again = context(doc, folder).apply_declarative_schema()
        self.assertEqual(again.message, "Şema zaten kurulu: değişiklik yok.")
        self.assertEqual(doc.info().revision, revision)

    def test_a_path_outside_the_package_a_missing_file_and_no_drawing_are_refused(self) -> None:
        doc = cad.Document.new("Mezarlık", srid=5256)
        with package(SCHEMA) as folder:
            ctx = context(doc, folder)
            for path in ("../schema.json", "/etc/passwd", "yok.json"):
                with self.assertRaises(ValueError):
                    ctx.apply_declarative_schema(path)
            with self.assertRaises(ext.NotAvailable):
                context(None, folder).apply_declarative_schema()
        newer = dict(SCHEMA, version=2)
        with package(newer) as folder, self.assertRaises(ext.NotAvailable):
            context(doc, folder).apply_declarative_schema()


if __name__ == "__main__":
    unittest.main()
