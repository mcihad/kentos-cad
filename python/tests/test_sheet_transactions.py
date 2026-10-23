"""Shared model/paper history cases through real doc.group and the typed command host (ADR 0590)."""
import json
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/sheet/v1/model-transactions.json").read_text())


def state(doc):
    s = json.loads(doc._session.summary())
    return {key: s.get(key) for key in ["objects", "revision", "dirty", "canUndo", "canRedo", "stamps"]}


class SheetTransactionTests(unittest.TestCase):
    def test_model_and_paper_transaction_cases(self):
        self.assertGreaterEqual(len(CASES["cases"]), 20)
        for case in CASES["cases"]:
            with self.subTest(case["name"]):
                doc = cad.Document.from_bytes(json.dumps(CASES["setup"]).encode())
                context = None
                for step in case["actions"]:
                    with self.subTest(step=step):
                        before = state(doc)
                        action = step["action"]
                        if action == "begin":
                            context = doc.group("Birlikte")
                            context.__enter__()
                        elif action == "end":
                            context.__exit__(None, None, None)
                            context = None
                        elif action == "cancel":
                            error = RuntimeError("script cancelled")
                            context.__exit__(type(error), error, None)
                            context = None
                        elif action in ["undo", "redo"]:
                            method = getattr(doc, action)
                            if "error" in step:
                                with self.assertRaises(cad.HostError) as caught:
                                    method()
                                self.assertEqual(caught.exception.code, step["error"])
                            else:
                                self.assertEqual(method() is not None, step["taken"])
                        elif action == "command":
                            done = doc.run(step["command"], step["input"], op=step.get("op", "execute"))
                            self.assertEqual(done["status"], step.get("status", "completed"), done)
                            if "code" in step:
                                self.assertEqual(done["error"]["code"], step["code"])
                        else:
                            self.fail(action)
                        self.assertEqual(len(doc), step["expect"]["objects"])
                        self.assertEqual([s.name for s in doc.sheets()], step["expect"]["sheets"])
                        if step.get("unchanged"):
                            self.assertEqual(state(doc), before)
