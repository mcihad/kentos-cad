"""``kentos.standards``: standart kural paketinin Python'dan çağrılması (docs/adr/8063)."""

import unittest

from kentos import standards


class Standards(unittest.TestCase):
    def test_kapsam_ayni_kuralin_sonucunu_verir(self):
        sonuc = standards.kapsam(
            {"plan_alani": "evet", "yapi_turu": "yeni bina", "kullanim": "konut", "mustakil": "hayır",
             "il_nufusu": "650000", "belediye_nufusu": "400000"},
            "2026-11-15",
            construction_area=1850.0,
            storeys_above=4,
        )
        satirlar = {s["id"]: s for s in sonuc["rows"]}
        self.assertEqual((satirlar["pdfa"]["state"], satirlar["pdfa"]["from"]), ("not_yet", "2029-03-01"))
        self.assertEqual(satirlar["ifc"]["from"], "2032-09-01")
        self.assertEqual(sonuc["inputs"]["yapi_insaat_alani_kaynagi"], "çizim")

    def test_olmayan_paket_nedenini_soyler(self):
        with self.assertRaises(FileNotFoundError) as e:
            standards.package("yok-boyle-paket")
        self.assertIn("KENTOS_STANDARDS_DIR", str(e.exception))


if __name__ == "__main__":
    unittest.main()
