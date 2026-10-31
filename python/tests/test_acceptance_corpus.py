"""The acceptance corpus's other samples from Python (plan/architecture-advanced.md ADV-07, docs/adr/0798): a
measured change on an existing plan, a two-storey house, a curved plan with a shaft and a renovation over a DXF
underlay, each a shared scenario (fixtures/acceptance/v1/<id>.json, worked out by
scripts/fixtures/acceptance_corpus.py without KentOS code) replayed as the 2+1 apartment's is
(test_acceptance_flat.replay). With ``KENTOS_ACCEPTANCE_OUT`` each drawing is saved as ``<id>/python.kcad``; the MCP
server and the desktop's agent link replay the same files."""

from __future__ import annotations

import json
import unittest
from pathlib import Path

from test_acceptance_flat import replay

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ["change", "house", "curved", "renovation"]


class AcceptanceCorpus(unittest.TestCase):
    def test_every_sample_is_drawn_changed_and_reopened_as_its_scenario_says(self) -> None:
        for name in CORPUS:
            path = ROOT / f"fixtures/acceptance/v1/{name}.json"
            if not path.exists():
                continue
            with self.subTest(sample=name):
                replay(self, json.loads(path.read_text(encoding="utf-8")), f"{name}/python.kcad")


if __name__ == "__main__":
    unittest.main()
