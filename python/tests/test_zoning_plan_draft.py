"""Plan değişikliği taslağı and the kept symbol choices (`cad.zoning.draft`, `cad.zoning.symbolize` with `keep`;
docs/adr/5040, plan/map-cad.md MAP-09), held by the test's own reasoning (no KentOS code in the expectations).

The drawings are the shared cases' (scripts/fixtures/zoning_draft_cases.py, zoning_symbols_cases.py). The test checks:

- a draft never writes the plan in force: every decision area's attributes and symbol are as before; the draft is in
  the saved file, read back by the independent reader, with the value each field had before;
- the comparison keeps the plan's area: the sum of the uses before is the sum after, and each use's area is the
  test's own shoelace sum over the plan's rectangles; the park made into trade is said under m. 26(3);
- a draft goes stale when the area it read changes: the zoning check says so (`draftStale`);
- a symbol choice kept in the project survives saving and opening, and the next run reads it (`from` `project`).
"""

from __future__ import annotations

import copy
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

DRAFT_CASES = json.loads((ROOT / "fixtures/commands/v1/cad.zoning.draft.json").read_text("utf-8"))
SYMBOL_CASES = json.loads((ROOT / "fixtures/commands/v1/cad.zoning.symbolize.json").read_text("utf-8"))
GELISME = "mpyy.uip.konut.gelisme-konut-alani"


def shoelace(pts: list[dict]) -> float:
    return abs(sum(a["x"] * b["y"] - b["x"] * a["y"] for a, b in zip(pts, pts[1:] + pts[:1]))) / 2


class PlanDraft(unittest.TestCase):
    def setUp(self) -> None:
        self.setup = DRAFT_CASES["setup"]
        self.doc = cad.Document.from_bytes(kr.container(kr.document(self.setup)))
        self.by_name = {e["attrs"].get("KararKimlik"): (u, e) for u, e in zip(self.setup["uids"], self.setup["entities"])}

    def saved(self) -> dict:
        return kcad.read(self.doc.to_bytes())[1]

    def change(self, name: str, field: str, value: str):
        return cad.ZoningDraftChange.from_json({"uid": self.by_name[name][0], "field": field, "value": value})

    def test_a_draft_keeps_the_plan_in_force_and_its_areas(self) -> None:
        before = self.saved()
        out = cad.zoning.draft(self.doc, id="T-1", plan="UİP-1", reason="Parkın yeri değişiyor.",
                               changes=[self.change("K1", "Kullanim", "ticaret")]).to_json()
        after = self.saved()
        self.assertEqual(after["entities"], before["entities"])
        kept = after["settings"]["zoningPlan"]["drafts"]
        self.assertEqual([(d["id"], d["plan"], d["changes"][0]["before"], d["changes"][0]["after"]) for d in kept],
                         [("T-1", "UİP-1", "park", "ticaret")])
        # The test's own areas: UİP-1's decision areas by use, the park counted as trade after.
        own_before, own_after = {}, {}
        for e in self.setup["entities"]:
            if e["layerId"] != "karar" or e["attrs"].get("Plan") != "UİP-1":
                continue
            use = e["attrs"].get("Kullanim", "")
            own_before[use] = own_before.get(use, 0.0) + shoelace(e["pts"])
            nxt = "ticaret" if e["attrs"].get("KararKimlik") == "K1" else use
            own_after[nxt] = own_after.get(nxt, 0.0) + shoelace(e["pts"])
        got = {u["use"]: (u["before"], u["after"]) for u in out["uses"]}
        for use in set(own_before) | set(own_after):
            self.assertAlmostEqual(got[use][0], own_before.get(use, 0.0), places=6)
            self.assertAlmostEqual(got[use][1], own_after.get(use, 0.0), places=6)
        self.assertAlmostEqual(sum(b for b, _ in got.values()), sum(a for _, a in got.values()), places=6)
        less = [f for f in out["findings"] if f["kind"] == "infrastructureLess"]
        self.assertEqual([(f["use"], round(f["area"], 6)) for f in less], [("park", round(own_before["park"], 6))])
        self.assertIn("m. 26(3)", less[0]["message"])
        self.doc.undo()
        self.assertNotIn("zoningPlan", self.saved()["settings"])

    def test_a_draft_goes_stale_when_its_area_changes(self) -> None:
        cad.zoning.draft(self.doc, id="T-1", plan="UİP-1", reason="Yoğunluk.",
                         changes=[self.change("K3", "KAKS", "1.50")])
        data = self.doc.to_bytes()
        self.assertEqual([f["kind"] for f in cad.zoning.check(cad.Document.from_bytes(data)).to_json()["findings"]
                          if f["kind"] == "draftStale"], [])
        # The plan in force changes after the draft: K3's KAKS is 1.30 now.
        snap = copy.deepcopy(self.setup)
        snap["settings"]["zoningPlan"] = kcad.read(data)[1]["settings"]["zoningPlan"]
        k3 = next(e for e in snap["entities"] if e["attrs"].get("KararKimlik") == "K3")
        k3["attrs"]["KAKS"] = "1.30"
        stale = [f for f in cad.zoning.check(cad.Document.from_bytes(kr.container(kr.document(snap)))).to_json()["findings"]
                 if f["kind"] == "draftStale"]
        self.assertEqual([f["uids"] for f in stale], [[self.by_name["K3"][0]]])
        self.assertIn("“1.20”, şimdi “1.30”", stale[0]["message"])


class KeptSymbols(unittest.TestCase):
    def test_a_kept_choice_survives_saving_and_is_read_next_time(self) -> None:
        doc = cad.Document.from_bytes(kr.container(kr.document(SYMBOL_CASES["setup"])))
        pick = cad.ZoningSymbolPick.from_json({"level": "uip", "use": "konut", "symbol": GELISME})
        cad.zoning.symbolize(doc, symbols=[pick], keep=True)
        data = doc.to_bytes()
        self.assertEqual(kcad.read(data)[1]["settings"]["zoningPlan"]["symbols"],
                         [{"level": "uip", "use": "konut", "symbol": GELISME}])
        again = cad.Document.from_bytes(data)
        rows = cad.zoning.symbolize.plan(again).to_json()["areas"]
        konut = [r for r in rows if r["use"] == "konut" and r["level"] == "uip"]
        self.assertTrue(konut)
        self.assertEqual({(r["symbol"], r["from"], r["state"]) for r in konut}, {(GELISME, "project", "same")})


if __name__ == "__main__":
    unittest.main()
