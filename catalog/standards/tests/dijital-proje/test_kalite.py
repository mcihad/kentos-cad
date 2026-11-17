"""EK-9'un modelde ölçülen sorularının sınamaları (docs/adr/8067): örtüşen mahaller, elle yazılmış olgularla.

    python3 -I catalog/standards/tests/dijital-proje/test_kalite.py
"""

import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "dijital-proje"
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import kalite, mimari  # noqa: E402


def mahal(uid, numara, ad, kat="K00"):
    return {"uid": uid, "number": numara, "name": ad, "level": "l0", "levelCode": kat, "at": [0.0, 0.0]}


def ortak(a, b, alan):
    return {"a": a, "b": b, "area": alan, "at": [2.5, 1.5]}


MAHALLER = [mahal("r1", "K00_GNL_001", "Salon"), mahal("r2", "K00_GNL_002", "Mutfak"), mahal("r3", None, None)]


class Kalite(unittest.TestCase):
    def test_ortusmeyen_mahaller_temiz(self):
        self.assertEqual(kalite.denetle({"cad": {"rooms": MAHALLER}}), [])
        self.assertEqual(kalite.denetle({}), [])

    def test_ortusen_iki_mahal_hata_ikisi_de_nesnesiyle(self):
        b = kalite.denetle({"cad": {"rooms": MAHALLER, "roomOverlaps": [ortak("r1", "r2", 0.36)]}})
        self.assertEqual([x["rule"] for x in b], ["quality_room_overlap"])
        self.assertEqual(b[0]["severity"], "error")
        self.assertEqual(b[0]["uids"], ["r1", "r2"])
        self.assertEqual(b[0]["at"], [2.5, 1.5])
        self.assertIn("“K00_GNL_001 Salon” ile “K00_GNL_002 Mutfak” (K00 katında) 0.36 m² örtüşüyor", b[0]["message"])
        self.assertIn("EK-9 Tablo 9.1 satır 13 · PDF s. 239", b[0]["cite"])
        self.assertTrue(b[0]["code"].startswith("digital_quality"))

    def test_esigin_altindaki_ortak_alan_yuvarlamadir(self):
        self.assertEqual(kalite.denetle({"cad": {"rooms": MAHALLER, "roomOverlaps": [ortak("r1", "r2", 0.0099)]}}), [])
        self.assertEqual(len(kalite.denetle({"cad": {"rooms": MAHALLER, "roomOverlaps": [ortak("r1", "r2", 0.01)]}})), 1)

    def test_adsiz_mahal_kimligiyle_anilir(self):
        b = kalite.denetle({"cad": {"rooms": MAHALLER, "roomOverlaps": [ortak("r3", "r9", 1.0)]}})
        self.assertIn("“r3” mahali ile “r9” mahali (K00 katında) 1.00 m²", b[0]["message"])

    def test_mimari_denetim_kurali_icerir(self):
        b = mimari.denetle({"cad": {"rooms": MAHALLER, "roomOverlaps": [ortak("r1", "r2", 2.0)]}})
        self.assertIn("quality_room_overlap", [x["rule"] for x in b])


if __name__ == "__main__":
    unittest.main()
