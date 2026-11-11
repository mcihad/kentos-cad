"""Kurumdaki alan adları (`cad.profiles.names`, docs/adr/5180; plan/map-cad.md MAP-17), held by this test's own readings:

- every field of the profile has one row, in the profile's order;
- a BÜYÜK_HARF name in ASCII is capitals, digits and single `_` between words; a lowerCamelCase one has no space and
  starts small; the mapping's own names stay as it writes them;
- `holds` is true exactly when no row has a problem, and the counts are the rows';
- a mapping that is the form alone, kept inside the drawing, makes Veri profilini uygula write the fields by their
  form's names, which Veri profilini denetle then finds; a mapping whose form puts two fields on one name is refused;
- nothing is written by the names.
"""

from __future__ import annotations

import json
import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import DataProfile, MappingNames, ProfileMapping  # noqa: E402


CASES = json.loads((ROOT / "fixtures/commands/v1/cad.profiles.names.json").read_text("utf-8"))
TUCBS = json.loads((ROOT / "catalog/v1/profiles/tucbs-temel.json").read_text("utf-8"))
SIVAS = json.loads((ROOT / "fixtures/profiles/v1/files/sivas-bel.eslem.json").read_text("utf-8"))
SNAKE = re.compile(r"^[A-Z0-9]+(_[A-Z0-9]+)*$")


def opened() -> cad.Document:
    return cad.Document.from_bytes(json.dumps(CASES["setup"]).encode())


def word(v: object) -> str:
    return str(getattr(v, "value", v))


def form(case: str, letters: str = "keep") -> MappingNames:
    return MappingNames.from_json({"case": case, "letters": letters})


def profile_named(name: str) -> dict:
    for c in CASES["cases"]:
        for s in c["steps"]:
            p = s.get("input", {}).get("profile")
            if isinstance(p, dict) and p.get("name") == name:
                return p
    raise KeyError(name)


class Names(unittest.TestCase):
    def test_every_field_has_its_row_in_the_form(self) -> None:
        doc = opened()
        before = doc.revision
        got = cad.profiles.names(doc, name="tucbs-temel", names=form("upperSnakeCase", "ascii"))
        fields = [(c["id"], f["name"]) for c in TUCBS["classes"] for f in c["fields"]]
        self.assertEqual([(r.class_, r.field) for r in got.rows], fields)
        for r in got.rows:
            self.assertEqual(word(r.source), "form")
            self.assertRegex(r.name, SNAKE, r.field)
        self.assertTrue(got.holds)
        camel = cad.profiles.names(doc, name="tucbs-temel", names=form("lowerCamelCase"))
        for r in camel.rows:
            self.assertNotIn(" ", r.name)
            self.assertTrue(r.name[0].islower(), r.name)
        self.assertEqual(doc.revision, before, "the names write nothing")

    def test_the_mappings_own_names_come_first_and_the_counts_are_the_rows(self) -> None:
        got = cad.profiles.names(opened(), name="tucbs-temel", mapping=ProfileMapping.from_json(SIVAS),
                                 names=form("upperSnakeCase", "ascii"))
        written = {(c["class"], f["profile"]): f["name"] for c in SIVAS["classes"] for f in c.get("fields", [])}
        for r in got.rows:
            key = (r.class_, r.field)
            if key in written:
                self.assertEqual((r.name, word(r.source)), (written[key], "mapping"))
        clash = cad.profiles.names(opened(), profile=DataProfile.from_json(profile_named("cakisan")),
                                   names=form("upperSnakeCase", "ascii"))
        problems = [word(r.problem) for r in clash.rows if isinstance(r.problem, str)]
        self.assertEqual(clash.unfit, problems.count("unfit"))
        self.assertEqual(clash.clashes, problems.count("clash"))
        self.assertFalse(clash.holds)
        self.assertEqual(clash.renamed, sum(1 for r in clash.rows if not isinstance(r.problem, str) and r.name != r.field))

    def test_apply_writes_the_forms_names_and_refuses_a_clashing_form(self) -> None:
        doc = opened()
        alone = {"format": "kentos.profile-mapping", "version": 1, "name": "buyuk-harf", "title": "Büyük harfli adlar",
                 "profile": "tucbs-temel", "names": {"case": "upperSnakeCase", "letters": "ascii"}, "classes": []}
        cad.profiles.import_(doc, mapping=ProfileMapping.from_json(alone), store="drawing")
        cad.profiles.apply(doc, name="tucbs-temel", mapping_name="buyuk-harf")

        def fields(nodes: list) -> list[str]:
            out: list[str] = []
            for n in nodes:
                out += [f.name for f in (n.fields if isinstance(n.fields, list) else [])]
                out += fields(n.children if isinstance(n.children, list) else [])
            return out

        names = fields(doc.layers())
        self.assertIn("MAHALLE_KODU", names)
        self.assertNotIn("MahalleKodu", names)
        for n in names:
            self.assertRegex(n, SNAKE)
        report = cad.profiles.check(doc, name="tucbs-temel", mapping_name="buyuk-harf")
        kinds = {word(f.kind) for f in report.findings}
        self.assertNotIn("missingField", kinds, "the form's names are found again")
        bad = dict(alone, name="cakisan-bicim", profile="cakisan")
        with self.assertRaises(CommandFailed) as refused:
            cad.profiles.apply(doc, profile=DataProfile.from_json(profile_named("cakisan")),
                               mapping=ProfileMapping.from_json(bad))
        self.assertEqual(refused.exception.code, "invalid_profile_mapping")


if __name__ == "__main__":
    unittest.main()
