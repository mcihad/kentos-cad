"""Vitrin'in saf işlevlerinin sınamaları: ``kentos kek test`` onları eklentinin ortamında Python'un ``unittest``'iyle
çalıştırır (kod ``src``'den, KentOS'un kitaplığı ortamdan). Bir pakete gerek yok."""

import struct
import unittest

from vitrin import logic


class AdaParsel(unittest.TestCase):
    """``ext.tr.ornek.vitrin.ada-parsel``: ifadelerin çağırdığı işlev."""

    def test_yazim(self):
        self.assertEqual(logic.ada_parsel("101", "1"), "101/1")

    def test_sayi_olarak_gelen_ada_ve_bosluk(self):
        self.assertEqual(logic.ada_parsel(101.0, " 4 "), "101/4")

    def test_biri_bossa_deger_yok(self):
        self.assertIsNone(logic.ada_parsel("101", None))
        self.assertIsNone(logic.ada_parsel("", "1"))


class CubukGrafigi(unittest.TestCase):
    """Yapı özeti panelinin grafiği: NumPy dizisinden PNG, çizim kitaplığı olmadan."""

    def test_png_ve_boyutu(self):
        png = logic._cubuklar([3.0, 1.0, 2.0])
        self.assertTrue(png.startswith(b"\x89PNG\r\n\x1a\n"))
        self.assertEqual(struct.unpack(">II", png[16:24]), (480, 140))

    def test_degersiz_grafik_de_cizilir(self):
        self.assertTrue(logic._cubuklar([]).startswith(b"\x89PNG\r\n\x1a\n"))


if __name__ == "__main__":
    unittest.main()
