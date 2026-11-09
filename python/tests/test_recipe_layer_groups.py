"""A recipe layer's groups, whole style and line type in another project (`road.project.produce`, docs/adr/4040;
plan/map-cad.md MAP-20), held by an independent reading (no KentOS code in the expectations).

Project A is scripts/fixtures/road_terrain_zeroline_cases.py's Yol project with its zero line's layer “Sıfır hattı”
put in the groups Harita › Toprak işleri, its style naming the project line type SINIR (a dash, a gap, a dot, a gap in
millimetres on paper; its nearest built-in type dash-dot) with a fill and a label. Its zero lines are drawn there and
İş grafiği's graph is saved as JSON with Python's own `json`, as Tarifi kaydet… would. The recipe keeps the groups, the
style as A's file holds it and SINIR's definition as A's settings hold it. Produced from it:

- in B, the project without that layer, its groups and line type, the layer is added in the groups Harita › Toprak
  işleri (both added last at the top), with A's style as it is, and SINIR is added to B's line types as A defines it;
  B's lines are A's, coordinate for coordinate; produced again, the layer is found by its name and nothing is written;
- in C, which has a group “HARİTA” (folded as Katmanları çevir folds names: İ → i, then lower case, it is Harita) and
  a line type of its own named “Sinir” (another pattern), the layer goes in C's group, in a Toprak işleri added in it,
  and its style names C's own line type; C's line types are as they were;
- in D, whose surfaces have other names (so the zero lines' sources are not there), every drawing is left: the layer,
  the groups and the line type added for it are taken away again, and D's tree and line types are as they were.
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
SINIR = {"name": "SINIR", "description": "Kazı sınırı", "elements": [{"length": 4.0}, {"length": -1.0}, {"length": 0.5},
                                                                    {"length": -1.0}], "unit": "paper"}
OWN = {"name": "Sinir", "description": "C'nin sınırı", "elements": [{"length": 6.0}, {"length": -2.0}], "unit": "paper"}
STYLE = {"color": "#2A9D8F", "lineType": "dashdot", "linePattern": "SINIR", "lineWeight": 0.35, "fill": "#2A9D8F",
         "label": {"placement": "along", "size": 10.0}}


def fold(name: str) -> str:
    """Katmanları çevir's folding, by hand: I → ı, İ → i, then lower case."""
    return name.replace("I", "ı").replace("İ", "i").lower()


def group(id_: str, name: str, children: list) -> dict:
    return {"id": id_, "name": name, "type": "group", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children}


def project_a() -> dict:
    s = zc.setup(work="road")
    layer = next(n for n in s["layers"] if n["id"] == "sifir")
    s["layers"] = [n for n in s["layers"] if n["id"] != "sifir"]
    layer["style"] = copy.deepcopy(STYLE)
    s["layers"].append(group("harita", "Harita", [group("toprak", "Toprak işleri", [layer])]))
    s["settings"]["lineTypes"] = [copy.deepcopy(SINIR)]
    return s


def without_layer(**changes) -> dict:
    s = zc.setup(work="road")
    s["layers"] = [n for n in s["layers"] if n["id"] != "sifir"]
    if "group" in changes:
        s["layers"].append(group("g-1", changes["group"], []))
    if "line_type" in changes:
        s["settings"]["lineTypes"] = [copy.deepcopy(changes["line_type"])]
    if changes.get("renamed"):
        for t in s["settings"]["road"]["surfaces"]:
            t["name"] += " (eski)"
    return s


def opened(setup: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def lines_of(doc: cad.Document, uids: list[str]) -> list[list[tuple[float, float]]]:
    return [[(p["x"], p["y"]) for p in doc.entity(u).entity.to_json()["pts"]] for u in uids]


def tree(doc: cad.Document) -> list[dict]:
    return [n.to_json() for n in doc.layers()]


def line_types(doc: cad.Document) -> list[dict]:
    return doc.settings.to_json().get("lineTypes", [])


def path_to(nodes: list[dict], id_: str, above: tuple = ()) -> tuple | None:
    """The names of the groups above `id_`, from the top, and the node."""
    for n in nodes:
        if n["id"] == id_:
            return above, n
        found = path_to(n.get("children", []), id_, (*above, n["name"]))
        if found is not None:
            return found
    return None


class RecipeLayerGroups(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.folder = tempfile.TemporaryDirectory()
        cls.a_setup = project_a()
        a = opened(cls.a_setup)
        cls.drawn = [cad.road.terrains.zeroline(a, base=b, comparison=c, layer_id="sifir") for b, c in PAIRS]
        cls.a = a
        path = Path(cls.folder.name) / "tarif.json"
        path.write_text(json.dumps(cad.road.project.graph(a).to_json(), ensure_ascii=False), "utf-8")
        cls.path = str(path)
        cls.recipe = json.loads(path.read_text("utf-8"))

    @classmethod
    def tearDownClass(cls) -> None:
        cls.folder.cleanup()

    def test_the_recipe_keeps_the_groups_the_style_and_the_line_type(self) -> None:
        for layer in [n["recipe"]["layer"] for n in self.recipe["nodes"] if n["kind"] == "drawing"]:
            self.assertEqual(layer["group"], ["Harita", "Toprak işleri"])
            self.assertEqual(layer["style"], STYLE)
            self.assertEqual(layer["lineTypeDef"], SINIR)
            self.assertEqual((layer["color"], layer["lineType"], layer["lineWeight"]),
                             (STYLE["color"], STYLE["lineType"], STYLE["lineWeight"]))

    def test_a_project_without_them_gets_the_groups_the_style_and_the_line_type(self) -> None:
        b = opened(without_layer())
        before = tree(b)
        out = cad.road.project.produce(b, path=self.path)
        [went] = out.layers
        self.assertEqual((str(went.how), list(went.group), list(went.groups_added), went.line_type_added),
                         ("created", ["Harita", "Toprak işleri"], ["Harita", "Toprak işleri"], "SINIR"))
        now = tree(b)
        self.assertEqual(now[:len(before)], before, "the tree before it stays")
        above, node = path_to(now, went.to)
        self.assertEqual(above, ("Harita", "Toprak işleri"))
        self.assertEqual(now[-1]["name"], "Harita", "the group is added last at the top")
        self.assertEqual((node["name"], node["style"]), ("Sıfır hattı", STYLE))
        self.assertEqual(line_types(b), [SINIR])
        for p, mine in zip(out.produced, self.drawn):
            self.assertEqual(lines_of(b, list(p.uids)), lines_of(self.a, list(mine.uids)), "B's lines are A's")
        again = cad.road.project.produce(b, path=self.path)
        self.assertEqual(([p.drawing for p in again.produced], [lf.why for lf in again.left]), ([], ["exists", "exists"]))

    def test_a_group_and_a_line_type_of_the_name_are_this_project_s(self) -> None:
        self.assertEqual(fold("HARİTA"), fold("Harita"))
        c = opened(without_layer(group="HARİTA", line_type=OWN))
        out = cad.road.project.produce(c, path=self.path)
        [went] = out.layers
        said = went.to_json()
        self.assertEqual((said["group"], said["groupsAdded"], said.get("lineTypeAdded")),
                         (["HARİTA", "Toprak işleri"], ["Toprak işleri"], None))
        above, node = path_to(tree(c), went.to)
        self.assertEqual(above, ("HARİTA", "Toprak işleri"))
        self.assertEqual(node["style"], dict(STYLE, linePattern="Sinir"))
        self.assertEqual(line_types(c), [OWN])

    def test_what_was_added_for_a_layer_no_drawing_went_on_is_taken_away(self) -> None:
        d = opened(without_layer(renamed=True))
        before, types = tree(d), line_types(d)
        out = cad.road.project.produce(d, path=self.path)
        self.assertEqual(([p.drawing for p in out.produced], [lf.why for lf in out.left]), ([], ["refused", "refused"]))
        self.assertEqual((tree(d), line_types(d)), (before, types))


if __name__ == "__main__":
    unittest.main()
