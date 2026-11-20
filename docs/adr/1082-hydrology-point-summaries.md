# 1082 — Noktalı hidroloji araçlarının özeti: giren, yazılan ve atlanan noktalar

Tarih: 2026-11-20. gis-usage-report.md GIS-017. Durum: kabul edildi.

İlgili: [ADR 0235](0235-hydrology.md) (hidroloji araçları).

## Sorun

Üç ağaçtan ikisini kapsayan DEM'de Döküm noktası ve Noktadan havza yeşil sonuçla “2 döküm noktası yazıldı” ya da
“2 havza yazıldı” diyordu; üçüncü noktanın raster dışında kaldığı yalnız komut geçmişinde yazıyordu.

## Karar

Döküm noktası ve Noktadan havza'nın özeti (`hydrology::points_said`):
- hepsi alındıysa yalnız sonuç: “5 döküm noktası yazıldı.”;
- bir kısmı atlandıysa: “6 noktadan 5 döküm noktası yazıldı; 1 nokta (4. nokta) rasterin dışında ya da değersiz
  hücrede kaldı.” (en çok beş nokta sırasıyla; noktalar girdideki sırasıyla, sonuçların Nokta/Havza özniteliği gibi);
- hepsi atlandıysa: “Noktaların hiçbiri rasterin üstünde ya da değerli hücrede değil: 3 noktanın hepsi atlandı;
  döküm noktası yazılmadı.”
Komut geçmişindeki uyarı satırı ayrıntı olarak kalır.

## Sınama

- `fixtures/processing/v1/hydrology.json` (`scripts/fixtures/hydrology_processing_cases.py`): dört nokta durumunun
  özeti yeni biçimle; işlem durumları geçer.
