"""A baseline session's independent baselines (`map.survey.query`'s `independent`, `map.survey.edit`'s `independent`;
docs/adr/5320, plan/map-cad.md MAP-03), held by an independent reading: the file is read by tools/kcad/kcad.py, the
reader written apart from KentOS, and the n − 1 rule is the test's own (a spanning forest grown shortest first).
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import MapSurveyOperation  # noqa: E402

import kcad  # noqa: E402  (the independent reader)

SAMPLE = ROOT / "fixtures/kcad/v2/map-survey-sessions.kcad"


def mine(survey, session):
    """The test's own rule: shortest first (lower id number of equals), a baseline joining two unjoined marks."""
    by = {b["id"]: b for b in survey["baselines"]}
    bs = [by[i] for i in session["baselines"] if i in by and "excluded" not in by[i]]
    bs.sort(key=lambda b: (math.sqrt(sum(x * x for x in b["d"])), int(b["id"].split("-")[1])))
    group = {}

    def find(x):
        while group.get(x, x) != x:
            x = group[x]
        return x
    free = []
    for b in bs:
        a, c = find(b["from"]), find(b["to"])
        if a != c:
            group[a] = c
            free.append(b["id"])
    return free


class Independent(unittest.TestCase):
    def test_the_query_marks_and_the_edit_applies_the_n_minus_one_rule(self) -> None:
        doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        _, saved = kcad.read(doc.to_bytes())
        survey = saved["settings"]["mapSurvey"]
        rows = {b["id"]: b for b in cad.map.survey.query(doc).to_json()["baselines"]}
        want = {}
        for s in survey["baselineSessions"]:
            free = mine(survey, s)
            marks = {m for i in s["baselines"] for b in survey["baselines"] if b["id"] == i for m in (b["from"], b["to"])}
            self.assertLessEqual(len(free), len(marks) - 1)
            for i in s["baselines"]:
                if "excluded" not in next(b for b in survey["baselines"] if b["id"] == i):
                    self.assertEqual(rows[i].get("independent"), i in free, i)
                    want[i] = i in free
        try:
            cad.map.survey.edit(doc, operation=MapSurveyOperation.from_json({"op": "independent"}))
        except CommandFailed as e:
            self.assertEqual(e.code, "nothing_to_set")
            self.assertTrue(all(b["adjust"] == want.get(b["id"], b["adjust"]) for b in survey["baselines"]))
            return
        _, after = kcad.read(doc.to_bytes())
        for b in after["settings"]["mapSurvey"]["baselines"]:
            if b["id"] in want:
                self.assertEqual(b["adjust"], want[b["id"]], b["id"])


if __name__ == "__main__":
    unittest.main()
