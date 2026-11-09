"""Sembolü karardan ver (`cad.zoning.symbolize`, docs/adr/4240; plan/map-cad.md MAP-09), held by the test's own reading
of the system library (no KentOS code in the expectations).

The drawing is the shared cases' (scripts/fixtures/zoning_symbols_cases.py): plan boundaries UİP-1 (uip) and NİP-1
(nip), decision areas with their uses. The test finds the symbols itself, by name, among the library's fill symbols of
MPYY › Uygulama imar planı, and checks MAP-09's acceptance:

- the decision gives the symbol: the park area takes the symbol named “Park”; the area whose symbol said Park but whose
  decision is ticaret takes “Ticaret alanı” (no meaning is read from a symbol);
- a symbol changed later (the user's pick for park) changes no decision: every attribute stays as it was;
- the saved file carries the symbols, read back by the independent reader;
- one undo takes the symbols back.
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

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.zoning.symbolize.json").read_text("utf-8"))
LIBRARY = json.loads((ROOT / "crates/native/style/assets/system-library.kstil").read_text("utf-8"))["items"]


def uip_fill(name: str) -> str:
    """The id of MPYY › Uygulama imar planı's fill symbol of this name; exactly one."""
    found = [i["id"] for i in LIBRARY if i["path"][:2] == ["MPYY", "Uygulama imar planı"]
             and (i.get("symbol") or {}).get("type") == "fill" and i["name"] == name]
    assert len(found) == 1, (name, found)
    return found[0]


def slot_of(decision: str) -> int:
    """The slot of the decision area with this KararKimlik."""
    return next(e["id"] for e in CASES["setup"]["entities"] if e["attrs"].get("KararKimlik") == decision)


class ZoningSymbols(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))

    def saved(self) -> dict:
        _, d = kcad.read(self.doc.to_bytes())
        return dict(zip(d["uids"], d["entities"]))

    def uid(self, decision: str) -> str:
        return CASES["setup"]["uids"][slot_of(decision) - 1]

    def test_the_decision_gives_the_symbol(self) -> None:
        before = self.saved()
        out = cad.zoning.symbolize(self.doc).to_json()
        after = self.saved()
        park, wrong = self.uid("K1"), self.uid("K4")
        self.assertEqual(after[park]["symbol"], uip_fill("Park"))
        # K4's symbol said Park, its decision is ticaret: the decision wins.
        self.assertEqual(before[wrong]["symbol"], uip_fill("Park"))
        self.assertEqual(after[wrong]["symbol"], uip_fill("Ticaret alanı"))
        self.assertEqual(after[wrong]["attrs"]["Kullanim"], "ticaret")
        # The konut area has no symbol named as its use: it is left open, not guessed.
        konut = self.uid("K3")
        self.assertNotIn("symbol", after[konut])
        self.assertEqual(next(r for r in out["areas"] if r["uid"] == konut)["state"], "unmapped")

    def test_a_symbol_changed_later_changes_no_decision(self) -> None:
        cad.zoning.symbolize(self.doc)
        decided = {u: e["attrs"] for u, e in self.saved().items()}
        arboretum = uip_fill("Arboretum - botanik parkı")
        pick = cad.ZoningSymbolPick.from_json({"level": "uip", "use": "park", "symbol": arboretum})
        out = cad.zoning.symbolize(self.doc, symbols=[pick]).to_json()
        park = self.uid("K1")
        row = next(r for r in out["areas"] if r["uid"] == park)
        self.assertEqual((row["state"], row["from"], row["was"]), ("set", "user", uip_fill("Park")))
        after = self.saved()
        self.assertEqual(after[park]["symbol"], arboretum)
        self.assertEqual({u: e["attrs"] for u, e in after.items()}, decided)

    def test_one_undo_takes_the_symbols_back(self) -> None:
        before = self.saved()
        cad.zoning.symbolize(self.doc)
        self.doc.undo()
        self.assertEqual(self.saved(), before)


if __name__ == "__main__":
    unittest.main()
