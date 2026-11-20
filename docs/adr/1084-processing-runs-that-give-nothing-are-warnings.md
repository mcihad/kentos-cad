# 1084 — Bir şey vermeyen işlem uyarıdır: M değerlerini çıkar ve boş raster sonuçları

Tarih: 2026-11-20. gis-usage-report.md GIS-019 (GIS-020 aynı yolu kullanır). Durum: kabul edildi.

İlgili: [ADR 0201](0201-geometry-operations.md) (işlem araçları).

## Sorun

M değerlerini çıkar, ölçüsüz beş yolda yeşil onayla “5 rotadan 0 rotanın ölçüleri yazıldı” dedi; beşinin de M
değeri olmadığı yalnız komut geçmişindeydi. Benzer biçimde tamamı boş raster sonuçları yeşil onay alıyordu.

## Karar

1. İşlem sonucu `RunResult::warning` taşır: iş bitti ama işe yarar bir şey vermedi. Masaüstü işlem penceresi böyle
   bir sonucu başarı değil uyarı satırıyla (sarı simge) gösterir (`RunStatus::Ok { warning }`).
2. M değerlerini çıkar: rotaların hiçbirinde ölçü yoksa özet “5 rotanın hiçbirinde M değeri yok; öznitelik
   yazılmadı. Ölçüleri Rota oluştur ya da Rotayı kalibre et verir.” ve uyarıdır; en az biri ölçülüyse eskisi gibi
   başarı.

## Sınama

- `crates/native/processing/tests/run_warnings.rs`: üç çizgi ve iki çoklu çizgi ölçüsüzken uyarı ve yönlendiren özet;
  ölçülü iki rota varken başarı.

## Açık

- Girdi satırında M değeri okunabilen rota sayısını göstermek (araca özgü girdi özeti gerekir).
