"""A recipe's layers in another project (`road.project.produce`, docs/adr/3640; plan/map-cad.md MAP-20), held by an
independent reading (no KentOS code in the expectations).

Project A is scripts/fixtures/road_terrain_zeroline_cases.py's Yol project: its zero lines are drawn on its layer
“Sıfır hattı” (red, 0.5 mm) and İş grafiği's graph is saved as JSON with Python's own `json`, as Tarifi kaydet… would.
The recipe now keeps that layer's definition. Produced from it:

- in B, the same project without that layer, the drawings go on a layer added with the recipe's name, colour, line
  type and weight, and B's lines are A's, coordinate for coordinate; produced again, the layer is found by its name and
  nothing is written;
- in C, whose layer of that name is written “SIFIR HATTI” under another id, the drawings go on that layer (the name
  folded as Katmanları çevir folds it: I → ı, then lower case), C's kept drawings name it, and the graph says them
  current;
- in D, with two layers of that name, every drawing is left with the question; given in `layers`, they go on the one
  given.
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
from kentos.cad.errors import CommandWarning  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import road_terrain_zeroline_cases as zc  # noqa: E402

PAIRS = [("Düz", "Tümsek"), ("Teras", "Set")]


def fold(name: str) -> str:
    """Katmanları çevir's folding, by hand: I → ı, İ → i, then lower case."""
    return name.replace("I", "ı").replace("İ", "i").lower()


def setup_without(layer: str | None = None, rename: tuple[str, str, str] | None = None, twin: str | None = None) -> dict:
    """The zero lines' project: without `layer`, or with `rename` (old id → new id, new name), or with a `twin` of the
    zero line's layer under another id."""
    s = zc.setup(work="road")
    if layer is not None:
        s["layers"] = [n for n in s["layers"] if n["id"] != layer]
    if rename is not None:
        old, new, name = rename
        for n in s["layers"]:
            if n["id"] == old:
                n.update({"id": new, "name": name})
    if twin is not None:
        copy_ = copy.deepcopy(next(n for n in s["layers"] if n["id"] == "sifir"))
        copy_.update({"id": twin, "name": "SIFIR HATTI"})
        s["layers"].append(copy_)
    return s


def opened(setup: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def lines_of(doc: cad.Document, uids: list[str]) -> list[list[tuple[float, float]]]:
    return [[(p["x"], p["y"]) for p in doc.entity(u).entity.to_json()["pts"]] for u in uids]


def flat(nodes) -> list:
    out = []
    for n in nodes:
        out.append(n)
        out.extend(flat(n.children or []))
    return out


class RecipeLayers(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.folder = tempfile.TemporaryDirectory()
        a = opened(zc.setup(work="road"))
        cls.drawn = [cad.road.terrains.zeroline(a, base=b, comparison=c, layer_id="sifir") for b, c in PAIRS]
        cls.a = a
        path = Path(cls.folder.name) / "tarif.json"
        path.write_text(json.dumps(cad.road.project.graph(a).to_json(), ensure_ascii=False), "utf-8")
        cls.path = str(path)
        cls.recipe = json.loads(path.read_text("utf-8"))
        theirs = next(n for n in zc.setup(work="road")["layers"] if n["id"] == "sifir")
        cls.theirs = theirs

    @classmethod
    def tearDownClass(cls) -> None:
        cls.folder.cleanup()

    def test_the_recipe_keeps_the_layer_s_definition(self) -> None:
        layers = [n["recipe"]["layer"] for n in self.recipe["nodes"] if n["kind"] == "drawing"]
        s = self.theirs["style"]
        want = {"id": "sifir", "name": self.theirs["name"], "color": s["color"], "lineType": s["lineType"],
                "lineWeight": s["lineWeight"]}
        self.assertEqual(layers, [want, want])

    def test_a_project_without_the_layer_gets_it_with_the_recipe_s_look(self) -> None:
        b = opened(setup_without(layer="sifir"))
        with self.assertWarns(CommandWarning) as said:
            out = cad.road.project.produce(b, path=self.path)
        self.assertIn("recipe_layer_created", str(said.warning))
        [went] = out.layers
        self.assertEqual((went.name, went.to_name, str(went.how)), (self.theirs["name"], self.theirs["name"], "created"))
        added = next(n for n in flat(b.layers()) if n.id == went.to)
        s = self.theirs["style"]
        self.assertEqual((added.name, added.style.color, str(added.style.line_type), added.style.line_weight),
                         (self.theirs["name"], s["color"], s["lineType"], s["lineWeight"]))
        for p, mine in zip(out.produced, self.drawn):
            self.assertEqual(lines_of(b, list(p.uids)), lines_of(self.a, list(mine.uids)), "B's lines are A's")
            self.assertTrue(all(b.entity(u).entity.to_json()["layerId"] == went.to for u in p.uids))
        again = cad.road.project.produce(b, path=self.path)
        self.assertEqual(([p.drawing for p in again.produced], [lf.why for lf in again.left]), ([], ["exists", "exists"]))

    def test_a_layer_of_the_name_written_otherwise_is_found(self) -> None:
        c = opened(setup_without(rename=("sifir", "zero", "SIFIR HATTI")))
        self.assertEqual(fold("SIFIR HATTI"), fold(self.theirs["name"]))
        out = cad.road.project.produce(c, path=self.path)
        [went] = out.layers
        self.assertEqual((went.to, went.to_name, str(went.how)), ("zero", "SIFIR HATTI", "name"))
        graph = cad.road.project.graph(c)
        kept = [n.recipe.input["layerId"] for n in graph.nodes if n.kind == "drawing"]
        self.assertEqual(kept, ["zero", "zero"])
        self.assertEqual((graph.drawings, graph.outdated), (2, 0))

    def test_two_layers_of_the_name_are_asked_and_the_one_given_is_taken(self) -> None:
        d = opened(setup_without(twin="ikiz"))
        asked = cad.road.project.produce(d, path=self.path)
        self.assertEqual([lf.why for lf in asked.left], ["refused", "refused"])
        self.assertIn("2 katman var (sifir, ikiz)", asked.left[0].message)
        given = cad.road.project.produce(d, path=self.path, layers={"sifir": "ikiz"})
        [went] = given.layers
        self.assertEqual((went.to, str(went.how)), ("ikiz", "given"))
        self.assertEqual(len(given.produced), 2)


if __name__ == "__main__":
    unittest.main()
