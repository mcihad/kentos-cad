"""The forms' developer examples (python/examples/forms, plan FORM-API-15): each runs end to end in a headless drawing
and gives what the README says."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

EXAMPLES = Path(__file__).resolve().parents[1] / "examples" / "forms"


def run(name: str) -> dict:
    spec = importlib.util.spec_from_file_location(f"example_{name}", EXAMPLES / f"{name}.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.main()


class ExamplesTests(unittest.TestCase):
    def test_building(self) -> None:
        out = run("building")
        self.assertFalse(out["asked_valid"])
        self.assertEqual(out["asked_issues"], ["form_required"])
        self.assertEqual(out["written"], {"Durum": "iyi", "Kat": "4", "Yükseklik": "12"})
        self.assertEqual(out["audit_failing"], 1, "the second building lacks its note")
        self.assertEqual(out["recomputed"], 2)
        self.assertEqual(out["height_now"], "16")

    def test_kinds(self) -> None:
        out = run("kinds")
        self.assertEqual(out["factory"], {"Tür": "sanayi", "Malzeme": "celik", "Çatı": "duz"})
        self.assertEqual(out["refusals"], ["invalid_attribute", "invalid_combination"])
        self.assertEqual(out["material_choices"], ["beton", "celik"])
        self.assertEqual(out["roof_choices"], ["duz"])

    def test_owners(self) -> None:
        out = run("owners")
        self.assertEqual(len(out["owners"]), 2)
        self.assertEqual(out["related"], out["owners"])
        self.assertEqual(out["copies"], 3, "the building and its two owners")
        self.assertEqual(out["left_after_delete"], 2, "the copy's owners stay")

    def test_address(self) -> None:
        out = run("address")
        self.assertEqual(out["districts_offered"], ["Çankaya", "Keçiören"])
        self.assertEqual(out["written"], {"İl": "06", "İlçe": "0601"})
        self.assertEqual(out["refused"], "form_invalid")


if __name__ == "__main__":
    unittest.main()
