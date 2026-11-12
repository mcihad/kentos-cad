"""Kataloğa taşı (`cad.profiles.carry`, docs/adr/5340; plan/map-cad.md MAP-04), held by an independent reading: the
report's shared drawing and catalogs (the first case of fixtures/commands/v1/cad.profiles.carry.json); the file after
the command is read by tools/kcad/kcad.py, the reader written apart from KentOS, and what should move is worked out here
from the objects' own field codes and the new catalog's classes.

- The control points split by their codes: K1 (KN) to the layer named as the GNSS points' class's path ends, P7 (PN) to
  the traverse points'.
- The tree's class is retired: its object stays on its old layer.
- One undo takes the step back: every object on its old layer.
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

CASE = json.loads((ROOT / "fixtures/commands/v1/cad.profiles.carry.json").read_text("utf-8"))["cases"][0]
NEW = CASE["steps"][2]["input"]["toProfile"]


def name_of(layers, lid):
    for n in layers:
        if n["id"] == lid:
            return n["name"]
        found = name_of(n.get("children", []), lid)
        if found:
            return found
    return None


def layer_names(doc: cad.Document):
    _, saved = kcad.read(doc.to_bytes())
    return {u: name_of(saved["layers"], e["layerId"]) for u, e in zip(saved["uids"], saved["entities"])}


class Carry(unittest.TestCase):
    def test_split_by_codes_the_retired_stays_and_one_undo(self) -> None:
        setup = json.loads(json.dumps(CASE["setup"]))
        doc = cad.Document.from_bytes(kr.container(kr.document(setup)))
        before = layer_names(doc)
        out = cad.profiles.carry(doc, from_="harita-varliklari", to_profile=cad.DataProfile.from_json(NEW)).to_json()
        after = layer_names(doc)
        ents = setup["entities"]
        uid = lambda i: setup["uids"][i]  # noqa: E731
        by_code = {c: k["layer"][-1] for k in NEW["classes"] for c in k.get("codes", [])}
        for i, e in enumerate(ents):
            code = e["attrs"].get("Kod", "")
            if e["layerId"] == "hv-kn":
                self.assertEqual(after[uid(i)], by_code[code], e["attrs"])
            if code == "AGC" and e["layerId"].startswith("hv-"):
                self.assertEqual(after[uid(i)], before[uid(i)], "the retired class's object stays")
        rows = {c["from"]: c for c in out["classes"]}
        self.assertEqual((rows["kontrol-noktasi"]["moved"], rows["agac"]["left"]), (2, 1))
        self.assertEqual(doc.undo(), "Kataloğa taşı")
        self.assertEqual(layer_names(doc), before)


if __name__ == "__main__":
    unittest.main()
