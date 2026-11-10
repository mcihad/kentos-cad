"""Katalog geçiş raporu (`cad.profiles.compare`, docs/adr/4940; plan/map-cad.md MAP-04), held by this test's own
readings of the shared drawing and of the two catalogs.

- Every old class's row counts the objects of its own layer, read here from the drawing: the layer the class's path
  leads to.
- A class the new catalog keeps by id is `same` or `changed`; one whose codes all went to one class of another id is
  `renamed` or `merged`, and that class holds the old codes.
- The rows that need a decision are counted in `decide`.
- The new catalog kept in the project's `profiller` folder is found by its name, as the catalog given by itself is.
- Nothing is written: the revision stays.
"""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import DataProfile  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.profiles.compare.json").read_text("utf-8"))
NEW = CASES["cases"][0]["steps"][1]["input"]["toProfile"]
OLD = json.loads((ROOT / "catalog/v1/profiles/harita-varliklari.json").read_text("utf-8"))


def opened() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))


def layer_objects(setup: dict, path: list[str]) -> int:
    nodes, lid = setup["layers"], None
    for name in path:
        node = next((n for n in nodes if n["name"] == name), None)
        if node is None:
            return 0
        lid, nodes = node["id"], node.get("children", [])
    return sum(1 for e in setup["entities"] if e["layerId"] == lid)


def word(kind: object) -> str:
    return str(getattr(kind, "value", kind))


class CatalogReport(unittest.TestCase):
    def test_the_rows_count_their_layers_objects_and_follow_ids_then_codes(self) -> None:
        got = cad.profiles.compare.plan(opened(), from_="harita-varliklari", to_profile=DataProfile.from_json(NEW))
        old = {c["id"]: c for c in OLD["classes"]}
        new = {c["id"]: c for c in NEW["classes"]}
        for row in got.classes:
            if not isinstance(row.from_, str):
                self.assertEqual(row.objects, 0)
                continue
            c = old[row.from_]
            self.assertEqual(row.objects, layer_objects(CASES["setup"], c["layer"]), row.from_)
            if row.from_ in new:
                self.assertIn(word(row.kind), ("same", "changed"), row.from_)
            elif word(row.kind) in ("renamed", "merged"):
                self.assertEqual(len(row.to), 1)
                self.assertTrue(set(c["codes"]) <= set(new[row.to[0]]["codes"]), row.from_)
        deciding = sum(r.decide for rows in (got.classes, got.fields, got.codes, got.decisions) for r in rows)
        self.assertEqual(got.decide, deciding)
        self.assertGreater(got.decide, 0)

    def test_nothing_is_written_and_the_folders_catalog_is_found_by_name(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "katalog.kcad"
            opened().save(path)
            (Path(tmp) / "profiller").mkdir()
            (Path(tmp) / "profiller" / "belediye-harita.profil.json").write_text(
                json.dumps(NEW, ensure_ascii=False), "utf-8")
            doc = cad.Document.open(path)
            before = doc.revision
            by_name = cad.profiles.compare(doc, from_="harita-varliklari", to="belediye-harita")
            by_itself = cad.profiles.compare.plan(doc, from_="harita-varliklari", to_profile=DataProfile.from_json(NEW))
            self.assertEqual(by_name.to_json(), by_itself.to_json())
            self.assertEqual(doc.revision, before, "a report writes nothing")
            with self.assertRaises(CommandFailed) as refused:
                cad.profiles.compare(doc, from_="harita-varliklari", to="yok-boyle")
            self.assertEqual(refused.exception.code, "unknown_profile")


if __name__ == "__main__":
    unittest.main()
