"""Alan dilimleri'nin hesabı, KentOS'suz: `PYTHONPATH=src python -m unittest discover -s tests`."""

import unittest

from ornek_isleme.logic import alan, dilim_adi, dilim_adlari, sinirlari_oku


class AlanDilimleri(unittest.TestCase):
    def test_bir_kare_ve_deligi(self):
        kare = [[0, 0], [10, 0], [10, 10], [0, 10], [0, 0]]
        delik = [[2, 2], [4, 2], [4, 4], [2, 4], [2, 2]]
        self.assertEqual(alan({"type": "Polygon", "coordinates": [kare]}), 100.0)
        self.assertEqual(alan({"type": "Polygon", "coordinates": [kare, delik]}), 96.0)
        self.assertEqual(alan({"type": "MultiPolygon", "coordinates": [[kare], [delik]]}), 104.0)

    def test_dilimler(self):
        sinirlar = sinirlari_oku("500, 1000, 5000")
        self.assertEqual(dilim_adi(499.99, sinirlar), "500 m² altı")
        self.assertEqual(dilim_adi(500, sinirlar), "500–1000 m²")
        self.assertEqual(dilim_adi(5000, sinirlar), "5000 m² ve üstü")
        self.assertEqual(len(dilim_adlari(sinirlar)), 4)

    def test_bozuk_sinirlar_nedenini_soyler(self):
        for metin in ["", "a, b", "1000, 500", "0, 10"]:
            with self.assertRaises(ValueError):
                sinirlari_oku(metin)


if __name__ == "__main__":
    unittest.main()
