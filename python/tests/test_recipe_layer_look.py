"""A recipe layer's transparency, blending mode, type and Yazdır in another project (`road.project.produce`,
docs/adr/4440; plan/map-cad.md MAP-20), held by an independent reading (no KentOS code in the expectations).

Project A is scripts/fixtures/road_terrain_zeroline_cases.py's Yol project with its zero line's layer “Sıfır hattı”
composed 40 % transparent (opacity 0.6) in the multiply mode, its labels masking Drenaj, a line layer that is not
printed. Its zero lines are drawn there and İş grafiği's graph is saved as JSON with Python's own `json`, as Tarifi
kaydet… would. The recipe keeps the opacity and the mode as A's file holds them, never the mask (it names A's layers by
their ids), the type and Yazdır off. Produced from it:

- in B, the project without that layer, the layer is added with that opacity and mode (no mask), a line layer not
  printed; B's lines are A's, coordinate for coordinate, and they are polylines, which a line layer holds;
- in C, which has a layer of that name of its own (opaque, any kind, printed), the drawings go on it and it keeps its
  own look;
- from a copy of the recipe whose layer is a point layer (edited here with `json`), the produce is refused with
  `layer_geometry_mismatch` — the plan too — and B's tree is as it was: a point layer holds points only;
- from a copy whose composition keeps a mask, the recipe is refused (`invalid_project_recipe`).
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
COMPOSE = {"opacity": 0.6, "blend": "multiply", "mask": {"labels": True, "size": 1.0, "targets": ["drenaj"]}}
# What a line layer holds and a point layer holds (docs/adr/0622 §5), read here from the ADR, not from KentOS.
LINE_KINDS = {"line", "polyline", "arc", "spline", "nurbs", "circle", "ellipse"}
POINT_KINDS = {"point"}


def project_a() -> dict:
    s = zc.setup(work="road")
    layer = next(n for n in s["layers"] if n["id"] == "sifir")
    layer.update({"compose": copy.deepcopy(COMPOSE), "geometry": "line", "plot": False})
    return s


def without_layer() -> dict:
    s = zc.setup(work="road")
    s["layers"] = [n for n in s["layers"] if n["id"] != "sifir"]
    return s


def opened(setup: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def tree(doc: cad.Document) -> list[dict]:
    return [n.to_json() for n in doc.layers()]


def node_of(nodes: list[dict], id_: str) -> dict | None:
    for n in nodes:
        if n["id"] == id_:
            return n
        found = node_of(n.get("children", []), id_)
        if found is not None:
            return found
    return None


def lines_of(doc: cad.Document, uids: list[str]) -> list[list[tuple[float, float]]]:
    return [[(p["x"], p["y"]) for p in doc.entity(u).entity.to_json()["pts"]] for u in uids]


class RecipeLayerLook(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.folder = tempfile.TemporaryDirectory()
        a = opened(project_a())
        cls.drawn = [cad.road.terrains.zeroline(a, base=b, comparison=c, layer_id="sifir") for b, c in PAIRS]
        cls.a = a
        cls.path = cls.write("tarif.json", cad.road.project.graph(a).to_json())
        cls.recipe = json.loads(Path(cls.path).read_text("utf-8"))

    @classmethod
    def write(cls, name: str, graph: dict) -> str:
        path = Path(cls.folder.name) / name
        path.write_text(json.dumps(graph, ensure_ascii=False), "utf-8")
        return str(path)

    @classmethod
    def edited(cls, name: str, **layer) -> str:
        """A copy of the recipe with each drawing's layer definition changed so."""
        graph = copy.deepcopy(cls.recipe)
        for n in graph["nodes"]:
            if n["kind"] == "drawing":
                n["recipe"]["layer"].update(copy.deepcopy(layer))
        return cls.write(name, graph)

    @classmethod
    def tearDownClass(cls) -> None:
        cls.folder.cleanup()

    def test_the_recipe_keeps_the_opacity_the_mode_the_type_and_yazdir_not_the_mask(self) -> None:
        unmasked = {k: v for k, v in COMPOSE.items() if k != "mask"}
        layers = [n["recipe"]["layer"] for n in self.recipe["nodes"] if n["kind"] == "drawing"]
        self.assertEqual(len(layers), 2)
        for layer in layers:
            self.assertEqual(layer["compose"], unmasked)
            self.assertEqual((layer["geometry"], layer["plot"]), ("line", False))

    def test_a_project_without_the_layer_gets_its_look(self) -> None:
        b = opened(without_layer())
        out = cad.road.project.produce(b, path=self.path)
        [went] = out.layers
        self.assertEqual(str(went.how), "created")
        node = node_of(tree(b), went.to)
        self.assertEqual(node["compose"], {"opacity": 0.6, "blend": "multiply"})
        self.assertEqual((node["geometry"], node.get("plot")), ("line", False))
        for p, mine in zip(out.produced, self.drawn):
            self.assertEqual(lines_of(b, list(p.uids)), lines_of(self.a, list(mine.uids)), "B's lines are A's")
            for u in p.uids:
                self.assertIn(b.entity(u).entity.to_json()["kind"], LINE_KINDS, "a line layer holds them")

    def test_a_layer_of_the_name_here_keeps_its_own_look(self) -> None:
        c = opened(zc.setup(work="road"))
        before = node_of(tree(c), "sifir")
        out = cad.road.project.produce(c, path=self.path)
        self.assertEqual([(lf.to, str(lf.how)) for lf in out.layers], [("sifir", "name")])
        after = node_of(tree(c), "sifir")
        self.assertEqual({k: after.get(k) for k in ("compose", "geometry", "plot")},
                         {k: before.get(k) for k in ("compose", "geometry", "plot")})
        self.assertIsNone(after.get("compose"))

    def test_a_point_layer_does_not_take_the_lines_the_plan_neither(self) -> None:
        kinds = {self.a.entity(u).entity.to_json()["kind"] for d in self.drawn for u in d.uids}
        self.assertFalse(kinds & POINT_KINDS, "the zero lines are no points")
        path = self.edited("nokta.json", geometry="point")
        b = opened(without_layer())
        before = tree(b)
        produce = cad.command("road.project.produce")
        planned = produce.run(b, produce.input_type.from_json({"path": path}), op="plan")
        self.assertEqual(planned.error.code, "layer_geometry_mismatch")
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.road.project.produce(b, path=path)
        self.assertEqual(caught.exception.code, "layer_geometry_mismatch")
        self.assertEqual(tree(b), before)

    def test_a_mask_in_the_recipe_is_refused(self) -> None:
        path = self.edited("maskeli.json", compose=copy.deepcopy(COMPOSE))
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.road.project.produce(opened(without_layer()), path=path)
        self.assertEqual(caught.exception.code, "invalid_project_recipe")


if __name__ == "__main__":
    unittest.main()
