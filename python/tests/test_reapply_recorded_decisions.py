"""Detay eşleştirme's recorded decisions taken again by a model's new run (`map.features.reconcile`'s `reapply`,
docs/adr/4540; plan/map-cad.md MAP-04), held by the test's own rule.

The drawing is the shared case's (fixtures/commands/v1/map.features.reconcile.json, “Yeniden çıkarım”): a reviewed run
left four decisions as resolved review notes (a manhole accepted, a tree corrected to a pole, a door given its number,
an unknown rejected), marked their candidates and wrote assets; a model's new run put five point candidates on the two
sources' layers. By the test's rule (the reviewer's corrections kept when the model runs again):

- a candidate not marked takes the decision recorded on the marked candidate of its own source nearest to it within
  the radius (0.5 m, plain distance), ties to the smaller note's number; one farther than the radius takes none;
- with the decision taken, a rejection rejects it, a class corrects it (the tree read again is the pole written then),
  fields given leave none missing;
- a decision given in the run comes before a recorded one; without `reapply` nothing is taken;
- Eşleştir's new notes, one for each candidate taken, name the note taken again.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/map.features.reconcile.json").read_text("utf-8"))
CASE = next(c for c in CASES["cases"] if c["name"].startswith("Yeniden çıkarım, plan"))
SETUP = CASE["setup"]
INPUT = CASE["steps"][0]["input"]
SOURCES = {s["layer"]: s["name"] for s in INPUT["sources"]}


def expected() -> list[tuple[str, int]]:
    """The test's own rule over the drawing: (candidate, note) taken, in the drawing's order."""
    points = [(u, SOURCES[e["layerId"]], (e["p"]["x"], e["p"]["y"]), bool(e["attrs"].get("Eşleştirme") or e["attrs"].get("Uzlaştırma")))
              for e, u in zip(SETUP["entities"], SETUP["uids"]) if e["kind"] == "point" and e["layerId"] in SOURCES]
    marked = {u: (source, at) for u, source, at, done in points if done}
    notes = [(n["number"], n["decision"]) for n in SETUP["settings"]["reviewNotes"] if n["status"] == "resolved"]
    taken = []
    for u, source, at, done in points:
        if done:
            continue
        near = [(math.dist(at, marked[d["candidate"]][1]), number) for number, d in notes
                if d["source"] == source and d["candidate"] in marked and marked[d["candidate"]][0] == source
                and math.dist(at, marked[d["candidate"]][1]) <= INPUT["radius"]]
        if near:
            taken.append((u, min(near)[1]))
    return taken


def run(doc: cad.Document, op: str = "plan", **extra) -> dict:
    command = cad.command("map.features.reconcile")
    answer = command.run(doc, command.input_type.from_json(dict(INPUT, **extra)), op=op)
    assert answer.error is None, answer.error and answer.error.message
    return answer.output.to_json()


class ReapplyRecordedDecisions(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(SETUP)))

    def test_the_nearest_recorded_decision_of_the_same_source_is_taken(self) -> None:
        want = expected()
        self.assertEqual(len(want), 4, "four of the five new candidates are near a decided one")
        out = run(self.doc)
        self.assertEqual([(r["candidate"], r["note"]) for r in out["reapplied"]], want)
        decisions = {n["number"]: n["decision"] for n in SETUP["settings"]["reviewNotes"]}
        candidates = {c["uid"]: c for c in out["candidates"]}
        for uid, note in want:
            d = decisions[note]
            if "reject" in d:
                self.assertEqual(candidates[uid]["status"], "rejected")
            if "class" in d:
                self.assertEqual(candidates[uid]["class"], d["class"])
            if "fields" in d:
                group = out["groups"][candidates[uid]["group"]]
                self.assertFalse(set(d["fields"]) & set(group.get("missing", [])))

    def test_without_reapply_nothing_is_taken(self) -> None:
        out = run(self.doc, reapply=False)
        self.assertNotIn("reapplied", out)

    def test_a_decision_given_comes_first(self) -> None:
        first = expected()[0][0]
        out = run(self.doc, decisions=[{"candidate": first, "class": "agac"}])
        self.assertNotIn(first, [r["candidate"] for r in out["reapplied"]])
        self.assertEqual(next(c for c in out["candidates"] if c["uid"] == first)["class"], "agac")

    def test_the_new_notes_name_the_note_taken_again(self) -> None:
        want = dict(expected())
        out = run(self.doc, op="execute", reviewer="Ayşe Demir")
        notes = {n["note"]["number"]: n["note"] for n in cad.review.notes.list(self.doc).to_json()["notes"]}
        new = [notes[k] for k in out["notes"]]
        self.assertEqual({n["decision"]["candidate"] for n in new}, set(want))
        for n in new:
            self.assertTrue(n["text"].endswith(f" Önceki karar (not {want[n['decision']['candidate']]}) yeniden uygulandı."), n["text"])
        self.doc.undo()
        self.assertEqual(len(cad.review.notes.list(self.doc).to_json()["notes"]), 4)


if __name__ == "__main__":
    unittest.main()
