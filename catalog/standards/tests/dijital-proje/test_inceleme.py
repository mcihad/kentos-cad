"""Gelen paketin incelemesi (docs/adr/8104): katman katman bulgular, KentOS'un ölçtüğü olgulardan. Modelin olguları
KentOS'un dosyasınınkidir (fixtures/ifc/v1/olgular/kesit-binasi-tam.json); paketin ve paftaların olguları burada
kurulur.

    python3 -I catalog/standards/tests/dijital-proje/test_inceleme.py
"""

import copy
import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "dijital-proje"
KOK = PAKET.parents[2]
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import inceleme  # noqa: E402

MODEL = "260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc"
PAFTA = "260123_00_MM_XXXX_PL_MMC_K00_01_000.pdf"
KAYNAK = {"revision": "9", "sha256": "a" * 64}


def ifc():
    return json.loads((KOK / "fixtures/ifc/v1/olgular/kesit-binasi-tam.json").read_text(encoding="utf-8"))


def olgular(**ust):
    o = {
        "paket": {"tur": "klasor", "liste": True, "listeHata": None, "kaynak": KAYNAK, "model": MODEL,
                  "formAdi": "kalite-kontrol-formu.json",
                  "dosyalar": [
                      {"ad": PAFTA, "tur": "pdf", "listede": True, "var": True, "baytTutar": True, "ozetTutar": True},
                      {"ad": MODEL, "tur": "ifc", "listede": True, "var": True, "baytTutar": True, "ozetTutar": True}]},
        "paftalar": [{"ad": PAFTA, "sayfa": 1, "pdfa": {"part": 2, "conformance": "B", "findings": []}, "taranmis": [],
                      "kaynak": KAYNAK}],
        "ifc": ifc(),
        "ifcHata": None,
        "modelOzeti": "b" * 64,
        "form": {"model": {"name": MODEL, "sha256": "b" * 64}, "rows": []},
        "idare": {"system": 5255},
    }
    o.update(ust)
    return o


def kodlar(cevap, katman=None):
    return [b["code"] for b in cevap["bulgular"] if katman is None or b["katman"] == katman]


class Inceleme(unittest.TestCase):
    def test_sound_package_holds_but_the_models_own_rows(self):
        c = inceleme.denetle(olgular())
        self.assertEqual(kodlar(c, "paket"), [])
        self.assertEqual(kodlar(c, "pafta"), [])
        self.assertEqual(kodlar(c, "konum"), [])
        self.assertEqual([k["key"] for k in c["katmanlar"]], inceleme.KATMANLAR)
        self.assertEqual(len(c["satirlar"]), 21)

    def test_another_zone_is_the_places(self):
        c = inceleme.denetle(olgular(idare={"system": 5256}))
        self.assertEqual(kodlar(c, "konum"), ["review_place_zone"])
        self.assertIn("EPSG:5256", next(b["message"] for b in c["bulgular"] if b["code"] == "review_place_zone"))

    def test_no_system_given_leaves_the_place_unjudged(self):
        c = inceleme.denetle(olgular(idare={}))
        konum = next(k for k in c["katmanlar"] if k["key"] == "konum")
        self.assertIsNone(konum["holds"])
        self.assertIn("EPSG:5255", konum["words"])

    def test_a_sheet_from_another_revision_is_the_sheets(self):
        p = copy.deepcopy(olgular())
        p["paftalar"][0]["kaynak"] = {"revision": "8", "sha256": "c" * 64}
        c = inceleme.denetle(p)
        self.assertEqual(kodlar(c, "pafta"), ["review_sheet_revision"])

    def test_scanned_and_not_pdfa(self):
        p = copy.deepcopy(olgular())
        p["paftalar"][0]["pdfa"] = {"part": None, "conformance": None, "findings": []}
        p["paftalar"][0]["taranmis"] = [1]
        c = inceleme.denetle(p)
        self.assertEqual(kodlar(c, "pafta"), ["review_sheet_not_pdfa", "review_sheet_scanned"])

    def test_the_list_against_the_folder(self):
        p = copy.deepcopy(olgular())
        p["paket"]["dosyalar"][0].update(var=False)
        p["paket"]["dosyalar"].append({"ad": "notlar.txt", "tur": "other", "listede": False, "var": True})
        c = inceleme.denetle(p)
        self.assertEqual(kodlar(c, "paket"), ["review_file_missing", "review_file_unlisted"])
        self.assertFalse(next(b for b in c["bulgular"] if b["code"] == "review_file_unlisted")["error"])

    def test_a_lone_model(self):
        p = olgular(paket={"tur": "dosya", "liste": False, "dosyalar": [], "model": MODEL}, paftalar=[], form=None)
        c = inceleme.denetle(p)
        self.assertEqual(kodlar(c, "paket"), ["review_model_only"])
        self.assertIsNone(next(k for k in c["katmanlar"] if k["key"] == "pafta")["holds"])
        self.assertIsNone(next(k for k in c["katmanlar"] if k["key"] == "form")["holds"])

    def test_a_model_that_does_not_read(self):
        c = inceleme.denetle(olgular(ifc=None, ifcHata="STEP dosyası değil"))
        self.assertEqual(kodlar(c, "model"), ["review_model_unreadable"])
        self.assertIsNone(next(k for k in c["katmanlar"] if k["key"] == "konum")["holds"])

    def test_a_form_of_another_model(self):
        p = olgular(form={"model": {"name": MODEL, "sha256": "d" * 64}, "rows": []})
        c = inceleme.denetle(p)
        self.assertIn("review_form_other_model", kodlar(c, "form"))

    def test_another_discipline_s_model_is_answered_by_table_9_2(self):
        # The section building's structural model by another program (docs/adr/8114): 16 rows, its place question the
        # fifth; an author's form of Table 9.1 is the wrong form, its rows not compared.
        statik = json.loads((KOK / "fixtures/ifc/v1/olgular/kesit-binasi-statik.json").read_text(encoding="utf-8"))
        statik["file"]["name"] = MODEL.replace("_MM_", "_ST_")
        c = inceleme.denetle(olgular(ifc=statik, form={"table": "9.1", "model": {"name": MODEL, "sha256": "b" * 64},
                                                       "rows": [{"no": 5, "answer": "Evet"}]}))
        self.assertEqual(c["tablo"], "9.2")
        self.assertEqual(len(c["satirlar"]), 16)
        self.assertIn("review_form_table", kodlar(c, "form"))
        self.assertNotIn("review_form_disagrees", kodlar(c, "form"))
        tablo = next(b for b in c["bulgular"] if b["code"] == "review_form_table")
        self.assertEqual(tablo["message"], "Form Tablo 9.1'in; model mimari dışı bir disiplinin, Tablo 9.2 düzenlenir.")
        # Its coordinate system's finding (Table 9.2 row 5 = 9.1's sixth) is the place layer's.
        self.assertIn("digital_system_wrong", kodlar(c, "konum"))
        self.assertNotIn("digital_system_wrong", kodlar(c, "model"))
        # The architectural model keeps Table 9.1.
        self.assertEqual(inceleme.denetle(olgular())["tablo"], "9.1")

    def test_the_findings_go_in_the_layers_order(self):
        p = copy.deepcopy(olgular(idare={"system": 5256}))
        p["paftalar"][0]["kaynak"] = {"revision": "8", "sha256": "c" * 64}
        p["paket"]["dosyalar"][1].update(ozetTutar=False)
        c = inceleme.denetle(p)
        sira = [inceleme.KATMANLAR.index(b["katman"]) for b in c["bulgular"]]
        self.assertEqual(sira, sorted(sira))


if __name__ == "__main__":
    unittest.main()
