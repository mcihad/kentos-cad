"""An institution's own data profile from Python (docs/adr/0558, plan/gis.md CBS-34's remainder): a profile and a mapping
the user brings go into the project's ``profiller`` folder beside the drawing's file (``kentos.cad.profiles.import_``),
the folder is listed (``list``), the data of a project at release 1 is moved to release 3 by the profile's own steps
(``migrate``) as one undo step, and the profile is applied and checked through the institution's mapping. The drawing's
file is filled in by the host, as for every command that names it (``drawing``). The shared cases hold the details
(fixtures/commands/v1/cad.profiles.*.json, written without KentOS code); this is the typed layer end to end.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
FILES = ROOT / "fixtures/profiles/v1/files"
MIGRATE = json.loads((ROOT / "fixtures/commands/v1/cad.profiles.migrate.json").read_text(encoding="utf-8"))


def project() -> cad.Document:
    """The release-1 park drawing; the commands are told where its file would be (`drawing`)."""
    return cad.Document.from_bytes(json.dumps(MIGRATE["cases"][0]["setup"]).encode())


class ProfileFilesTests(unittest.TestCase):
    def test_a_profile_is_imported_listed_and_the_data_migrated_in_one_undo_step(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = project()
            drawing = str(Path(tmp) / "park.kcad")
            plan = cad.profiles.import_.plan(doc, source=str(FILES / "park-bahce.profil.json"), drawing=drawing)
            self.assertFalse((Path(tmp) / "profiller").exists(), "a plan writes nothing")
            done = cad.profiles.import_(doc, source=str(FILES / "park-bahce.profil.json"), drawing=drawing)
            self.assertEqual((done.name, done.release, done.migrations, done.classes), ("park-bahce", 3, 2, 2))
            self.assertEqual(done.path, plan.path)
            self.assertEqual(Path(done.path), Path(tmp) / "profiller" / "park-bahce.profil.json")
            self.assertFalse(doc.info().can_undo, "importing a file changes no drawing")
            listed = cad.profiles.list(doc, drawing=drawing)
            self.assertEqual([i.name for i in listed.items], ["tucbs-temel", "park-bahce"])
            self.assertEqual(listed.items[1].source.value, "project")
            self.assertEqual(listed.items[1].release, 3)
            # The data moves from release 1 to 3 by the profile's steps; a plan says what it will do.
            planned = cad.profiles.migrate.plan(doc, name="park-bahce", from_=1, drawing=drawing)
            self.assertFalse(doc.info().can_undo)
            moved = cad.profiles.migrate(doc, name="park-bahce", from_=1, drawing=drawing)
            self.assertEqual((moved.from_, moved.to, moved.changed), (1, 3, True))
            self.assertEqual([s.kind for s in moved.steps][:4], ["renameLayer", "renameField", "renameField", "changeFieldType"])
            self.assertEqual([s.applied for s in moved.steps], [s.applied for s in planned.steps])
            kept = {(lo.field, lo.value): lo.count for lo in moved.losses}
            self.assertEqual(kept[("Yas", "on beş")], 2, "a value that does not fit is kept and counted")
            self.assertTrue(doc.info().can_undo)
            self.assertEqual(doc.undo(), "Profil göçü")
            # The fields are back as they were; the layer keeps its new name (a layer's name is no undo step).
            names = {n.name: [f.name for f in (n.fields or [])] for n in doc.layers()[0].children}
            self.assertEqual(names["Ağaç"], ["No", "Tur", "Yas", "Sulama", "Alan"])

    def test_a_mapping_makes_the_profile_read_with_the_institutions_names(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = project()
            drawing = str(Path(tmp) / "park.kcad")
            mapping = json.loads((FILES / "sivas-bel.eslem.json").read_text(encoding="utf-8"))
            cad.profiles.import_(doc, source=str(FILES / "sivas-bel.eslem.json"), drawing=drawing)
            self.assertTrue((Path(tmp) / "profiller" / "sivas-bel.eslem.json").is_file())
            # Without the mapping the institution's names are not the profile's; nothing is renamed by itself.
            plain = cad.profiles.check(doc, name="tucbs-temel")
            through = cad.profiles.check(doc, name="tucbs-temel", mapping_name="sivas-bel", drawing=drawing)
            self.assertIs(plain.mapping, cad.UNSET)
            self.assertEqual(through.mapping, "sivas-bel")
            layers = {c.class_: c for c in through.classes}
            self.assertIn("parsel", layers)
            self.assertEqual(mapping["classes"][0]["layer"], ["Kadastro", "Parseller"])
            planned = cad.profiles.apply.plan(doc, name="tucbs-temel", mapping_name="sivas-bel", drawing=drawing)
            self.assertTrue(planned.layers, "the layers the drawing lacks are added at the institution's paths")
            self.assertFalse(doc.info().can_undo, "a plan writes nothing")

    def test_what_is_wrong_is_refused_with_its_code(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = project()
            drawing = str(Path(tmp) / "park.kcad")
            with self.assertRaises(cad.CommandFailed) as refused:
                cad.profiles.import_(doc, source=str(FILES / "profil-surum2.json"), drawing=drawing)
            self.assertEqual(refused.exception.code, "unsupported_profile_version")
            with self.assertRaises(cad.CommandFailed) as refused:
                cad.profiles.migrate(doc, profile=cad.DataProfile.from_json(json.loads((FILES / "park-bahce.profil.json").read_text(encoding="utf-8"))), from_=3)
            self.assertEqual(refused.exception.code, "invalid_migration")
            with self.assertRaises(cad.CommandFailed) as refused:
                cad.profiles.check(doc, name="tucbs-temel", mapping_name="yok", drawing=drawing)
            self.assertEqual(refused.exception.code, "unknown_mapping")

    def test_the_hosts_own_file_names_the_folder(self) -> None:
        # A drawing with a file of its own: the host says where it is, so `drawing` need not be given.
        setup = {
            "format": "kentos.document", "version": 1, "name": "Bos",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False,
                        "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                        "children": []}],
            "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []},
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "bos.kcad"
            cad.Document.from_bytes(json.dumps(setup).encode()).save(path)
            doc = cad.Document.open(path)
            done = cad.profiles.import_(doc, source=str(FILES / "peyzaj.profil.json"))
            self.assertEqual(Path(done.path), Path(tmp) / "profiller" / "peyzaj.profil.json")
            self.assertEqual(cad.profiles.list(doc).folder, str(Path(tmp) / "profiller"))
            # The project's own profile is applied by its name.
            applied = cad.profiles.apply(doc, name="peyzaj")
            self.assertTrue(applied.changed)


if __name__ == "__main__":
    unittest.main()
