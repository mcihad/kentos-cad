# 1071 — Üç noktalı yayın katsayısı ortadaki açıdan: harita koordinatında kararlı okuma

Tarih: 2026-11-20. gis-usage-report.md GIS-005 (GIS-043 aynı okuyucudan geçer). Durum: kabul edildi.

İlgili: [ADR 0502](0502-geopackage.md) (GeoPackage alışverişi).

## Sorun

KentOS'un yazdığı eğrili park tamponu (TM33, `CURVEPOLYGON`) GeoPackage'tan geri alındığında görüntü yüzlerce
kilometreye taşıyordu. Köşeler aynıydı; birkaç mikrometrelik, neredeyse doğrusal bir `CIRCULARSTRING`'in yay katsayısı
(bulge) +2,6 × 10⁻⁷ yerine −7,2 × 10¹² okunuyordu. Okuyucu yayı, üç noktadan geçen çemberin merkezini mutlak harita
koordinatlarıyla (x ≈ 4,9 × 10⁵, y ≈ 4,4 × 10⁶) bulup açı farkından hesaplıyordu: neredeyse doğrusal üç noktada merkez
sonsuza kaçar, koordinatların karesindeki yuvarlama süpürme açısını rastgele yapar. FlatGeobuf, GML ve WKT'li CSV
okuyucuları aynı dönüşümden (`geopackage::table_of_rows`) geçer.

## Dayanak

- OGC GeoPackage eğri geometri uzantısı ([belgesi](https://www.geopackage.org/spec/#extension_geometry_types)) ve
  ISO 19125 / SQL-MM `CIRCULARSTRING`: bir yay başlangıç, ara ve bitiş noktasıyla verilir; merkez yazılmaz.
- GDAL 3.13 (`ogrinfo`, OGR Python, 20 Kasım 2026): aynı halkanın zarfı 243,88 × 163,76 m. GDAL'ın kendi eğri alanı
  38 956,275 m²; bu halka için 60 basamaklı bağımsız hesaptan 0,244 m² sapar (doğrusallaştırılmış kopyası tutar),
  bu yüzden başvuru olarak alınmadı.

## Karar

1. Yay katsayısı merkezsiz, ortadaki noktanın açısından hesaplanır: u = a − m, v = b − m için θ/4 = (π − ∠amb)/2,
   dolayısıyla `bulge = (v × u) / (|u||v| − u·v)`. Yalnız noktaların farkları girer; neredeyse doğrusal üç nokta 0'a
   yakın katsayı verir; 180°'yi aşan yay süpürmesini ve yönünü korur. İki nokta aynıysa ya da başlangıç ile bitiş
   buluşuyorsa (tam çember) tek katsayı söyleyemez, kenar düz kalır (eskisi gibi).
   Kod: `crates/native/application/src/gis/geopackage.rs::arc_bulge`.
2. Daha önce yanlış katsayıyla kaydedilmiş `.kcad` sessizce onarılmaz (CLAUDE.md §23.3).

## Sınama

- `scripts/fixtures/gpkg_arc_area.py`: raporun halkasının alanı ve çevresi, KentOS kodu olmadan mpmath ile 60 basamak
  (alan 38 956,519 339 831 m², çevre 790,231 084 908 664 m).
- `crates/native/application/src/gis/geopackage_arc_tests.rs`: raporun halkası (alan 1e-5 m², çevre 1e-6 m içinde;
  köşeler GDAL zarfının 1 cm içinde), üç döndürülmüş kopya, 270° gerçek yay iki yönde, yarım ve çeyrek çember,
  doğrusal ve kapalı durumlar.
- `fixtures/geopackage/v1/egri-tampon.gpkg`: raporun paketinden GDAL ile alınan tek tablo (sentetik veri);
  `gpkg_tests::a_curve_polygons_near_straight_arc_stays_in_the_park` TM33 CBS projesine alır, Tümünü göster parkı
  çerçeveler (görünüm < 1 km). Resim `.run/gui-evidence/gis-005/`.
