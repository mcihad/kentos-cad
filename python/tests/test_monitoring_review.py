"""İzleme incelemesi (`cad.asbuilt.review`, docs/adr/5220; plan/map-cad.md MAP-19), held by the test's own reckoning:
the kept review's marks are the monitoring check's own tracks (last movement, speed, state), its digest is SHA-256 of
the points the job reads written here by the test (`k<TAB>layer`, `p<TAB>name<TAB>x<TAB>y<TAB>z`, six decimals), the
independent `.kcad` reader reads it back with the thresholds' source and the reviewer's statement; a point moved
after the review makes it belong to the earlier result.
"""

from __future__ import annotations

import copy
import hashlib
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad  # noqa: E402  (the independent reader)
import kcad_v2_reference as kr  # noqa: E402  (the independent writer)

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.asbuilt.review.json").read_text("utf-8"))


def digest(setup: dict, job: dict) -> str:
    text = ""
    for layer in [e["layer"] for e in job.get("epochs", [])] + [job["design"], job["measured"]]:
        text += f"k\t{layer}\n"
        for e in setup["entities"]:
            if e["kind"] == "point" and e["layerId"] == layer:
                z = "-" if e.get("z") is None else f"{e['z']:.6f}"
                text += f"p\t{e['attrs'].get('Ad', '').strip()}\t{e['p']['x']:.6f}\t{e['p']['y']:.6f}\t{z}\n"
    return hashlib.sha256(text.encode()).hexdigest()


class MonitoringReview(unittest.TestCase):
    def test_the_review_keeps_the_check_and_says_when_it_is_old(self) -> None:
        case = next(c for c in CASES["cases"] if c["name"].startswith("üç dönemli izleme"))
        setup = case.get("setup", CASES["setup"])
        step = case["steps"][0]
        doc = cad.Document.from_bytes(kr.container(kr.document(setup)))
        check = cad.asbuilt.check.plan_input(doc, cad.AsbuiltCheck.from_json(step["input"]["check"])).to_json()
        out = cad.asbuilt.review.execute_input(doc, cad.AsbuiltReview.from_json(step["input"])).to_json()
        kept = out["kept"]
        self.assertEqual(kept["digest"], digest(setup, kept["job"]))
        self.assertEqual([(m["name"], m.get("state")) for m in kept["marks"]],
                         [(t["name"], t.get("state")) for t in check["tracks"]])
        for m, t in zip(kept["marks"], check["tracks"]):
            self.assertAlmostEqual(m["horizontal"], t["steps"][-1]["horizontal"], places=12)
        self.assertEqual(out["reviews"][0]["current"], True)
        _, saved = kcad.read(doc.to_bytes())
        reviews = saved["settings"]["monitoringReviews"]
        self.assertEqual(reviews[0]["job"]["thresholds"]["source"], step["input"]["thresholdsSource"])
        self.assertEqual(reviews[0]["reviewer"]["phrase"], "checked")
        # M1 of the third epoch moved after the review: the review belongs to the earlier result.
        moved = copy.deepcopy(setup)
        moved["settings"]["monitoringReviews"] = reviews
        point = next(e for e in moved["entities"] if e["layerId"] == "donem3" and e["attrs"].get("Ad") == "M1")
        point["p"]["x"] += 0.004
        later = cad.Document.from_bytes(kr.container(kr.document(moved)))
        listed = cad.asbuilt.review.plan_input(later, cad.AsbuiltReview.from_json(
            {**step["input"], "name": "Kasım incelemesi"})).to_json()["reviews"]
        self.assertEqual([(r["name"], r["current"]) for r in listed],
                         [(reviews[0]["name"], False), ("Kasım incelemesi", True)])


if __name__ == "__main__":
    unittest.main()
