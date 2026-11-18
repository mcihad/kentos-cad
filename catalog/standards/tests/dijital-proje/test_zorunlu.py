"""Modelin zorunlu alanları kuralının sınamaları (docs/adr/8068): elle yazılmış olgularla.

    python3 -I catalog/standards/tests/dijital-proje/test_zorunlu.py
"""

import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "dijital-proje"
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import mimari, zorunlu  # noqa: E402


def bosluk(path, tr, missing, of, sinif="IfcWall", sinif_tr="Duvar", tablo="6.66", sayfa=107):
    return {"class": sinif, "classTr": sinif_tr, "path": path, "tr": tr, "table": tablo, "page": sayfa,
            "missing": missing, "of": of}


class Zorunlu(unittest.TestCase):
    def test_bosluk_yoksa_bulgu_yok(self):
        self.assertEqual(zorunlu.denetle({"cad": {}}), [])
        self.assertEqual(zorunlu.denetle({}), [])

    def test_duvarin_yangin_sinifi_eksik_nesneleriyle(self):
        b = zorunlu.denetle({"cad": {"modelGaps": [
            bosluk("Pset_WallCommon.FireRating", "Yangın Dayanım Sınıfı", ["w1", "w2"], 16)]}})
        self.assertEqual(len(b), 1)
        self.assertEqual(b[0]["severity"], "error")
        self.assertEqual(b[0]["uids"], ["w1", "w2"])
        self.assertIn("2/16 duvar nesnesinde Yangın Dayanım Sınıfı (Pset_WallCommon.FireRating) yok", b[0]["message"])
        self.assertIn("EK-6 Tablo 6.66 · PDF s. 107 · EK-9 Tablo 9.1 satır 9", b[0]["cite"])
        self.assertIn("Öznitelikler › Yangın dayanımı", b[0]["repair"])
        self.assertTrue(b[0]["code"].startswith("digital_required"))

    def test_binanin_alani_nesnesiz(self):
        b = zorunlu.denetle({"cad": {"modelGaps": [
            bosluk("Pset_BuildingCommon.FireProtectionClass", "Yangın Koruma Sınıfı", [""], 1, "IfcBuilding", "Bina",
                   "6.7", 42)]}})
        self.assertEqual(b[0]["uids"], [])
        self.assertTrue(b[0]["message"].startswith("Bina: Yangın Koruma Sınıfı"))
        self.assertIn("Proje ayarları › Mimari › Yangın güvenliği", b[0]["repair"])

    def test_ek7_tablosu_ek7_der(self):
        b = zorunlu.denetle({"cad": {"modelGaps": [
            bosluk("Pset_ProjectCommon.ProjectType", "Proje Tipi", [""], 1, "IfcProject", "Proje", "7.1", 216)]}})
        self.assertIn("EK-7 Tablo 7.1", b[0]["message"])

    def test_acil_cikis_ve_malzeme_onarimi(self):
        b = zorunlu.denetle({"cad": {"modelGaps": [
            bosluk("Pset_DoorCommon.FireExit", "Acil Çıkış", ["d1"], 5, "IfcDoor", "Kapı", "6.25", 64),
            bosluk("IfcMaterial.Name", "Malzeme", ["s1"], 1, "IfcStair", "Merdiven", "6.61", 99)]}})
        self.assertIn("Acil çıkış", b[0]["repair"])
        self.assertIn("Malzemeler", b[1]["repair"])

    def test_mimari_denetim_kurali_icerir(self):
        b = mimari.denetle({"cad": {"modelGaps": [bosluk("Pset_WallCommon.FireRating", "Yangın Dayanım Sınıfı", ["w"], 1)]}})
        self.assertIn("model_required", [x["rule"] for x in b])


if __name__ == "__main__":
    unittest.main()
