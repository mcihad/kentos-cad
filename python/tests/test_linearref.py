"""Events kept on their routes (docs/adr/0569) through Python: a link defined with ``kentos.cad.linearref.define``
(its ``set`` and ``remove``), then a write that moves the route is followed in its own undo step, one that cannot place an
event is said with ``events_dirty``, and what the reference says of the shared cases
(fixtures/commands/v1/cad.linearref.follow.json, scripts/fixtures/linearref_follow_cases.py) holds on a case by hand."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/commands/v1/cad.linearref.follow.json").read_text(encoding="utf-8"))


def open_setup() -> cad.Document:
    return cad.Document.from_bytes(json.dumps(CASES["setup"], ensure_ascii=False).encode("utf-8"))


def by_id(doc: cad.Document) -> dict[int, Any]:
    return {r.entity.id: r for r in doc.entities(page_size=1000)}


class LinearEventsTest(unittest.TestCase):
    def test_a_moved_route_takes_its_events_with_it_in_one_undo_step(self) -> None:
        doc = open_setup()
        before = by_id(doc)
        route = before[1]
        answer = doc.run("cad.entities.transform", {"uids": [route.uid], "transform": {"kind": "move", "dx": 0.0, "dy": 10.0}})
        self.assertEqual(answer["status"], "completed", answer)
        self.assertEqual(answer["warnings"], [])
        after = by_id(doc)
        # The point of Km 40, offset 2 to the right of the route, follows it 10 m north; so does the line event.
        self.assertAlmostEqual(after[3].entity.p.y - before[3].entity.p.y, 10.0, places=6)
        self.assertAlmostEqual(after[6].entity.a.y - before[6].entity.a.y, 10.0, places=6)
        # The events of the other route stay.
        self.assertEqual(after[4].entity.p, before[4].entity.p)
        self.assertIsNotNone(doc.undo())
        undone = by_id(doc)
        self.assertAlmostEqual(undone[3].entity.p.y, before[3].entity.p.y, places=6)
        self.assertAlmostEqual(undone[1].entity.a.y, before[1].entity.a.y, places=6)

    def test_a_route_that_is_gone_leaves_its_events_and_says_so(self) -> None:
        doc = open_setup()
        route = by_id(doc)[1]
        answer = doc.run("cad.entities.delete", {"uids": [route.uid]})
        self.assertEqual(answer["status"], "completed", answer)
        codes = [w["code"] for w in answer["warnings"]]
        self.assertEqual(codes, ["events_dirty", "events_dirty"], answer["warnings"])
        self.assertIn("“D-100” rotası yok", answer["warnings"][0]["message"])

    def test_the_link_is_defined_and_taken_away_through_the_sdk(self) -> None:
        doc = cad.Document.from_bytes(json.dumps({**CASES["setup"], "settings": {k: v for k, v in CASES["setup"]["settings"].items() if k != "linearEvents"}}, ensure_ascii=False).encode("utf-8"))
        link = CASES["setup"]["settings"]["linearEvents"][0]
        done = cad.linearref.define(doc, operation="set", link=cad.EventLink.from_json(link))
        self.assertEqual([l.id for l in done.links], [link["id"]])
        again = cad.linearref.define(doc, operation="remove", id=link["id"])
        self.assertEqual(again.links, [])
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.linearref.define(doc, operation="remove", id=link["id"])
        self.assertEqual(refused.exception.code, "unknown_event_link")


if __name__ == "__main__":
    unittest.main()
