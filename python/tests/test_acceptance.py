"""The first end-to-end acceptance drawing from Python (TODOS.md CADP-12, plan/cad.md §23.1 K01–K04, K11, K12): the shared
scenario (fixtures/acceptance/v1/room.json, worked out by scripts/fixtures/acceptance_room.py without KentOS code)
replayed through kentos.cad, every step's objects, measures and sheets held to it, its DXF and PDF read back
without KentOS code (acceptance_files.py). The headless session, the MCP server and
the desktop replay the same file."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from typing import Any

import kentos.cad as cad

from acceptance_files import dxf_problems, pdf_problems, pdf_text_problems

ROOT = Path(__file__).resolve().parents[2]
SCENARIO = json.loads((ROOT / "fixtures/acceptance/v1/room.json").read_text(encoding="utf-8"))


def matches(actual: Any, expected: Any, tolerance: float, at: str) -> list[str]:
    """Where ``actual`` differs from ``expected``: only the fields ``expected`` names; a point ``[x, y]`` is
    ``{"x", "y"}`` in a drawing; numbers to ``tolerance``."""
    if isinstance(expected, list) and len(expected) == 2 and all(isinstance(v, (int, float)) for v in expected) \
            and isinstance(actual, dict) and "x" in actual:
        actual = [actual["x"], actual["y"]]
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return [f"{at}: {actual!r}, beklenen {expected!r}"]
        out: list[str] = []
        for k, v in expected.items():
            out += matches(actual.get(k), v, tolerance, f"{at}.{k}")
        return out
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(actual) != len(expected):
            return [f"{at}: {actual!r}, beklenen {expected!r}"]
        out = []
        for i, (a, e) in enumerate(zip(actual, expected)):
            out += matches(a, e, tolerance, f"{at}[{i}]")
        return out
    if isinstance(expected, float) or (isinstance(expected, int) and not isinstance(expected, bool)):
        ok = isinstance(actual, (int, float)) and abs(actual - expected) <= tolerance
        return [] if ok else [f"{at}: {actual!r}, beklenen {expected!r}"]
    return [] if actual == expected else [f"{at}: {actual!r}, beklenen {expected!r}"]


def bind(value: Any, names: dict[str, str]) -> Any:
    if isinstance(value, str) and value.startswith("$"):
        return names[value[1:]]
    if isinstance(value, dict):
        return {k: bind(v, names) for k, v in value.items()}
    if isinstance(value, list):
        return [bind(v, names) for v in value]
    return value


class Acceptance(unittest.TestCase):
    def test_the_room_is_drawn_corrected_dimensioned_and_hatched_as_the_scenario_says(self) -> None:
        p = SCENARIO["project"]

        def new() -> cad.Document:
            return cad.Document.new(p["name"], workspace=p["workspace"], coords=p["coords"],
                                    drawing_unit=p["drawingUnit"], plot_scale=p["plotScale"])

        doc = new()
        names: dict[str, str] = {"layer": doc.active_layer}
        # The journal is the session's (docs/adr/0253 §3): taken before every reopening.
        rows: list[dict[str, Any]] = []
        tolerance = SCENARIO["tolerance"]
        for step in SCENARIO["steps"]:
            name = step["name"]
            if "command" in step:
                answer = doc.run(step["command"], bind(step["input"], names))
                self.assertEqual(answer.get("status"), "completed", f"{name}: {answer}")
                output = answer["output"]
                bound = step.get("as")
                if isinstance(bound, str):
                    names[bound] = output["uid"]
                elif isinstance(bound, list):
                    names.update(zip(bound, output["created"]))
            elif "undo" in step:
                self.assertEqual(doc.undo(), step["undo"], name)
            elif step.get("reopen"):
                rows += doc.journal()["calls"]
                doc = cad.Document.from_bytes(doc.to_bytes())
            problems: list[str] = []
            expect = step.get("expect", {})
            if step.get("export") == "dxf":
                with tempfile.TemporaryDirectory() as d:
                    written = doc.export_dxf(Path(d) / "oda.dxf")
                    problems += dxf_problems(written.path.read_bytes(), expect["dxf"])
            elif step.get("export") == "pdf":
                with tempfile.TemporaryDirectory() as d:
                    printed = doc.export_pdf(Path(d) / "oda.pdf")
                    problems += pdf_problems(printed.path.read_bytes(), expect["pdf"])
                    texts = pdf_text_problems(str(printed.path), expect["pdf"])
                    if texts is None:
                        print("pdftotext yok: PDF'in yazıları denetlenmedi (poppler kurun).")
                    else:
                        problems += texts
            if "sheets" in expect:
                problems += matches([x.to_json() for x in doc.sheets()], expect["sheets"], tolerance, "paftalar")
            for key, want in expect.get("objects", {}).items():
                problems += matches(doc.entity(names[key]).entity.to_json(), want, tolerance, key)
            for key, want in expect.get("measures", {}).items():
                m = doc.measure(names[key])
                problems += matches({"area": m.area, "length": m.length}, want, tolerance, f"{key} ölçü")
            if "count" in expect:
                problems += matches(len(doc), expect["count"], 0, "nesne sayısı")
            self.assertEqual(problems, [], name)
        self.replays_the_journal(doc, rows + doc.journal()["calls"], new())

    def replays_the_journal(self, doc: cad.Document, rows: list[dict[str, Any]], fresh: cad.Document) -> None:
        """The record says what ran, in order, and replays (docs/adr/0253 §3): on a new drawing of the same
        project every command row done is the same, the undone ones are not run, the files are planned
        again, not written; the replayed drawing is the same drawing."""
        ran = [s["command"] for s in SCENARIO["steps"] if "command" in s]
        recorded = [r["command"] for r in rows if r["kind"] == "command" and not r.get("effects")]
        self.assertEqual(recorded, ran, "the record's commands are the scenario's, in order")
        self.assertEqual([r for r in rows if r["kind"] == "edit"], [], "every write went through a command")
        undone = sum(1 for s in SCENARIO["steps"] if "undo" in s)
        self.assertEqual(sum(1 for r in rows if r["status"] == "undone"), undone)
        report = fresh.replay(dict(doc.journal(), calls=rows))
        done = sum(1 for r in rows if r["kind"] == "command" and r["status"] == "done" and not r.get("effects"))
        wrong = [x for x in report.steps if x.outcome in ("differs", "failed")]
        # The files were written to folders gone since: planned again, they are skipped, never written.
        self.assertEqual((report.same, report.failed, report.differs), (done, 0, 0), wrong)
        # The same drawing: every object, by the id the replay made in its place.
        self.assertEqual(len(fresh), len(doc))
        for record in doc.entities():
            mine = record.entity.to_json()
            theirs = fresh.entity(report.mapped.get(record.uid, record.uid)).entity.to_json()
            self.assertEqual(json.loads(json.dumps(theirs)),
                             json.loads(rename(json.dumps(mine), report.mapped)), record.uid)


def rename(text: str, names: dict[str, str]) -> str:
    """``text`` with every id the replay made anew written as the replay's."""
    for old, new in names.items():
        text = text.replace(old, new)
    return text


if __name__ == "__main__":
    unittest.main()
