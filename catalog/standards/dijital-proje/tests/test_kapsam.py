"""Dijital teslim kapsamının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/kapsam.json, beklenenler
yönetmelik metninden elle), girdilerin okunması ve keşfin sınırı.

    python3 -I catalog/standards/dijital-proje/tests/test_kapsam.py
"""

import json
import sys
import time
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import kapsam  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "kapsam.json"


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        durumlar = json.loads(DURUMLAR.read_text(encoding="utf-8"))
        self.assertGreaterEqual(len(durumlar["cases"]), 20)
        for durum in durumlar["cases"]:
            sonuc = kapsam.karar(durum["permit"], durum["today"])
            satirlar = {s["id"]: s for s in sonuc["rows"]}
            for kimlik, beklenen in durum["expect"].items():
                for alan, deger in beklenen.items():
                    with self.subTest(durum=durum["id"], satir=kimlik, alan=alan):
                        self.assertEqual(satirlar[kimlik][alan], deger, satirlar[kimlik]["reason"])

    def test_her_satir_maddesini_ve_nedenini_soyler(self):
        durumlar = json.loads(DURUMLAR.read_text(encoding="utf-8"))
        for durum in durumlar["cases"]:
            for satir in kapsam.karar(durum["permit"], durum["today"])["rows"]:
                with self.subTest(durum=durum["id"], satir=satir["id"]):
                    self.assertTrue(satir["articles"], "madde yok")
                    self.assertTrue(satir["reason"].strip())
                    self.assertEqual(len(satir["articles"]), len(set(satir["articles"])))
                    if satir["state"] == "confirm":
                        self.assertTrue(satir["missing"] or "K28" in satir["reason"], satir["reason"])
                    if satir["state"] == "not_yet":
                        self.assertIsNotNone(satir["from"])


class Girdiler(unittest.TestCase):
    def test_tarih_sayi_ve_secim_okunur(self):
        self.assertEqual(kapsam._gun_degeri("1.9.2029").isoformat(), "2029-09-01")
        self.assertEqual(kapsam._gun_degeri("2029-09-01").isoformat(), "2029-09-01")
        self.assertIsNone(kapsam._gun_degeri("31.02.2029"))
        self.assertEqual(kapsam._nufus("2.345.678"), 2345678)
        self.assertEqual(kapsam._nufus("2 345 678"), 2345678)
        self.assertIsNone(kapsam._nufus("2,3 milyon"))
        self.assertEqual(kapsam._alan("12.500,75"), 12500.75)
        self.assertEqual(kapsam._alan("5000,5"), 5000.5)
        self.assertIsNone(kapsam._alan("5.000"))
        secenek = ["konut", "ticaret+konut", "diğer"]
        self.assertEqual(kapsam._secim("Ticaret + Konut", secenek), "ticaret+konut")
        self.assertEqual(kapsam._secim("DİĞER", secenek), "diğer")
        self.assertEqual(kapsam._secim("26. madde", ["yok", "4", "26", "27"]), "26")
        self.assertIsNone(kapsam._secim("e", ["hayır", "evet", "entegre tesis"]))

    def test_okunamayan_girdi_sorun_olarak_doner(self):
        sonuc = kapsam.karar({"particulars": {"il_nufusu": "iki milyon"}}, "2026-11-15")
        self.assertEqual([s["input"] for s in sonuc["problems"]], ["il_nufusu"])
        self.assertIn("rakamla", sonuc["problems"][0]["message"])

    def test_bos_kunye_hizla_teyit_ister(self):
        bas = time.perf_counter()
        sonuc = kapsam.karar({}, "2026-11-15")
        self.assertLess(time.perf_counter() - bas, 2.0)
        self.assertEqual({s["state"] for s in sonuc["rows"]}, {"confirm"})

    def test_ayni_girdi_ayni_sonuc(self):
        permit = {"particulars": {"plan_alani": "evet", "yapi_turu": "yeni bina", "kullanim": "konut"}}
        self.assertEqual(json.dumps(kapsam.karar(permit, "2026-11-15")), json.dumps(kapsam.karar(permit, "2026-11-15")))

    def test_paketteki_veri_katalogunkidir(self):
        katalog = DEPO / "crates" / "shared" / "digital-project" / "data" / "kapsam-2026.json"
        if katalog.exists():
            self.assertEqual(json.loads(katalog.read_text(encoding="utf-8")), kapsam.veri())


if __name__ == "__main__":
    unittest.main()
