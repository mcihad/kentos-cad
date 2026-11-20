# 1134 — Bulutu konumla tablosu kayıtlı konumu, bu adımı ve toplamı ayırır

Tarih: 2026-11-20. gis-usage-report.md GIS-063. Durum: kabul edildi.

İlgili: ADR 5100 (Nesnede tut, bulutun konumu nesnede).

## Sorun

Öteleme 3 4 0 m ile bir kez Konumla'ya basınca durum satırı “Şimdi … 5 m”, aynı ekrandaki tablo “Ortanın kayması 10 m”
gösterdi: tablo, aynı girişle bir kez daha basılırsa varılacak toplamı yazıyordu, başlığı ise şimdiki konum gibi
okunuyordu. Konumu kaldır'dan sonra da tablo 5 m göstermeyi sürdürdü.

## Karar

Tablonun her dosya satırı dört değeri ayrı sütunlarda yazar: Kayıtlı kayma (bulutun ortasının dosyanın kendi yerinden
şimdiki kaydı), Bu adım (Konumla'ya basılırsa bu girişin kaydırdığı), Uygulanırsa kayma ve Uygulanırsa dönme (basılırsa
varılacak toplam). Girişler korunur: ardışık dönüşümler bilerek yapılabilir, her basışta “Bu adım” aynı, “Kayıtlı”
artar. Konumu kaldır kayıtlıyı sıfırlar.

## Kanıt

`apps/desktop/src/tests/cloud_place_window_tests.rs` `the_table_tells_the_kept_place_from_the_next_step`: basmadan önce
0 / 5 / 5; bir basıştan sonra 5 / 5 / 10; Konumu kaldır'dan sonra kayıtlı 0. Euler çeyrek dönüş testi yeni sütunlarla
geçer.
