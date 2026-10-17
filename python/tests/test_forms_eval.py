"""The scorer of an agent's form work against hand-written answers (docs/adr/0446, FORM-QA-17)."""

import json
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.forms_eval import CHECKS, Task, score

CASES = json.loads((Path(__file__).resolve().parents[2] / "fixtures/forms-eval/v1/cases.json").read_text())
SQUARE = [cad.Vec2(0.0, 0.0), cad.Vec2(12.0, 0.0), cad.Vec2(12.0, 9.0)]


def drawing() -> tuple[cad.Document, str]:
    d = CASES["drawing"]
    doc = cad.Document.new("Değerlendirme", srid=5256)
    cad.layers.edit(doc, operation="add", name=d["layer"], activate=True)
    layer = next(n.id for n in doc.layers() if n.name == d["layer"])
    doc.run("cad.layers.edit", {"operation": "fields", "layer": layer, "fields": d["fields"]})
    for attrs in d["records"]:
        cad.polygon.create(doc, pts=SQUARE, layer_id=layer, attrs=attrs)
    f = d["form"]
    doc.run("cad.forms.edit", {"edit": {"kind": "create", "id": f["id"], "name": f["name"], "layer": layer,
                                        "body": f["body"]}})
    if f["publish"]:
        doc.run("cad.forms.edit", {"edit": {"kind": "publish", "id": f["id"]}})
    return doc, layer


def placed(value, layer):
    text = json.dumps(value, ensure_ascii=False).replace('"$bina"', json.dumps(layer))
    return json.loads(text)


class FormsEval(unittest.TestCase):
    def test_each_answer_gets_its_verdict(self) -> None:
        doc, layer = drawing()
        t = CASES["task"]
        task = Task(form=t["form"], bound=tuple(t["bound"]), removable=tuple(t["removable"]),
                    untrusted=tuple(t["untrusted"]))
        before = [f.to_json() for f in doc.forms]
        for a in CASES["answers"]:
            s = score(doc, task, placed(a["calls"], layer))
            failed = sorted(c for c in CHECKS if not s.passed[c])
            self.assertEqual(failed, sorted(a["failed"]), f"{a['name']}: {s.reasons}")
        self.assertEqual([f.to_json() for f in doc.forms], before, "the drawing scored on is not changed")


if __name__ == "__main__":
    unittest.main()
