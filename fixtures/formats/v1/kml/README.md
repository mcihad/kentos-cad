# KML ve KMZ örnekleri (ADR 0222)

Bu klasördeki dosyaları `scripts/fixtures/kml_cases.py` yazar ve `--check` ile denetler (KentOS kodu olmadan: Python'un expat'i ve
zipfile'ı, dönüşümler PROJ'la).

- `gezi.kml`, `alanlar.kmz`, `parsel.kml`: okuma örnekleri (klasörler, biçemler, StyleMap, veriler, zaman, izler, karışık MultiGeometry,
  bozuk ve atlanan öğeler; KMZ; örnek çizimin 101/7 parseli). `kml-yok.kmz`, `bozuk.kml`, `kok.kml`, `derin.kml`: okunmayan dosyalar.
- `cases.json`: her örneğin okunuşu (`reads`, float float) ve okumaların projenin sistemine taşınması (`moves`, 10⁻⁶ m içinde).
- `write/<ad>.case.json`: yazıcının girdileri (sistem, datum seçimleri, `KmlWriteInput`); `write/<ad>.kml` ve `.kmz` yazıcının
  çıktısıdır, `KENTOS_WRITE_KML=1 cargo test -p kentos-formats --test all kml` yeniden yazar; başvuru onları okuyup girdisine ve PROJ'a
  göre denetler.

GPX ve WGS 84 GeoJSON yazıcılarının girdi ve çıktıları `../wgs84/`'tedir (`scripts/fixtures/wgs84_write_cases.py`,
`KENTOS_WRITE_WGS84=1 cargo test -p kentos-formats --test all wgs84`).
