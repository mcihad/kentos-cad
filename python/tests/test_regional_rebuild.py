"""`road.project.rebuild`'s `regional` from Python (docs/adr/4320; plan/map-cad.md MAP-14): a stale contour drawing drawn
again writing only what changed, held by the test's own reading.

On two copies of the shared cases' map project the ground's contour lines are drawn and one point of the second hill is
raised by 0.6 m; one copy is drawn again with `regional`, the other whole. The test then reads, itself:

- the drawing is the whole one, object for object in its order (geometry and look, numbers left out);
- the lines that stayed are those of the old drawing alike to a line of the new one (the test's own multiset match of
  their JSON), with their ids; the rest of the old ones are gone;
- `region` is the box of the old lines removed and the new ones written, from their places;
- written to bytes and read back, the drawing is current with the same objects.
"""

from __future__ import annotations

import json
import sys
import unittest
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/road.project.rebuild.regional.json").read_text("utf-8"))
UIDS = CASES["setup"]["uids"]


def drawn() -> cad.Document:
    """The cases' project, its contour lines drawn and the hill's point raised (their first case's two steps)."""
    doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))
    for step in CASES["cases"][0]["steps"][:2]:
        text = json.dumps(step["input"])
        for slot in range(len(UIDS), 0, -1):
            text = text.replace(f'"$uidOf:{slot}"', json.dumps(UIDS[slot - 1]))
        answer = doc.run(step["command"], json.loads(text))
        assert answer["status"] == "completed", answer
    return doc


def look(doc: cad.Document, uid: str) -> str:
    data = doc.entity(uid).entity.to_json()
    data.pop("id", None)
    return json.dumps(data, sort_keys=True)


def places(doc: cad.Document, uid: str) -> list[tuple[float, float]]:
    return [(p["x"], p["y"]) for p in doc.entity(uid).entity.to_json()["pts"]]


class RegionalRebuild(unittest.TestCase):
    def test_only_what_changed_is_written(self) -> None:
        part, whole = drawn(), drawn()
        old = [r.uid for r in part.entities(layer="esyukselti")]
        old_looks = {u: look(part, u) for u in old}
        old_places = {u: places(part, u) for u in old}

        made = cad.road.project.rebuild.execute_input(part, cad.RoadProjectRebuild.from_json({"regional": True}))
        full = cad.road.project.rebuild.execute_input(whole, cad.RoadProjectRebuild.from_json({}))
        (r,), (w,) = made.rebuilt, full.rebuilt
        self.assertIs(w.kept, cad.UNSET)

        # The drawing is the whole one, object for object.
        self.assertEqual([look(part, u) for u in r.uids], [look(whole, u) for u in w.uids])

        # The test's own match: each new line takes an old one alike not taken yet.
        free = Counter(old_looks.values())
        stayed = []
        for u in w.uids:
            k = look(whole, u)
            if free[k] > 0:
                free[k] -= 1
                stayed.append(k)
        self.assertEqual(r.kept, len(stayed))
        kept_uids = [u for u in r.uids if u in old_looks]
        self.assertEqual(sorted(old_looks[u] for u in kept_uids), sorted(stayed))
        gone = [u for u in old if u not in r.uids]
        self.assertEqual(r.removed, len(gone))
        now = {e.uid for e in part.entities(layer="esyukselti")}
        self.assertTrue(now.isdisjoint(gone))
        self.assertEqual(now, set(r.uids))

        # Where it changed: the box of the lines removed and written.
        written = [u for u in r.uids if u not in old_looks]
        xy = [p for u in gone for p in old_places[u]] + [p for u in written for p in places(part, u)]
        box = [min(x for x, _ in xy), min(y for _, y in xy), max(x for x, _ in xy), max(y for _, y in xy)]
        for got, want in zip(r.region, box):
            self.assertAlmostEqual(got, want, delta=1e-9)

        # Written and read back: current, the same objects.
        back = cad.Document.from_bytes(part.to_bytes())
        tree = cad.road.project.query.execute_input(back, cad.RoadProjectQuery.from_json({}))
        drawing = next(n for n in tree.nodes if n.kind == "drawing")
        self.assertEqual(drawing.state, "current")
        again = cad.road.project.rebuild.execute_input(
            back, cad.RoadProjectRebuild.from_json({"drawings": ["cz-1"], "regional": True}))
        self.assertEqual((again.rebuilt[0].removed, again.rebuilt[0].kept), (0, len(r.uids)))
        self.assertEqual(list(again.rebuilt[0].uids), list(r.uids))


if __name__ == "__main__":
    unittest.main()
