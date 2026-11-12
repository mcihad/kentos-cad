"""A recipe layer's fields, the value lists they link and its subtypes in another project (`road.project.graph` and
`road.project.produce`, docs/adr/4860; plan/map-cad.md MAP-20), held by an independent reading (no KentOS code in the
expectations).

Project A is scripts/fixtures/road_terrain_zeroline_cases.py's Yol project with its zero line's layer “Sıfır hattı”
holding four fields: a name, a kind linked to A's value list “Sınır türü” (Kazı, Dolgu), a depth in metres and a day
its bound table makes itself (a column's `now()`); its subtypes sort the records by the kind, a cut's depth required.
Its zero lines are drawn there and İş grafiği's graph is saved as JSON with Python's own `json`, as Tarifi kaydet…
would. The recipe keeps the fields but the column (the binding is A's), the list as A defines it and the subtypes.
Produced from it:

- in B, without that layer and without lists, the layer is added with those fields and subtypes, and A's list with
  its id; the kind links it;
- in C, without the layer, whose own “Sınır türü” holds a third value, C's list stays as it is and A's is added as
  “Sınır türü_1”, the kind linked to that one — never merged;
- in D, whose layer of that name holds only a name, the drawings go on it, it keeps its own fields and the warning
  `recipe_layer_fields` names the recipe's fields it lacks;
- from a copy of the recipe whose field keeps a column (edited here with `json`), the recipe is refused.
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
          {"name": "Kayıt günü", "kind": "date", "column": {"made": "now()"}}]
SUBTYPES = {"field": "Tür", "types": [{"code": "K", "label": "Kazı sınırı",
                                       "fields": [{"field": "Derinlik", "required": True}]},
                                      {"code": "D", "label": "Dolgu sınırı"}]}


def project_a() -> dict:
    s = zc.setup(work="road")
    layer = next(n for n in s["layers"] if n["id"] == "sifir")
    layer.update({"fields": copy.deepcopy(FIELDS), "subtypes": copy.deepcopy(SUBTYPES)})
    s["settings"]["domains"] = [copy.deepcopy(LIST)]
    return s


def without_layer(lists=()) -> dict:
    s = zc.setup(work="road")
    s["layers"] = [n for n in s["layers"] if n["id"] != "sifir"]
    if lists:
        s["settings"]["domains"] = copy.deepcopy(list(lists))
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


def lists_of(doc: cad.Document) -> list[dict]:
    return [d.to_json() for d in (doc.settings.domains or [])]


def kept_fields() -> list[dict]:
    """A's fields as a recipe keeps them, read here: the column taken away."""
    return [{k: v for k, v in f.items() if k != "column"} for f in FIELDS]


class RecipeLayerFields(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.folder = tempfile.TemporaryDirectory()
        a = opened(project_a())
        for b, c in PAIRS:
            cad.road.terrains.zeroline(a, base=b, comparison=c, layer_id="sifir")
        cls.path = cls.write("tarif.json", cad.road.project.graph(a).to_json())
        cls.recipe = json.loads(Path(cls.path).read_text("utf-8"))

    @classmethod
    def write(cls, name: str, graph: dict) -> str:
        path = Path(cls.folder.name) / name
        path.write_text(json.dumps(graph, ensure_ascii=False), "utf-8")
        return str(path)

    @classmethod
    def tearDownClass(cls) -> None:
        cls.folder.cleanup()

    def test_the_recipe_keeps_the_fields_the_list_and_the_subtypes_not_the_column(self) -> None:
        layers = [n["recipe"]["layer"] for n in self.recipe["nodes"] if n["kind"] == "drawing"]
        self.assertEqual(len(layers), 2)
        for layer in layers:
            self.assertEqual(layer["fields"], kept_fields())
            self.assertEqual(layer["domains"], [LIST])
            self.assertEqual(layer["subtypes"], SUBTYPES)

    def test_a_project_without_lists_gets_the_layer_s_fields_and_the_list(self) -> None:
        b = opened(without_layer())
        out = cad.road.project.produce(b, path=self.path)
        [went] = out.layers
        self.assertEqual((str(went.how), list(went.domains_added)), ("created", ["Sınır türü"]))
        node = node_of(b, went.to)
        self.assertEqual(node["fields"], kept_fields())
        self.assertEqual(node["subtypes"], SUBTYPES)
        self.assertEqual(lists_of(b), [LIST])

    def test_a_list_of_the_name_with_other_values_stays_and_the_recipe_s_is_added_beside_it(self) -> None:
        own = dict(LIST, values=VALUES + [{"code": "Y", "label": "Yarma"}])
        c = opened(without_layer([own]))
        out = cad.road.project.produce(c, path=self.path)
        [went] = out.layers
        self.assertEqual(list(went.domains_added), ["Sınır türü_1"])
        lists = lists_of(c)
        self.assertEqual(lists[0], own, "C's own list is not touched")
        added = lists[1]
        self.assertEqual((added["name"], added["values"]), ("Sınır türü_1", VALUES))
        node = node_of(c, went.to)
        kind = next(f for f in node["fields"] if f["name"] == "Tür")
        self.assertEqual(kind["domain"], added["id"])
        self.assertEqual(kind["values"], VALUES)

    def test_a_layer_of_the_name_keeps_its_own_fields_and_the_lack_is_said(self) -> None:
        s = zc.setup(work="road")
        next(n for n in s["layers"] if n["id"] == "sifir")["fields"] = [{"name": "Ad", "kind": "text"}]
        d = opened(s)
        produce = cad.command("road.project.produce")
        r = produce.run(d, produce.input_type.from_json({"path": self.path}))
        self.assertIsNone(r.error)
        said = [w for w in r.warnings if w.code == "recipe_layer_fields"]
        self.assertEqual(len(said), 1)
        for name in ("Tür", "Derinlik", "Kayıt günü"):
            self.assertIn(f"“{name}”", said[0].message)
        self.assertEqual(node_of(d, "sifir")["fields"], [{"name": "Ad", "kind": "text"}])
        self.assertEqual(lists_of(d), [])

    def test_a_field_keeping_a_column_is_refused(self) -> None:
        graph = copy.deepcopy(self.recipe)
        for n in graph["nodes"]:
            if n["kind"] == "drawing":
                n["recipe"]["layer"]["fields"][0]["column"] = {"made": "now()"}
        path = self.write("sutunlu.json", graph)
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.road.project.produce(opened(without_layer()), path=path)
        self.assertEqual(caught.exception.code, "invalid_project_recipe")


if __name__ == "__main__":
    unittest.main()
