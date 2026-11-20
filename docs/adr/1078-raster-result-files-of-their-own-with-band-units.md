# 1078 — Raster sonucu kendi dosyasına yazılır, bant birimi dosyada yazılı

Tarih: 2026-11-20. gis-usage-report.md GIS-012, GIS-016. Durum: kabul edildi.

İlgili: [ADR 0231](0231-raster-analysis-and-surface.md) (raster çözümleme), [ADR 0515](0515-cloud-optimized-geotiff.md).

## Sorun

Eğim aracı çıktı dosyası verilmeden önce Derece, sonra Yüzde ile çalıştırıldı. İkisi de `Ankara-DEM-egim.tif`'e
yazdı: projedeki iki raster nesnesi aynı dosyaya bakıyordu ve ilk sonucun derece değerleri (orta hücre 12,60438)
uyarısız yüzdeyle (22,36068) değişti (GIS-012). İki GeoTIFF'in bant birimi boştu: yalnız dosyası olan kişi
12,60'ın derece mi yüzde mi olduğunu öğrenemiyordu (GIS-016).

## Dayanak

- GDAL raster bandı ([belgesi](https://gdal.org/en/stable/api/gdalrasterband_cpp.html)): `GetUnitType` ve
  `GetDescription`; GeoTIFF'te GDAL bunları `GDAL_METADATA` (etiket 42112) XML'inde `UNITTYPE` ve `DESCRIPTION`
  öğeleri olarak tutar (GDAL'ın kendi yazdığı dosyadan okundu, 20 Kasım 2026).

## Karar

1. Çıktı dosyası verilmeyince sonuç adı `kaynak<ek>.tif`; o ad diskte varsa ya da bu süreçte başka bir çalıştırma
   ayırdıysa `…-2`, `…-3` alınır (`kentos_processing::files::free_output`). Ad çalıştırmaya ayrılır: aynı anda başlayan
   iki iş aynı adı alamaz, sonraki iş bir katmanın gösterdiği önceki sonucun üzerine yazmaz. Kullanıcının açıkça
   verdiği dosya eskisi gibi kullanılır.
2. Yüzde eğimin eki `-egim-yuzde`; derece `-egim` kalır.
3. Eğim ve bakı sonuçları bandın birimini ve açıklamasını GeoTIFF'in GDAL üstverisine yazar: `degree` “Eğim
   (derece)”, `percent` “Eğim (yüzde)”, bakı `degree` “Bakı (kuzeyden saat yönünde derece; düzlük -1)”
   (`raster::write::Writer::band_unit`, `raster::out::Out::band_unit`).

## Sınama

- `surface_tests::an_automatic_result_file_never_writes_over_an_earlier_one`: modelin kopyası yanında derece, yüzde,
  yine derece: `dem-egim.tif`, `dem-egim-yuzde.tif`, `dem-egim-2.tif`; ilk dosyanın baytları değişmez, üç nesne üç
  dosyaya bakar, geri alınınca da ilk dosya kalır; dosyalar birimlerini taşır.
- GDAL 3.13 bu dosyalarda bant birimini ve açıklamasını okudu (`degree`/“Eğim (derece)”, `percent`/“Eğim (yüzde)”);
  orta hücrede 33,13° ve %65,27 (tan 33,13° × 100).

## Açık

- Sonuç özetine birim yazmak (ortak durumların özetleri değişir).
