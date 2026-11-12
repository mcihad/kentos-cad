"""A recipe layer's value combinations, record rules, indexes and forms in another project, and the fields a layer
found there lacks (`road.project.graph` and `road.project.produce`, docs/adr/5240; plan/map-cad.md MAP-20), held by an
independent reading (no KentOS code in the expectations).

Project A is scripts/fixtures/road_terrain_zeroline_cases.py's Yol project with its zero line's layer “Sıfır hattı”
holding a name, a kind linked to A's list “Sınır türü” (Kazı, Dolgu), a depth and a ground with its own values; a
value combination of the kind and the ground (a cut any ground, a fill on soil only), a record rule on the depth, a
unique index on the name and a form editing it, published and enforced in A. Its zero lines are drawn and İş grafiği's
graph is saved with Python's own `json`. Produced from it:

- the recipe keeps the combination, rule and index as A holds them and the form's draft only (its id, name and draft);
- in B, without that layer, the layer added holds them, and B's forms gain the draft as a new form editing it, not
  published, not enforced;
- in C, whose layer of that name holds only a name, `addFields` adds the recipe's other fields after it (their list
  too) and gives it no combination, rule, index or form; without `addFields` it keeps its own field alone.
"""

from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import road_terrain_zeroline_cases as zc  # noqa: E402

PAIRS = [("Düz", "Tümsek"), ("Teras", "Set")]
VALUES = [{"code": "K", "label": "Kazı"}, {"code": "D", "label": "Dolgu"}]
LIST = {"id": "sinir-turu", "name": "Sınır türü", "kind": "text", "values": VALUES}
FIELDS = [{"name": "Ad", "kind": "text", "length": 40},
          {"name": "Tür", "kind": "text", "values": VALUES, "domain": "sinir-turu"},
          {"name": "Derinlik", "kind": "decimal", "scale": 2, "unit": "m"},
          {"name": "Zemin", "kind": "text", "values": [{"code": "T", "label": "Toprak"}, {"code": "Y", "label": "Kaya"}]}]
GROUP = {"name": "Tür ve zemin", "fields": ["Tür", "Zemin"], "restrictive": True,
         "rows": [{"cells": [{"kind": "code", "code": "K"}, {"kind": "any"}]},
                  {"cells": [{"kind": "code", "code": "D"}, {"kind": "code", "code": "T"}]}]}
RULE = {"id": "derinlik-arti", "name": "Derinlik artı", "rule": "Derinlik boş veya Derinlik > 0",
        "message": "Derinlik sıfırdan büyük olmalı.", "fields": ["Derinlik"]}
INDEX = {"name": "Ad", "fields": ["Ad"], "unique": True}
DRAFT = {"revision": 1, "items": [{"type": "field", "id": "ad", "field": "Ad", "required": True},
                                  {"type": "field", "id": "tur", "field": "Tür"}]}


def project_a() -> dict:
    s = zc.setup(work="road")
    layer = next(n for n in s["layers"] if n["id"] == "sifir")
    layer.update({"fields": copy.deepcopy(FIELDS), "contingents": [copy.deepcopy(GROUP)], "rules": [copy.deepcopy(RULE)],
                  "indexes": [copy.deepcopy(INDEX)]})
    s["settings"]["domains"] = [copy.deepcopy(LIST)]
    s["settings"]["forms"] = [{"id": "sinir-formu", "name": "Sınır formu", "layer": "sifir", "draft": copy.deepcopy(DRAFT)}]
    return s


def opened(setup: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def node_of(doc: cad.Document, id_: str) -> dict | None:
    def walk(nodes):
        for n in nodes:
            if n["id"] == id_:
                return n
            found = walk(n.get("children", []))
            if found is not None:
                return found
        return None
    return walk([n.to_json() for n in doc.layers()])


def forms_of(doc: cad.Document) -> list[dict]:
    return [f.to_json() for f in (doc.settings.forms or [])]


class RecipeLayerRules(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.folder = tempfile.TemporaryDirectory()
        a = opened(project_a())
        for b, c in PAIRS:
            cad.road.terrains.zeroline(a, base=b, comparison=c, layer_id="sifir")
        path = Path(cls.folder.name) / "tarif.json"
        path.write_text(json.dumps(cad.road.project.graph(a).to_json(), ensure_ascii=False), "utf-8")
        cls.path = str(path)
        cls.recipe = json.loads(path.read_text("utf-8"))

    @classmethod
    def tearDownClass(cls) -> None:
        cls.folder.cleanup()

    def test_the_recipe_keeps_the_combination_rule_index_and_the_form_s_draft(self) -> None:
        for n in (n for n in self.recipe["nodes"] if n["kind"] == "drawing"):
            layer = n["recipe"]["layer"]
            self.assertEqual(layer["contingents"], [GROUP])
            self.assertEqual(layer["rules"], [RULE])
            self.assertEqual(layer["indexes"], [INDEX])
            self.assertEqual(layer["forms"], [{"id": "sinir-formu", "name": "Sınır formu", "draft": DRAFT}])

    def test_an_added_layer_takes_them_and_its_form_as_a_draft(self) -> None:
        s = zc.setup(work="road")
        s["layers"] = [n for n in s["layers"] if n["id"] != "sifir"]
        b = opened(s)
        out = cad.road.project.produce(b, path=self.path)
        [went] = out.layers
        self.assertEqual((str(went.how), list(went.forms_added)), ("created", ["Sınır formu"]))
        node = node_of(b, went.to)
        self.assertEqual((node["contingents"], node["rules"], node["indexes"]), ([GROUP], [RULE], [INDEX]))
        [form] = forms_of(b)
        self.assertEqual((form["id"], form["name"], form["layer"], form["draft"]), ("sinir-formu", "Sınır formu", went.to, DRAFT))
        self.assertNotIn("releases", form)
        self.assertNotIn("enforce", form)

    def test_add_fields_gives_a_found_layer_the_fields_it_lacks_and_nothing_else(self) -> None:
        def c() -> cad.Document:
            s = zc.setup(work="road")
            next(n for n in s["layers"] if n["id"] == "sifir")["fields"] = [{"name": "Ad", "kind": "text"}]
            return opened(s)
        kept = c()
        cad.road.project.produce(kept, path=self.path)
        self.assertEqual(node_of(kept, "sifir")["fields"], [{"name": "Ad", "kind": "text"}])
        grown = c()
        out = cad.road.project.produce(grown, path=self.path, add_fields=True)
        [went] = out.layers
        self.assertEqual(list(went.fields_added), ["Tür", "Derinlik", "Zemin"])
        node = node_of(grown, "sifir")
        self.assertEqual([f["name"] for f in node["fields"]], ["Ad", "Tür", "Derinlik", "Zemin"])
        self.assertEqual(node["fields"][0], {"name": "Ad", "kind": "text"}, "its own field stays")
        self.assertEqual([d.to_json() for d in grown.settings.domains], [LIST])
        for key in ("contingents", "rules", "indexes"):
            self.assertFalse(node.get(key), key)
        self.assertEqual(forms_of(grown), [])


if __name__ == "__main__":
    unittest.main()
