"""Detay eşleştirme's decisions recorded as review notes, apart from the objects (`map.features.reconcile`,
docs/adr/4140; plan/map-cad.md MAP-04), read back through the review notes' own list and through the `.kcad` file by
the independent reader (tools/kcad/kcad.py, written apart from KentOS).

The drawing is the shared cases' (fixtures/commands/v1/map.features.reconcile.json: a LiDAR and an orthophoto
extraction's candidates, the drawing's own note 3). With a reviewer and a time:

- the four decisions (accept, class, fields, reject) are the notes 4–7, resolved, by the reviewer at that time; each
  bound to its candidate, and all but the rejection to the asset its group was written as; their decisions are the ones
  given, with the source;
- the asset carries no `İnceleme` nor `Gözden geçiren`; the candidate is marked `Eşleştirme` as before;
- right after, every anchor is current; the asset changed afterwards, its notes are stale (their stamps were the state
  the decision left);
- the saved file is schema 215 and holds the decisions as written;
- one undo takes the assets, the marks and the notes away together.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402
from kentos.cad.types import CandidateDecision, CandidateSource  # noqa: E402

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/map.features.reconcile.json").read_text("utf-8"))
CASE = next(c for c in CASES["cases"] if c["name"].startswith("Kararlar inceleme notu"))
INPUT = CASE["steps"][0]["input"]


class ReviewNotes(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASE["setup"])))
        self.out = cad.map.features.reconcile(
            self.doc, name=INPUT["name"], sources=[CandidateSource.from_json(s) for s in INPUT["sources"]],
            confidence=INPUT["confidence"], radius=INPUT["radius"], reviewer="Ayşe Demir",
            reviewed_at="2026-11-09T16:00:00+03:00",
            decisions=[CandidateDecision.from_json(d) for d in INPUT["decisions"]])

    def listed(self) -> dict[int, dict]:
        notes = cad.review.notes.list(self.doc).to_json()["notes"]
        return {n["note"]["number"]: n for n in notes}

    def test_each_decision_is_a_resolved_note_bound_to_its_candidate_and_asset(self) -> None:
        out = self.out.to_json()
        self.assertEqual(out["notes"], [4, 5, 6, 7])
        notes = self.listed()
        self.assertEqual(sorted(notes), [3, 4, 5, 6, 7])
        for number, given in zip((4, 5, 6, 7), INPUT["decisions"]):
            n = notes[number]["note"]
            self.assertEqual((n["status"], n["author"], n["createdAt"]), ("resolved", "Ayşe Demir", "2026-11-09T16:00:00+03:00"))
            d = n["decision"]
            self.assertEqual(d["candidate"], given["candidate"])
            for key in ("accept", "class", "fields", "reject"):
                self.assertEqual(d.get(key), given.get(key), key)
            bound = [a["uid"] for a in n["anchors"]]
            if "reject" in given:
                self.assertEqual(bound, [given["candidate"]])
                self.assertNotIn("asset", d)
            else:
                self.assertEqual(bound, [given["candidate"], d["asset"]])
                self.assertIn(d["asset"], out["written"])
            self.assertFalse(notes[number]["stale"], "current right after")

    def test_the_asset_carries_no_review_attributes(self) -> None:
        _, saved = kcad.read(self.doc.to_bytes())
        by_uid = dict(zip(saved["uids"], saved["entities"]))
        for asset in self.out.to_json()["written"]:
            attrs = by_uid[asset]["attrs"]
            self.assertNotIn("İnceleme", attrs)
            self.assertNotIn("Gözden geçiren", attrs)
        self.assertEqual(by_uid[INPUT["decisions"][0]["candidate"]]["attrs"]["Eşleştirme"],
                         self.listed()[4]["note"]["decision"]["asset"])

    def test_the_file_is_schema_215_and_keeps_the_decisions(self) -> None:
        data = self.doc.to_bytes()
        _, payload = kcad.read_header(data)
        self.assertEqual(kcad.decode_payload(payload)["version"], 215)
        _, saved = kcad.read(data)
        kept = {n["number"]: n for n in saved["settings"]["reviewNotes"]}
        self.assertEqual(kept[7]["decision"]["reject"], "Gölge; nesne yok")
        self.assertEqual(kept[5]["decision"]["class"], "direk")

    def test_a_changed_asset_makes_its_notes_stale(self) -> None:
        asset = self.listed()[4]["note"]["decision"]["asset"]
        cad.entities.set(self.doc, uids=[asset], operation="attributes", attrs={"Durum": "Sökülecek"})
        notes = self.listed()
        self.assertTrue(notes[4]["stale"])
        self.assertFalse(notes[7]["stale"], "the rejection is not bound to it")

    def test_one_undo_takes_the_notes_away_with_the_assets(self) -> None:
        self.doc.undo()
        self.assertEqual(sorted(self.listed()), [3])


if __name__ == "__main__":
    unittest.main()
