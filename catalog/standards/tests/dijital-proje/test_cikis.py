"""Acil çıkış beyanı ve kaçış yolları kuralının sınamaları (docs/adr/8124): elle yazılmış olgularla.

    python3 -I catalog/standards/tests/dijital-proje/test_cikis.py
"""

import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "dijital-proje"
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import cikis, mimari  # noqa: E402


def kapi(uid, route=None, declared=None, name=None):
    x = {"uid": uid, "kind": "door", "level": "zemin"}
    if name:
        x["name"] = name
    if route:
        x["route"] = route
    if declared is not None:
        x["declared"] = declared
    return x


def merdiven(uid, route=None, declared=None):
    x = {"uid": uid, "kind": "stair", "level": "zemin"}
    if route:
        x["route"] = route
    if declared is not None:
        x["declared"] = declared
    return x


class Cikis(unittest.TestCase):
    def test_olgu_yoksa_bulgu_yok(self):
        self.assertEqual(cikis.denetle({}), [])
        self.assertEqual(cikis.denetle({"cad": {"exits": []}}), [])

    def test_beyansizlara_yollarin_onerisi_tek_bulgu_tek_duzeltme(self):
        b = cikis.denetle({"cad": {"exits": [
            kapi("d1", "outside", name="K1"), kapi("d2"), kapi("d3", "intoStair", name="K3"), merdiven("s1", "stairRoom"),
            kapi("d4", "outside", declared=True)]}})
        self.assertEqual(len(b), 1)
        o = b[0]
        self.assertEqual((o["code"], o["severity"]), ("digital_exit_suggestion", "info"))
        self.assertIn("kaçış yolları 2 kapıyı ve 1 merdiveni çıkış sayıyor; 1 kapı çıkış değil", o["message"])
        self.assertIn("Kapı K1 (dışarıya açılan kapı, binanın son çıkışı)", o["message"])
        self.assertIn("beyanı olmayan 4 nesne", o["message"])
        self.assertEqual(o["fix"]["command"], "cad.arch.performance")
        self.assertEqual(o["fix"]["input"], {"operation": {"kind": "exits", "yes": ["d1", "d3", "s1"], "no": ["d2"]}})
        self.assertEqual(o["fix"]["label"], "Önerileri yaz (3 evet, 1 hayır)")
        self.assertEqual(o["uids"], ["d1", "d3", "s1", "d2"])

    def test_beyan_yolla_uyusmuyorsa_uyari_onerinin_yonunde(self):
        b = cikis.denetle({"cad": {"exits": [
            kapi("d1", "outside", declared=False, name="K1"), kapi("d2", declared=True), kapi("d3", declared=False),
            merdiven("s1", "openStair", declared=True)]}})
        self.assertEqual([x["code"] for x in b], ["digital_exit_mismatch"] * 2)
        hayir, evet = b
        self.assertIn("Kapı K1: acil çıkış değil diye beyan edilmiş ama kaçış yolları onu çıkış sayıyor", hayir["message"])
        self.assertEqual(hayir["fix"]["input"]["operation"], {"kind": "exits", "yes": ["d1"], "no": []})
        self.assertEqual(hayir["severity"], "warning")
        self.assertIn("Kapı: acil çıkış diye beyan edilmiş ama kaçış yolları onu çıkış saymıyor", evet["message"])
        self.assertEqual(evet["fix"]["input"]["operation"], {"kind": "exits", "yes": [], "no": ["d2"]})
        self.assertIn("EK-6 Tablo 6.25", evet["cite"])

    def test_yolun_hic_cikis_saymadigi_bina(self):
        b = cikis.denetle({"cad": {"exits": [kapi("d1"), merdiven("s1")]}})
        self.assertIn("hiçbir kapıyı ya da merdiveni çıkış sayıyor; 1 kapı ve 1 merdiven çıkış değil", b[0]["message"])

    def test_masaustunun_denetimi_kurali_cagirir(self):
        bulgular = mimari.denetle({"cad": {"exits": [kapi("d1", "outside")]}}, {"files": []})
        self.assertTrue(any(x["code"] == "digital_exit_suggestion" for x in bulgular))


if __name__ == "__main__":
    unittest.main()
