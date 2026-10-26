"""The scorer of an agent's GIS work against hand-written answers (docs/adr/0653, plan/gis.md CBS-41)."""

import json
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path

from kentos.gis_eval import CHECKS, Task, build, cases, main, placed, report, score

CASES = cases()


class GisEval(unittest.TestCase):
    def test_each_answer_gets_its_verdict(self) -> None:
        doc, ids = build(CASES["drawing"])
        before = doc.revision
        scores = []
        for t in CASES["tasks"]:
            task = Task.from_json(t)
            for a in t["answers"]:
                s = score(doc, ids, task, placed(a["calls"], ids), a["said"])
                scores.append(s)
                failed = sorted(c for c in CHECKS if not s.passed[c])
                self.assertEqual(failed, sorted(a["failed"]), f"{t['id']} / {a['name']}: {s.reasons}")
        self.assertEqual(doc.revision, before, "the drawing scored on is not changed")
        r = report(scores)
        self.assertEqual(r.answers, sum(len(t["answers"]) for t in CASES["tasks"]))
        self.assertEqual(r.whole, sum(1 for t in CASES["tasks"] for a in t["answers"] if not a["failed"]))
        for c in CHECKS:
            want = sum(1 for t in CASES["tasks"] for a in t["answers"] if c not in a["failed"])
            self.assertEqual(r.passed[c], want, c)

    def test_a_run_of_answers_is_scored_from_its_file(self) -> None:
        run = [{"task": t["id"], "calls": a["calls"], "said": a["said"]} for t in CASES["tasks"] for a in t["answers"][:1]]
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "cevaplar.json"
            path.write_text(json.dumps(run, ensure_ascii=False), "utf-8")
            out = StringIO()
            with redirect_stdout(out):
                self.assertEqual(main([str(path)]), 0)
        text = out.getvalue()
        self.assertIn(f"{len(run)} cevap; bütünüyle doğru: {len(run)} (%100)", text)


if __name__ == "__main__":
    unittest.main()
