"""Associative arrays from Python (docs/adr/0300): ``kentos.cad.entities.array`` with
``associative=True`` makes the objects a block and the places its inserts, one array the
document keeps; ``Document.arrays()`` reads it, the block is ``Document.blocks()``'s, the
items are ordinary inserts, a file keeps it (schema 53) and one undo takes it all back.

The shared cases (fixtures/commands/v1/cad.entities.array.assoc.json) run here too through
test_command_cases.py; this test is the reading side.
"""

from __future__ import annotations

import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
EDITS = ROOT / "fixtures/interaction/v1/edits.kcad"


def drawing() -> cad.Document:
    return cad.Document.from_bytes(EDITS.read_bytes())


def uid_of(doc: cad.Document, slot: int) -> str:
    return next(r.uid for r in doc.entities(page_size=1000) if r.entity.id == slot)


class ArrayTests(unittest.TestCase):
    def test_an_associative_grid_is_a_block_its_inserts_and_the_array(self) -> None:
        doc = drawing()
        line = uid_of(doc, 2)
        made = cad.entities.array(
            doc, uids=[line], layout=cad.GridArrayLayout(rows=2, cols=2, dx=5.0, dy=4.0), associative=True
        )
        self.assertIsNotNone(made.array)
        self.assertEqual(len(made.created), 4)
        arrays = doc.arrays()
        self.assertEqual(len(arrays), 1)
        array = arrays[0]
        self.assertEqual(array["id"], made.array)
        self.assertEqual([i["uid"] for i in array["items"]], made.created)
        self.assertEqual(array["layout"], {"kind": "grid", "rows": 2, "cols": 2, "dx": 5.0, "dy": 4.0})
        block = next(b for b in doc.blocks() if b.id == array["block"])
        self.assertEqual(block.name, "Dizi 1")
        kinds = {r.entity.to_json()["kind"] for r in doc.entities(page_size=1000) if r.uid in made.created}
        self.assertEqual(kinds, {"insert"})
        # A file keeps the array; one undo takes the block, the items and the array back.
        again = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(again.arrays(), arrays)
        self.assertEqual(doc.undo(), "Dizi")
        self.assertEqual(doc.arrays(), [])
        self.assertEqual(doc.blocks(), [])


if __name__ == "__main__":
    unittest.main()
