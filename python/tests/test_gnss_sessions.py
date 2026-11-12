"""GNSS baseline sessions, new points' velocities and the place limits kept in the project (`map.survey.network`'s
`correlations` and `velocities`, `map.survey.edit`'s `baselines` and `tolerance`; docs/adr/4920, plan/map-cad.md
MAP-03), held by the test's own rules and read back by the independent .kcad reader (tools/kcad/kcad.py).

- A GVX session's blocks are kept as the file gives them (Python's ElementTree here), nine numbers row by row.
- A session whose cross-correlations are all zero adds nothing: the coordinates are the independent baselines'.
- `correlations: session` says which sessions went in; without it, they are said to be left out.
- The kept limits are written with their source at schema 248 and judge the known points' places.
- An accepted network point keeps its velocity with the grid's name and SHA-256 (the campaign's grid).
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from xml.etree import ElementTree as ET

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402

import kcad  # noqa: E402  Independent saved-file reader.
import kcad_v2_reference as kr  # noqa: E402

NETWORK = json.loads((ROOT / "fixtures/commands/v1/map.survey.network.json").read_text("utf-8"))
EDIT = json.loads((ROOT / "fixtures/commands/v1/map.survey.edit.json").read_text("utf-8"))


def setup(cases: dict, starts: str) -> dict:
    case = next(c for c in cases["cases"] if c["name"].startswith(starts))
    return case.get("setup", cases.get("setup"))


def opened(snapshot: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(snapshot)))


def run(doc: cad.Document, command: str, input_: dict, op: str = "plan") -> dict:
    c = cad.command(command)
    answer = c.run(doc, c.input_type.from_json(input_), op=op)
    assert answer.error is None, answer.error and answer.error.message
    return answer.output.to_json()


class GnssSessions(unittest.TestCase):
    def test_a_gvx_session_is_kept_as_the_file_gives_it(self) -> None:
        doc = opened(setup(EDIT, "GVX 1.0 dosyasından sekiz baz"))
        path = ROOT / "fixtures/map-survey/v1/ag.gvx"
        run(doc, "map.survey.edit", {"operation": {"op": "baselines", "campaign": "kp-1", "file": str(path)}},
            op="execute")
        root = ET.fromstring(path.read_bytes())
        block = root.find("SESSION/CROSS_CORRELATION_MATRIX/CCM_BLOCK")
        want = [float(x) for x in block.findtext("CORRELATIONS").split(",")]
        saved = kcad.read(doc.to_bytes())[1]["settings"]["mapSurvey"]
        session = saved["baselineSessions"][0]
        self.assertEqual(session["name"], root.find("SESSION").get("ID"))
        by_source = {b["source"]: b["id"] for b in saved["baselines"]}
        self.assertEqual(session["baselines"], [by_source[block.get("VECTOR_ID_ROW")], by_source[block.get("VECTOR_ID_COL")]])
        self.assertEqual(session["blocks"][0]["r"], want)

    def test_zero_cross_correlations_add_nothing(self) -> None:
        zero = opened(setup(NETWORK, "Çapraz korelasyonları sıfır oturum"))
        with_ = run(zero, "map.survey.network", {"correlations": "session"})
        without = run(zero, "map.survey.network", {"correlations": "none"})
        self.assertEqual(with_["correlations"]["sessions"], ["bo-1", "bo-2"])
        self.assertEqual(without["correlations"]["unused"], ["bo-1", "bo-2"])
        for a, b in zip(with_["stations"], without["stations"]):
            for k in ("lat", "lon", "height"):
                self.assertAlmostEqual(a[k], b[k], delta=1e-10, msg=(a["mark"], k))
        self.assertAlmostEqual(with_["constrained"]["omega"], without["constrained"]["omega"], delta=1e-9)

    def test_correlations_move_the_marks(self) -> None:
        doc = opened(setup(NETWORK, "İki baz oturumu"))
        with_ = {s["mark"]: s for s in run(doc, "map.survey.network", {"correlations": "session"})["stations"]}
        without = {s["mark"]: s for s in run(doc, "map.survey.network", {})["stations"]}
        moved = max(abs(with_[m]["height"] - without[m]["height"]) for m in ("N1", "N4", "N5"))
        self.assertGreater(moved, 1e-6)
        self.assertLess(moved, 0.01, "correlations move a mark by millimetres, not more")

    def test_the_kept_limits_are_written_with_their_source(self) -> None:
        doc = opened(setup(NETWORK, "İki baz oturumu"))
        source = "İdarenin şartnamesi (örnek)"
        run(doc, "map.survey.edit", {"operation": {"op": "tolerance", "horizontal": 0.005, "source": source}},
            op="execute")
        data = doc.to_bytes()
        document = kcad.read(data)[1]
        self.assertEqual(document["settings"]["mapSurvey"]["tolerance"], {"horizontal": 0.005, "source": source})
        # The payload's schema: the reader's own, 240.
        self.assertEqual(kcad.decode_payload(kcad.read_header(data)[1])["version"], kcad.SCHEMA_WITH_GNSS_SESSIONS)
        tree = run(doc, "map.survey.query", {"places": True})
        self.assertEqual(tree["tolerance"]["source"], source)

    def test_an_accepted_point_keeps_its_velocity_and_its_grid(self) -> None:
        snapshot = setup(NETWORK, "Yeni noktaların hızı")
        grid = (ROOT / "fixtures/geoid/v1/hiz.tif").read_bytes()
        import hashlib
        sha = hashlib.sha256(grid).hexdigest()
        with tempfile.TemporaryDirectory() as library:
            (Path(library) / f"{sha}.tif").write_bytes(grid)
            os.environ["KENTOS_GRID_DIR"] = library
            try:
                doc = opened(snapshot)
                answer = run(doc, "map.survey.network", {"velocities": True})
                run(doc, "map.survey.edit", {"operation": {"op": "network", "velocities": True}},
                    op="execute")
            finally:
                os.environ.pop("KENTOS_GRID_DIR", None)
        self.assertEqual(answer["velocityGrid"]["sha256"], sha)
        saved = kcad.read(doc.to_bytes())[1]["settings"]["mapSurvey"]
        n1 = next(p for p in saved["points"] if p["mark"] == "N1")["result"]
        self.assertEqual(n1["velocity"]["grid"]["sha256"], sha)
        want = next(s for s in answer["stations"] if s["mark"] == "N1")["velocity"]
        for a, b in zip(n1["velocity"]["enu"], want):
            self.assertAlmostEqual(a, b, delta=1e-12)


if __name__ == "__main__":
    unittest.main()
