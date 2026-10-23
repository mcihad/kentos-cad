"""What the split and merged objects came from (docs/adr/0598) through Python: Birleştir names the lines merged into
the first, Kır's new piece names the line broken, undo takes a step's records back; ancestors and descendants walk them.
The shared cases (fixtures/commands/v1/cad.entities.*.lineage.json, scripts/fixtures/lineage_cases.py) say the same
step by step."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
JOIN = json.loads((ROOT / "fixtures/commands/v1/cad.entities.join.lineage.json").read_text(encoding="utf-8"))


def by_id(doc: cad.Document) -> dict[int, Any]:
    return {r.entity.id: r for r in doc.entities(page_size=1000)}


class LineageTest(unittest.TestCase):
    def test_a_join_and_a_break_keep_their_parents(self) -> None:
        doc = cad.Document.from_bytes(json.dumps(JOIN["setup"], ensure_ascii=False).encode("utf-8"))
        rows = by_id(doc)
        a, b, c = rows[17].uid, rows[18].uid, rows[19].uid
        joined = doc.run("cad.entities.join", {"uids": [a, b, c]})
        self.assertEqual(joined["status"], "completed", joined)
        self.assertEqual(
            [(r.child, r.parents, r.operation, r.step) for r in doc.lineage],
            [(a, [b, c], "merge", "Birleştir")],
        )
        broken = doc.run(
            "cad.entities.break",
            {"uid": a, "first": {"x": 487004, "y": 4420060}, "second": {"x": 487006, "y": 4420060}},
        )
        self.assertEqual(broken["status"], "completed", broken)
        piece = broken["output"]["created"][0]
        self.assertEqual(doc.ancestors(piece), [a, b, c])
        self.assertEqual(doc.descendants(b), [a, piece])
        self.assertEqual(doc.undo(), "Kır")
        self.assertEqual(doc.undo(), "Birleştir")
        self.assertEqual(doc.lineage, [])


if __name__ == "__main__":
    unittest.main()
